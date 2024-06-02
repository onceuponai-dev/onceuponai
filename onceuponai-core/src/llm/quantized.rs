use super::parse_device;
use crate::common::{hf_hub_get, hf_hub_get_path, OptionToResult, ResultExt};
use anyhow::Result;
use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::quantized_llama as model;
use model::ModelWeights;
use tokenizers::Tokenizer;

pub struct QuantizedModel {
    pub model: ModelWeights,
    pub tokenizer: Tokenizer,
    pub device: Device,
}

impl QuantizedModel {
    #[allow(clippy::too_many_arguments)]
    pub async fn invoke(
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

        let prep = self.prepare(prompt, seed, temp, top_p).await?;
        let prompt_tokens_len = prep.0;
        let mut all_tokens = prep.1;
        let mut logits_processor = prep.2;

        let mut previous_text = String::new();
        for index in 0..sample_len {
            if let Some(current_text) = self
                .loop_process(
                    prompt_tokens_len,
                    index,
                    repeat_penalty,
                    repeat_last_n,
                    &mut all_tokens,
                    &mut logits_processor,
                    eos_token,
                )
                .await?
            {
                previous_text = current_text;
            } else {
                break;
            }
        }

        Ok(previous_text)
    }

    pub async fn prepare(
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

    pub fn load(
        model_repo: &str,
        model_file: &str,
        tokenizer_repo: &str,
        device_type: Option<String>,
    ) -> Result<QuantizedModel> {
        let base_repo_id = (model_repo, model_file);

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
