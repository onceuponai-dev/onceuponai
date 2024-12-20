use crate::parse_device;
use actix_telepathy::RemoteAddr;
use anyhow::Result;
use async_trait::async_trait;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use log::info;
use once_cell::sync::OnceCell;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors::abstractions::{
    ActorActions, ActorError, ActorInvokeData, ActorInvokeError, ActorInvokeRequest,
    ActorInvokeResponse, ActorInvokeResult,
};
use onceuponai_core::common::{hf_hub_get, OptionToResult, ResultExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokenizers::{PaddingParams, Tokenizer};
use uuid::Uuid;

pub const E5_MODEL_REPO: &str = "intfloat/e5-small-v2";

static E5_INSTANCE: OnceCell<Arc<Mutex<E5Model>>> = OnceCell::new();

#[derive(Deserialize, Debug, Clone)]
pub struct E5Spec {
    pub model_repo: Option<String>,
    pub device: Option<String>,
    pub hf_token: Option<String>,
}

#[async_trait]
impl ActorActions for E5Spec {
    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["embed".to_string()])
    }

    fn kind(&self) -> String {
        "e5".to_string()
    }

    async fn init(&self) -> Result<()> {
        E5Model::init(self.clone())
    }

    async fn start(&self) -> Result<()> {
        E5Model::lazy(self.clone())?;
        info!("MODEL STARTED");
        Ok(())
    }

    async fn invoke(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        let input = match request.data.clone() {
            ActorInvokeData::Entity(entity) => entity.get("input").unwrap().clone(),
            _ => {
                source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));

                return Ok(());
            }
        };

        let input: Vec<String> = input
            .iter()
            .map(|x| match x {
                EntityValue::STRING(i) => i.clone(),
                _ => todo!(),
            })
            .collect();

        let embeddings_data: Vec<EntityValue> = E5Model::lazy(self.clone())?
            .lock()
            .map_anyhow_err()?
            .embed(input)?
            .iter()
            .map(|e| EntityValue::FLOAT32ARRAY(e.clone()))
            .collect();

        let result = ActorInvokeResult {
            uuid,
            task_id: request.task_id,
            stream: request.stream,
            metadata: HashMap::new(),
            data: HashMap::from([(String::from("embeddings"), embeddings_data)]),
        };

        source.do_send(ActorInvokeResponse::Success(result));
        Ok(())
    }
}

pub struct E5Model {
    pub spec: E5Spec,
    pub model: BertModel,
    pub tokenizer: Tokenizer,
    pub normalize_embeddings: Option<bool>,
    pub device: Device,
}

impl E5Model {
    pub fn lazy<'a>(spec: E5Spec) -> Result<&'a Arc<Mutex<E5Model>>> {
        if E5_INSTANCE.get().is_none() {
            let e5_model = E5Model::load(spec)?;
            let _ = E5_INSTANCE.set(Arc::new(Mutex::new(e5_model))).is_ok();
        };

        Ok(E5_INSTANCE.get().expect("E5_INSTANCE"))
    }

    pub fn embeddings(input: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let model = E5_INSTANCE
            .get()
            .ok_or_err("E5_MODEL")?
            .lock()
            .map_anyhow_err()?;
        let embeddings_data = model.embed(input)?;
        Ok(embeddings_data)
    }

    pub fn init(spec: E5Spec) -> Result<()> {
        let e5_model_repo = &spec.model_repo.expect("model_repo");
        let _weights = hf_hub_get(
            e5_model_repo,
            "model.safetensors",
            spec.hf_token.clone(),
            None,
        )?;
        let _tokenizer = hf_hub_get(e5_model_repo, "tokenizer.json", spec.hf_token.clone(), None)?;
        let _candle_config = hf_hub_get(e5_model_repo, "config.json", spec.hf_token, None)?;
        Ok(())
    }

    pub fn load(spec: E5Spec) -> Result<E5Model> {
        let spec_clone = spec.clone();
        let model_repo = spec.model_repo.clone().expect("model_repo");
        let weights = hf_hub_get(
            &model_repo,
            "model.safetensors",
            spec.hf_token.clone(),
            None,
        )?;
        let tokenizer = hf_hub_get(&model_repo, "tokenizer.json", spec.hf_token.clone(), None)?;
        let candle_config = hf_hub_get(&model_repo, "config.json", spec.hf_token, None)?;
        let candle_config: BertConfig = serde_json::from_slice(&candle_config)?;

        let device = parse_device(spec.device)?;
        let mut tokenizer = Tokenizer::from_bytes(&tokenizer).map_anyhow_err()?;

        if let Some(pp) = tokenizer.get_padding_mut() {
            pp.strategy = tokenizers::PaddingStrategy::BatchLongest
        } else {
            let pp = PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            };
            tokenizer.with_padding(Some(pp));
        }

        let vb = VarBuilder::from_buffered_safetensors(weights, DType::F32, &device)?;
        let model = BertModel::load(vb, &candle_config)?;
        Ok(E5Model {
            spec: spec_clone,
            model,
            tokenizer,
            normalize_embeddings: Some(true),
            device,
        })
    }

    pub fn embed(&self, input: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let device = &self.device;
        let tokens = self
            .tokenizer
            .encode_batch(input.clone(), true)
            .map_anyhow_err()?;

        let token_ids: Vec<Tensor> = tokens
            .iter()
            .map(|tokens| {
                let tokens = tokens.get_ids().to_vec();
                Tensor::new(tokens.as_slice(), device)
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let attention_mask: Vec<Tensor> = tokens
            .iter()
            .map(|tokens| {
                let tokens = tokens.get_attention_mask().to_vec();
                Tensor::new(tokens.as_slice(), device)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let token_ids = Tensor::stack(&token_ids, 0)?;
        let attention_mask = Tensor::stack(&attention_mask, 0)?;
        let token_type_ids = token_ids.zeros_like()?;

        let embeddings = self
            .model
            .forward(&token_ids, &token_type_ids, Some(&attention_mask))?;
        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
        let embeddings = if let Some(true) = self.normalize_embeddings {
            embeddings.broadcast_div(&embeddings.sqr()?.sum_keepdim(1)?.sqrt()?)?
        } else {
            embeddings
        };
        let embeddings_data: Vec<Vec<f32>> = embeddings.to_vec2()?;
        Ok(embeddings_data)
    }
}
