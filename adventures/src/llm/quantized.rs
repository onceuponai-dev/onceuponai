use super::parse_device;
use crate::common::{hf_hub_get, hf_hub_get_path, OptionToResult, ResultExt};
use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use async_stream::stream;
use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::quantized_llama as model;
use model::ModelWeights;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokenizers::Tokenizer;
use tokio::sync::Mutex;

static QUANTIZED_MODEL: OnceCell<Arc<Mutex<QuantizedModel>>> = OnceCell::new();

pub async fn chat(
    prompt: &str,
    sample_len: usize,
    eos_token: u32,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let mut model = QUANTIZED_MODEL
        .get()
        .ok_or_err("QUANTIZED_MODEL")?
        .lock()
        .await;

    let prompt_tokens = model
        .tokenizer
        .encode(prompt, true)
        .map_err(anyhow::Error::msg)?
        .get_ids()
        .to_vec();

    let seed: u64 = 299792458;
    let temperature: Option<f64> = Some(0.8);
    let top_p: Option<f64> = None;
    let repeat_penalty: f32 = 1.1;
    let repeat_last_n: usize = 64;

    let mut all_tokens = vec![];
    let mut logits_processor = LogitsProcessor::new(seed, temperature, top_p);

    let input = Tensor::new(prompt_tokens.as_slice(), &model.device)?.unsqueeze(0)?;
    let logits = model.model.forward(&input, 0)?;
    let logits = logits.squeeze(0)?;
    let next_token = logits_processor.sample(&logits)?;

    all_tokens.push(next_token);
    let prompt_tokens_len = prompt_tokens.len();

    let stream_tasks = stream! {
        let mut previous_text = String::new();
        for index in 0..sample_len {

            if let Some(current_text) = model.loop_process(prompt_tokens_len, index, repeat_penalty, repeat_last_n, &mut all_tokens, &mut logits_processor, eos_token).await? {
                let text = current_text.split_at(previous_text.len()).1.to_string();
                previous_text = current_text;
                let byte = bytes::Bytes::from(text);
                yield Ok::<bytes::Bytes, Box<dyn std::error::Error>>(byte);
            } else {
                break;
            }
        }

    };

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(Box::pin(stream_tasks)))
}

pub struct QuantizedModel {
    pub model: ModelWeights,
    pub tokenizer: Tokenizer,
    pub device: Device,
}

impl QuantizedModel {
    #[allow(clippy::too_many_arguments)]
    pub async fn loop_process(
        &mut self,
        prompt_tokens_len: usize,
        index: usize,
        repeat_penalty: f32,
        repeat_last_n: usize,
        all_tokens: &mut Vec<u32>,
        logits_processor: &mut LogitsProcessor,
        eos_token: u32,
    ) -> Result<Option<String>> {
        let next_token = *all_tokens.last().expect("Wrong ALL_TOKENS");
        let input = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
        let logits = self.model.forward(&input, prompt_tokens_len + index)?;
        let logits = logits.squeeze(0)?;
        let logits = if repeat_penalty == 1. {
            logits
        } else {
            let start_at = all_tokens.len().saturating_sub(repeat_last_n);
            candle_transformers::utils::apply_repeat_penalty(
                &logits,
                repeat_penalty,
                &all_tokens[start_at..],
            )?
        };
        let next_token = logits_processor.sample(&logits)?;
        all_tokens.push(next_token);

        tokio::task::yield_now().await;

        if next_token == eos_token {
            return Ok(None);
        };

        let current_text = self
            .tokenizer
            .decode(all_tokens, true)
            .map_err(anyhow::Error::msg)?;

        Ok(Some(current_text))
    }

    pub fn init(
        model_repo: &str,
        model_file: &str,
        tokenizer_repo: &str,
        device_type: &str,
    ) -> Result<u32> {
        let model = QuantizedModel::load(model_repo, model_file, tokenizer_repo, device_type)?;

        let eos_token = "</s>";
        let vocab = model.tokenizer.get_vocab(true).clone();
        let eos_token = *vocab.get(eos_token).ok_or_err("EOS_TOKEN")?;

        let _ = QUANTIZED_MODEL.set(Arc::new(Mutex::new(model)));
        Ok(eos_token)
    }

    pub fn load(
        model_repo: &str,
        model_file: &str,
        tokenizer_repo: &str,
        device_type: &str,
    ) -> Result<QuantizedModel> {
        //let base_repo_id = ("TheBloke/CodeLlama-7B-GGUF", "codellama-7b.Q4_0.gguf");
        let base_repo_id = (model_repo, model_file);
        //let base_repo_id = ("MaziyarPanahi/gemma-2b-it-GGUF", "gemma-2b-it.Q4_K_M.gguf");
        //let tokenizer_repo = "hf-internal-testing/llama-tokenizer";
        //let tokenizer_repo = "google/gemma-2b-it";

        let model_path = if model_file.starts_with("file://") {
            std::path::PathBuf::from(model_file.replace("file://", ""))
        } else {
            hf_hub_get_path(base_repo_id.0, base_repo_id.1, None, None)?
        };

        let tokenizer = if tokenizer_repo.starts_with("file://") {
            std::fs::read(tokenizer_repo.replace("file://", ""))?
        } else {
            hf_hub_get(tokenizer_repo, "tokenizer.json", None, None)?
        };

        let device = parse_device(device_type)?;
        let tokenizer = Tokenizer::from_bytes(&tokenizer).map_anyhow_err()?;

        let mut file = std::fs::File::open(model_path)?;
        let model_content = gguf_file::Content::read(&mut file)?;
        let model = ModelWeights::from_gguf(model_content, &mut file, &device)?;

        Ok(QuantizedModel {
            model,
            tokenizer,
            device,
        })
    }
}
