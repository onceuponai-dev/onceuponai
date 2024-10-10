use crate::parse_device;
use actix_telepathy::RemoteAddr;
use anyhow::Result;
use async_trait::async_trait;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::gemma::{Config, Model};
use once_cell::sync::OnceCell;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors::abstractions::{
    ActorActions, ActorError, ActorInvokeError, ActorInvokeFinish, ActorInvokeRequest,
    ActorInvokeResponse, ActorInvokeResult,
};
use onceuponai_core::common::{hf_hub_get, hf_hub_get_multiple, ResultExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;
use uuid::Uuid;

pub const GEMMA_2B_REPO_ID: &str = "google/gemma-2b-it";

static GEMMA_INSTANCE: OnceCell<Arc<Mutex<GemmaModel>>> = OnceCell::new();

#[derive(Deserialize, Debug, Clone)]
pub struct GemmaSpec {
    pub model_repo: Option<String>,
    pub model_revision: Option<String>,
    pub tokenizer_repo: Option<String>,
    pub device: Option<String>,
    pub seed: Option<u64>,
    pub repeat_last_n: Option<usize>,
    pub repeat_penalty: Option<f32>,
    pub temp: Option<f64>,
    pub top_p: Option<f64>,
    pub hf_token: Option<String>,
    pub use_flash_attn: Option<bool>,
    pub sample_len: Option<usize>,
}

#[async_trait]
impl ActorActions for GemmaSpec {
    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["chat".to_string()])
    }

    fn kind(&self) -> String {
        "gemma".to_string()
    }

    fn init(&self) -> Result<()> {
        GemmaModel::init(self.clone())
    }

    async fn start(&self) -> Result<()> {
        GemmaModel::lazy(self.clone())?;
        Ok(())
    }

    async fn invoke(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        let input = request.data.get("message");

        if input.is_none() {
            source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
                uuid,
                task_id: request.task_id,
                error: ActorError::BadRequest(
                    "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
                ),
            }));
            return Ok(());
        }

        let input: Vec<String> = input
            .expect("MESSAGE")
            .iter()
            .map(|x| match x {
                EntityValue::MESSAGE { role: _, content } => content.clone(),
                _ => todo!(),
            })
            .collect();

        let mut model = GemmaModel::lazy(self.clone())?.lock().map_anyhow_err()?;

        let results = input
            .iter()
            .map(|prompt| model.invoke(prompt))
            .collect::<Result<Vec<String>, _>>()?;

        let results = results
            .iter()
            .map(|r| EntityValue::STRING(r.clone()))
            .collect::<Vec<EntityValue>>();

        let result = ActorInvokeResult {
            uuid,
            task_id: request.task_id,
            stream: request.stream,
            metadata: HashMap::new(),
            data: HashMap::from([(String::from("content"), results)]),
        };

        source.do_send(ActorInvokeResponse::Success(result));
        Ok(())
    }

    async fn invoke_stream(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        let input = request.data.get("message");

        if input.is_none() {
            source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));

            return Ok(());
        }

        let input = input
            .expect("MESSAGE")
            .iter()
            .map(|x| match x {
                EntityValue::MESSAGE { role, content } => {
                    let turn_type = match role.as_str() {
                        "user" => "user",
                        "model" => "model",
                        _ => "unknown",
                    };
                    format!("<start_of_turn>{}\n{}\n<end_of_turn>", turn_type, content)
                }
                _ => todo!(),
            })
            .collect::<Vec<_>>()
            .join("\n");

        let mut model = GemmaModel::lazy(self.clone())?.lock().map_anyhow_err()?;
        let sample_len: usize = model.sample_len;
        model.model.clear_kv_cache();

        let mut tokens = model
            .tokenizer
            .encode(input, true)
            .map_err(anyhow::Error::msg)?
            .get_ids()
            .to_vec();

        let tokens_len = tokens.len();

        for index in 0..sample_len {
            if let Some(_text) = model.loop_process(tokens.len(), index, &mut tokens)? {
                let text = model
                    .tokenizer
                    .decode(&tokens[tokens_len + index..], true)
                    .map_err(anyhow::Error::msg)?;

                let result = ActorInvokeResult {
                    uuid,
                    task_id: request.task_id,
                    stream: request.stream,
                    metadata: HashMap::new(),
                    data: HashMap::from([(
                        String::from("content"),
                        vec![EntityValue::STRING(text)],
                    )]),
                };

                let response = ActorInvokeResponse::Success(result);
                source.do_send(response);
            } else {
                break;
            }
        }

        let result = ActorInvokeFinish {
            uuid,
            task_id: request.task_id,
            stream: request.stream,
        };

        let response = ActorInvokeResponse::Finish(result);
        source.do_send(response);

        Ok(())
    }
}

