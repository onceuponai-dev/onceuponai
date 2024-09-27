use super::parse_device;
use actix_telepathy::RemoteAddr;
use anyhow::Result;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::mistral::{Config, Model};
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

pub const MISTRAL_REPO_ID: &str = "mistralai/Mistral-7B-Instruct-v0.2";

static MISTRAL_INSTANCE: OnceCell<Arc<Mutex<MistralModel>>> = OnceCell::new();

#[derive(Deserialize, Debug, Clone)]
pub struct MistralSpec {
    pub base_repo_id: Option<String>,
    pub tokenizer_repo: Option<String>,
    pub device: Option<String>,
    pub seed: Option<u64>,
    pub repeat_last_n: Option<usize>,
    pub repeat_penalty: Option<f32>,
    pub temp: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<usize>,
    pub hf_token: Option<String>,
    pub sample_len: Option<usize>,
}

impl ActorActions for MistralSpec {
    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["chat".to_string()])
    }

    fn kind(&self) -> String {
        "mistral".to_string()
    }

    fn init(&self) -> Result<()> {
        MistralModel::init(self.clone())
    }

    fn start(&self) -> Result<()> {
        MistralModel::lazy(self.clone())?;
        Ok(())
    }

    fn invoke(&self, uuid: Uuid, request: &ActorInvokeRequest) -> Result<ActorInvokeResponse> {
        let input = request.data.get("message");

        if input.is_none() {
            return Ok(ActorInvokeResponse::Failure(ActorInvokeError {
                uuid,
                task_id: request.task_id,
                error: ActorError::BadRequest(
                    "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
                ),
            }));
        }

        let input: Vec<String> = input
            .expect("MESSAGE")
            .iter()
            .map(|x| match x {
                EntityValue::MESSAGE { role: _, content } => content.clone(),
                _ => todo!(),
            })
            .collect();

        let mut model = MistralModel::lazy(self.clone())?.lock().map_anyhow_err()?;

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

        Ok(ActorInvokeResponse::Success(result))
    }

    fn invoke_stream(
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

        let input = input[0].clone();

        let mut model = MistralModel::lazy(self.clone())?.lock().map_anyhow_err()?;
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

pub struct MistralModel {
    pub model: Model,
    pub device: Device,
    pub tokenizer: Tokenizer,
    pub logits_processor: LogitsProcessor,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
    pub eos_token: u32,
    pub sample_len: usize,
}

impl MistralModel {
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

    pub fn lazy<'a>(spec: MistralSpec) -> Result<&'a Arc<Mutex<MistralModel>>> {
        if MISTRAL_INSTANCE.get().is_none() {
            let model = MistralModel::load(spec)?;

            let _ = MISTRAL_INSTANCE.set(Arc::new(Mutex::new(model))).is_ok();
        };

        Ok(MISTRAL_INSTANCE.get().expect("MISTRAL_INSTANCE"))
    }

    pub fn init(spec: MistralSpec) -> Result<()> {
        let base_repo_id = &spec.base_repo_id.expect("base_repo_id");
        let hf_token = Some(
            spec.hf_token
                .unwrap_or(std::env::var("HF_TOKEN").expect("HF_TOKEN")),
        );

        let _paths = hf_hub_get_multiple(
            base_repo_id,
            "model.safetensors.index.json",
            hf_token.clone(),
            None,
        )?;

        let tokenizer_repo = spec.tokenizer_repo.unwrap_or(base_repo_id.clone());
        let _tokenizer = hf_hub_get(&tokenizer_repo, "tokenizer.json", hf_token.clone(), None)?;
        Ok(())
    }

    pub fn load(spec: MistralSpec) -> Result<MistralModel> {
        let seed = spec.seed.unwrap_or(299792458);
        let repeat_last_n = spec.repeat_last_n.unwrap_or(64);
        let repeat_penalty = spec.repeat_penalty.unwrap_or(1.1);
        let hf_token = Some(
            spec.hf_token
                .unwrap_or(std::env::var("HF_TOKEN").expect("HF_TOKEN")),
        );

        let base_repo_id = spec.base_repo_id.expect("base_repo_id");
        let paths = hf_hub_get_multiple(
            &base_repo_id,
            "model.safetensors.index.json",
            hf_token.clone(),
            None,
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
        let model = Model::new(&candle_config, vb)?;

        let logits_processor = {
            let temperature = spec.temp.unwrap_or(0.);
            let sampling = if temperature <= 0. {
                Sampling::ArgMax
            } else {
                match (spec.top_k, spec.top_p) {
                    (None, None) => Sampling::All { temperature },
                    (Some(k), None) => Sampling::TopK { k, temperature },
                    (None, Some(p)) => Sampling::TopP { p, temperature },
                    (Some(k), Some(p)) => Sampling::TopKThenTopP { k, p, temperature },
                }
            };
            LogitsProcessor::from_sampling(seed, sampling)
        };

        let eos_token = match tokenizer.get_vocab(true).get("</s>").copied() {
            Some(token) => token,
            None => {
                return Err(anyhow::anyhow!("EOS token not found in vocabulary")).map_io_err()?
            }
        };

        let sample_len = spec.sample_len.unwrap_or(1000);

        Ok(MistralModel {
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
