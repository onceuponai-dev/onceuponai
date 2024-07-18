use super::parse_device;
use anyhow::Result;
use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::quantized_llama as model;
use model::ModelWeights;
use once_cell::sync::OnceCell;
use onceuponai_core::common::{hf_hub_get, hf_hub_get_path, OptionToResult, ResultExt};
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;

static QUANTIZED_INSTANCE: OnceCell<Arc<Mutex<QuantizedInstance>>> = OnceCell::new();

pub struct QuantizedModel {
    pub model: ModelWeights,
    pub tokenizer: Tokenizer,
    pub device: Device,
}

pub struct QuantizedInstance {
    pub instance: QuantizedModel,
    pub eos_token: u32,
    pub seed: u64,
    pub repeat_last_n: usize,
    pub repeat_penalty: f32,
    pub temp: f64,
    pub top_p: Option<f64>,
    pub sample_len: usize,
}

impl QuantizedModel {
    #[allow(clippy::too_many_arguments)]
    pub fn invoke(
        &mut self,
        prompt: &str,
        sample_len: usize,
        eos_token: u32,
        seed: Option<u64>,
        repeat_last_n: Option<usize>,
        repeat_penalty: Option<f32>,
        temp: Option<f64>,
        top_p: Option<f64>,
    ) -> Result<String> {
        let repeat_penalty: f32 = repeat_penalty.unwrap_or(1.1);
        let repeat_last_n: usize = repeat_last_n.unwrap_or(64);

        let prep = self.prepare(prompt, seed, temp, top_p)?;
        let prompt_tokens_len = prep.0;
        let mut all_tokens = prep.1;
        let mut logits_processor = prep.2;

        let mut previous_text = String::new();
        for index in 0..sample_len {
            if let Some(current_text) = self.loop_process(
                prompt_tokens_len,
                index,
                repeat_penalty,
                repeat_last_n,
                &mut all_tokens,
                &mut logits_processor,
                eos_token,
            )? {
                previous_text = current_text;
            } else {
                break;
            }
        }

        Ok(previous_text)
    }

    pub fn prepare(
        &mut self,
        prompt: &str,
        seed: Option<u64>,
        temperature: Option<f64>,
        top_p: Option<f64>,
    ) -> Result<(usize, Vec<u32>, LogitsProcessor)> {
        let prompt_tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(anyhow::Error::msg)?
            .get_ids()
            .to_vec();

        let seed: u64 = seed.unwrap_or(299792458);
        let temperature = temperature.unwrap_or(0.8);

        let mut all_tokens = vec![];
        let mut logits_processor = LogitsProcessor::new(seed, Some(temperature), top_p);

        let input = Tensor::new(prompt_tokens.as_slice(), &self.device)?.unsqueeze(0)?;
        let logits = self.model.forward(&input, 0)?;
        let logits = logits.squeeze(0)?;
        let next_token = logits_processor.sample(&logits)?;

        all_tokens.push(next_token);
        let prompt_tokens_len = prompt_tokens.len();

        Ok((prompt_tokens_len, all_tokens, logits_processor))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn loop_process(
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

        if next_token == eos_token {
            return Ok(None);
        };

        let current_text = self
            .tokenizer
            .decode(all_tokens, true)
            .map_err(anyhow::Error::msg)?;

        Ok(Some(current_text))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn lazy<'a>(
        model_repo: Option<String>,
        model_file: Option<String>,
        model_revision: Option<String>,
        tokenizer_repo: Option<String>,
        device: Option<String>,
        seed: Option<u64>,
        repeat_last_n: Option<usize>,
        repeat_penalty: Option<f32>,
        temp: Option<f64>,
        top_p: Option<f64>,
        sample_len: Option<usize>,
    ) -> Result<&'a Arc<Mutex<QuantizedInstance>>> {
        if QUANTIZED_INSTANCE.get().is_none() {
            let model = QuantizedModel::load(
                &model_repo.expect("model_repo"),
                &model_file.expect("model_file"),
                model_revision,
                tokenizer_repo,
                device,
            )?;

            let eos_token = "</s>";
            let vocab = model.tokenizer.get_vocab(true).clone();
            let eos_token = *vocab.get(eos_token).ok_or_err("EOS_TOKEN")?;

            let seed: u64 = seed.unwrap_or(299792458);
            let temp = temp.unwrap_or(0.8);
            let repeat_penalty: f32 = repeat_penalty.unwrap_or(1.1);
            let repeat_last_n: usize = repeat_last_n.unwrap_or(64);
            let sample_len = sample_len.unwrap_or(1000);

            let quantized_instance = QuantizedInstance {
                instance: model,
                eos_token,
                seed,
                repeat_last_n,
                repeat_penalty,
                temp,
                top_p,
                sample_len,
            };

            let _ = QUANTIZED_INSTANCE
                .set(Arc::new(Mutex::new(quantized_instance)))
                .is_ok();
        };

        Ok(QUANTIZED_INSTANCE.get().expect("QUANTIZED_INSTANCE"))
    }

    pub fn load(
        model_repo: &str,
        model_file: &str,
        model_revision: Option<String>,
        tokenizer_repo: Option<String>,
        device_type: Option<String>,
    ) -> Result<QuantizedModel> {
        let base_repo_id = (model_repo, model_file);

        let model_path = if model_file.starts_with("file://") {
            std::path::PathBuf::from(model_file.replace("file://", ""))
        } else {
            hf_hub_get_path(base_repo_id.0, base_repo_id.1, None, model_revision)?
        };

        let tokenizer_repo = tokenizer_repo.unwrap_or(model_repo.to_string());

        let tokenizer = if tokenizer_repo.starts_with("file://") {
            std::fs::read(tokenizer_repo.replace("file://", ""))?
        } else {
            hf_hub_get(&tokenizer_repo, "tokenizer.json", None, None)?
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

#[tokio::test]
async fn test_bielik() -> Result<()> {
    let mut bielik = QuantizedModel::load(
        "speakleash/Bielik-7B-Instruct-v0.1-GGUF",
        "bielik-7b-instruct-v0.1.Q4_K_S.gguf",
        None,
        Some("speakleash/Bielik-7B-Instruct-v0.1".to_string()),
        Some("cuda".to_string()),
    )?;

    let eos_token = "</s>";
    let vocab = bielik.tokenizer.get_vocab(true).clone();
    let eos_token = *vocab.get(eos_token).ok_or_err("EOS_TOKEN")?;

    let resp = bielik.invoke(
        "Jak ugotować żurek ?",
        500,
        eos_token,
        None,
        None,
        None,
        None,
        None,
    )?;

    println!("RESPONSE: {resp}");
    Ok(())
}

#[tokio::test]
async fn test_phi3() -> Result<()> {
    let mut phi3 = QuantizedModel::load(
        "microsoft/Phi-3-mini-4k-instruct-gguf",
        "Phi-3-mini-4k-instruct-q4.gguf",
        Some("5eef2ce24766d31909c0b269fe90c817a8f263fb".to_string()),
        Some("microsoft/Phi-3-mini-4k-instruct".to_string()),
        Some("cuda".to_string()),
    )?;

    let resp = phi3.invoke(
        "Write loop in python",
        500,
        200,
        None,
        None,
        None,
        None,
        None,
    )?;

    println!("RESPONSE: {resp}");
    Ok(())
}