pub struct GemmaModel {
    pub model: Model,
    pub device: Device,
    pub tokenizer: Tokenizer,
    pub logits_processor: LogitsProcessor,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
    pub eos_token: u32,
    pub sample_len: usize,
}

impl GemmaModel {
    pub fn invoke(&mut self, prompt: &str) -> Result<String> {
        self.model.clear_kv_cache();

        let mut tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(anyhow::Error::msg)?
            .get_ids()
            .to_vec();
        let tokens_len = tokens.len();

        for index in 0..self.sample_len {
            if let Some(_text) = self.loop_process(tokens.len(), index, &mut tokens)? {
            } else {
                break;
            }
        }

        let text = self
            .tokenizer
            .decode(&tokens[tokens_len..], true)
            .map_err(anyhow::Error::msg)?;

        Ok(text)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn loop_process(
        &mut self,
        tokens_len: usize,
        index: usize,
        tokens: &mut Vec<u32>,
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
        if next_token == self.eos_token {
            return Ok(None);
        }

        Ok(Some("".to_string()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn lazy<'a>(spec: GemmaSpec) -> Result<&'a Arc<Mutex<GemmaModel>>> {
        if GEMMA_INSTANCE.get().is_none() {
            let model = GemmaModel::load(spec)?;

            let _ = GEMMA_INSTANCE.set(Arc::new(Mutex::new(model))).is_ok();
        };

        Ok(GEMMA_INSTANCE.get().expect("GEMMA_INSTANCE"))
    }

    pub fn init(spec: GemmaSpec) -> Result<()> {
        let base_repo_id = &spec.model_repo.expect("base_repo_id");
        let hf_token = Some(
            spec.hf_token
                .unwrap_or(std::env::var("HF_TOKEN").expect("HF_TOKEN")),
        );

        let _paths = hf_hub_get_multiple(
            base_repo_id,
            "model.safetensors.index.json",
            hf_token.clone(),
            spec.model_revision,
        )?;

        let tokenizer_repo = spec.tokenizer_repo.unwrap_or(base_repo_id.clone());
        let _tokenizer = hf_hub_get(&tokenizer_repo, "tokenizer.json", hf_token.clone(), None)?;
        Ok(())
    }

    pub fn load(spec: GemmaSpec) -> Result<GemmaModel> {
        let seed = spec.seed.unwrap_or(299792458);
        let repeat_last_n = spec.repeat_last_n.unwrap_or(64);
        let repeat_penalty = spec.repeat_penalty.unwrap_or(1.1);
        let use_flash_attn = spec.use_flash_attn.unwrap_or(false);
        let hf_token = Some(
            spec.hf_token
                .unwrap_or(std::env::var("HF_TOKEN").expect("HF_TOKEN")),
        );
        let base_repo_id = spec.model_repo.expect("base_repo_id");
        let paths = hf_hub_get_multiple(
            &base_repo_id,
            "model.safetensors.index.json",
            hf_token.clone(),
            spec.model_revision,
        )?;

        let device = parse_device(spec.device)?;
        let dtype = if device.is_cuda() {
            DType::BF16
        } else {
            DType::F32
        };

        let tokenizer_repo = spec.tokenizer_repo.unwrap_or(base_repo_id.clone());

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&paths, dtype, &device)? };
        let tokenizer = hf_hub_get(&tokenizer_repo, "tokenizer.json", hf_token.clone(), None)?;
        let tokenizer = Tokenizer::from_bytes(&tokenizer).map_anyhow_err()?;
        let candle_config = hf_hub_get(&base_repo_id, "config.json", hf_token, None)?;
        let candle_config: Config = serde_json::from_slice(&candle_config)?;
        let model = Model::new(use_flash_attn, &candle_config, vb)?;

        let logits_processor = LogitsProcessor::new(seed, spec.temp, spec.top_p);

        let eos_token = match tokenizer.get_vocab(true).get("<eos>").copied() {
            Some(token) => token,
            None => {
                return Err(anyhow::anyhow!("EOS token not found in vocabulary")).map_io_err()?
            }
        };

        let sample_len = spec.sample_len.unwrap_or(1000);

        Ok(GemmaModel {
            model,
            tokenizer,
            logits_processor,
            repeat_penalty,
            repeat_last_n,
            device,
            eos_token,
            sample_len,
        })
    }
}

/*
#[tokio::test]
async fn test_codegemma() -> Result<()> {
    let mut phi3 = GemmaModel::load(
        "google/codegemma-1.1-7b-it".to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    //let resp = phi3.invoke("Write loop in python").await?;

    //println!("RESPONSE: {resp}");
    Ok(())
}
*/
