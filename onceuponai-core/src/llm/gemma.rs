use super::parse_device;
use crate::common::{hf_hub_get, hf_hub_get_multiple, ResultExt};
use anyhow::Result;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::gemma::{Config, Model};
use tokenizers::Tokenizer;

pub const GEMMA_2B_REPO_ID: &str = "google/gemma-2b-it";

pub struct GemmaModel {
    pub model: Model,
    pub device: Device,
    pub tokenizer: Tokenizer,
    pub logits_processor: LogitsProcessor,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
}

impl GemmaModel {
    pub async fn invoke(
        &mut self,
        prompt: &str,
        sample_len: usize,
        eos_token: u32,
    ) -> Result<String> {
        self.model.clear_kv_cache();

        let mut tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(anyhow::Error::msg)?
            .get_ids()
            .to_vec();

        let mut full_text = String::new();
        for index in 0..sample_len {
            if let Some(text) = self
                .loop_process(tokens.len(), index, &mut tokens, eos_token)
                .await?
            {
                full_text.push_str(&text);
            } else {
                break;
            }
        }

        Ok(full_text)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn loop_process(
        &mut self,
        tokens_len: usize,
        index: usize,
        tokens: &mut Vec<u32>,
        eos_token: u32,
    ) -> Result<Option<String>> {
        let context_size = if index > 0 { 1 } else { tokens_len };
        let start_pos = tokens_len.saturating_sub(context_size);
        let ctxt = &tokens[start_pos..];
        let input = Tensor::new(ctxt, &self.device)?.unsqueeze(0)?;
        let logits = self.model.forward(&input, start_pos)?;
        let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?;
        let logits = if self.repeat_penalty == 1. {
            logits
        } else {
            let start_at = tokens.len().saturating_sub(self.repeat_last_n);
            candle_transformers::utils::apply_repeat_penalty(
                &logits,
                self.repeat_penalty,
                &tokens[start_at..],
            )?
        };

        let next_token = self.logits_processor.sample(&logits)?;
        tokens.push(next_token);
        if next_token == eos_token {
            return Ok(None);
        }

        tokio::task::yield_now().await;
        let text = &self
            .tokenizer
            .decode(&[next_token], true)
            .map_err(anyhow::Error::msg)?;
        println!("{text}");
        Ok(Some(text.to_string()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn load(
        base_repo_id: String,
        tokenizer_repo: Option<String>,
        device: Option<String>,
        seed: Option<u64>,
        repeat_last_n: Option<usize>,
        repeat_penalty: Option<f32>,
        temp: Option<f64>,
        top_p: Option<f64>,
        hf_token: Option<String>,
    ) -> Result<GemmaModel> {
        let seed = seed.unwrap_or(299792458);
        let repeat_last_n = repeat_last_n.unwrap_or(64);
        let repeat_penalty = repeat_penalty.unwrap_or(1.1);
        let hf_token = Some(hf_token.unwrap_or(std::env::var("HF_TOKEN").expect("HF_TOKEN")));

        let paths = hf_hub_get_multiple(
            &base_repo_id,
            "model.safetensors.index.json",
            None,
            hf_token.clone(),
        )?;

        let device = parse_device(device)?;
        let dtype = if device.is_cuda() {
            DType::BF16
        } else {
            DType::F32
        };

        let tokenizer_repo = tokenizer_repo.unwrap_or(base_repo_id.clone());

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&paths, dtype, &device)? };
        let tokenizer = hf_hub_get(&tokenizer_repo, "tokenizer.json", None, hf_token.clone())?;
        let tokenizer = Tokenizer::from_bytes(&tokenizer).map_anyhow_err()?;
        let candle_config = hf_hub_get(&base_repo_id, "config.json", None, hf_token)?;
        let candle_config: Config = serde_json::from_slice(&candle_config)?;
        let model = Model::new(&candle_config, vb)?;

        let logits_processor = LogitsProcessor::new(seed, temp, top_p);
        Ok(GemmaModel {
            model,
            tokenizer,
            logits_processor,
            repeat_penalty,
            repeat_last_n,
            device,
        })
    }
}
