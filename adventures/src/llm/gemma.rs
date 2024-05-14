use super::parse_device;
use crate::common::{hf_hub_get, hf_hub_get_multiple, ResultExt};
use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use async_stream::stream;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::gemma::{Config, Model};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokenizers::Tokenizer;
use tokio::sync::Mutex;

pub const GEMMA_2B_REPO_ID: &str = "google/gemma-2b-it";

static GEMMA_MODEL: OnceCell<Arc<Mutex<GemmaModel>>> = OnceCell::new();

pub async fn chat(
    prompt: &str,
    sample_len: usize,
    eos_token: u32,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let mut model = GEMMA_MODEL.get().unwrap().lock().await;
    model.model.clear_kv_cache();

    let mut tokens = model
        .tokenizer
        .encode(prompt, true)
        .map_err(anyhow::Error::msg)?
        .get_ids()
        .to_vec();

    let stream_tasks = stream! {
        for index in 0..sample_len {
            if let Some(text) = model.loop_process(tokens.len(), index, &mut tokens, eos_token).await? {
                let byte = bytes::Bytes::from(text);
                yield Ok::<bytes::Bytes, Box<dyn std::error::Error>>(byte);
            }
            else
            {
                break;
            }
        }

    };

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(Box::pin(stream_tasks)))
}

pub struct GemmaModel {
    pub model: Model,
    pub device: Device,
    pub tokenizer: Tokenizer,
    pub logits_processor: LogitsProcessor,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
}

impl GemmaModel {
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

    pub fn init(hf_token: &str, device_type: &str) -> Result<u32> {
        let model = GemmaModel::load(
            GEMMA_2B_REPO_ID,
            None,
            299792458,
            Some(0.8),
            None,
            1.1,
            64,
            Some(hf_token.to_string()),
            device_type,
        )
        .unwrap();

        let eos_token = match model.tokenizer.get_vocab(true).get("<eos>").copied() {
            Some(token) => token,
            None => {
                return Err(anyhow::anyhow!("EOS token not found in vocabulary")).map_io_err()?
            }
        };

        let _ = GEMMA_MODEL.set(Arc::new(Mutex::new(model))).is_ok();

        Ok(eos_token)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn load(
        base_repo_id: &str,
        model_endpoint: Option<String>,
        seed: u64,
        temp: Option<f64>,
        top_p: Option<f64>,
        repeat_penalty: f32,
        repeat_last_n: usize,
        hf_token: Option<String>,
        device_type: &str,
    ) -> Result<GemmaModel> {
        let paths = hf_hub_get_multiple(
            base_repo_id,
            "model.safetensors.index.json",
            model_endpoint.clone(),
            hf_token.clone(),
        )?;

        let device = parse_device(device_type)?;
        let dtype = if device.is_cuda() {
            DType::BF16
        } else {
            DType::F32
        };

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&paths, dtype, &device)? };
        let tokenizer = hf_hub_get(
            base_repo_id,
            "tokenizer.json",
            model_endpoint.clone(),
            hf_token.clone(),
        )?;
        let tokenizer = Tokenizer::from_bytes(&tokenizer).map_anyhow_err()?;
        let candle_config = hf_hub_get(
            base_repo_id,
            "config.json",
            model_endpoint.clone(),
            hf_token,
        )?;
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
