use crate::actors::{ActorInvokeRequest, ActorInvokeResponse, WorkerActor};
use actix_telepathy::{RemoteActor, RemoteAddr};
use anyhow::Result;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors_abstractions::{
    ActorActions, ActorError, ActorInvokeError, ActorInvokeInput, ActorInvokeOutput,
    ActorInvokeResult,
};
use onceuponai_candle::llm::gemma::GemmaModel;
use onceuponai_core::common::ResultExt;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

pub fn start(spec: GemmaSpec) -> Result<()> {
    GemmaModel::lazy(
        spec.base_repo_id,
        spec.tokenizer_repo,
        spec.device,
        spec.seed,
        spec.repeat_last_n,
        spec.repeat_penalty,
        spec.temp,
        spec.top_p,
        spec.hf_token,
        spec.use_flash_attn,
        spec.sample_len,
    )?;
    Ok(())
}

pub fn invoke(uuid: Uuid, request: ActorInvokeRequest) -> Result<ActorInvokeResponse> {
    let input = request.data.get("prompt");

    if input.is_none() {
        return Ok(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER PROMPT COLUMN WITH Vec<String>".to_string(),
            ),
        }));
    }

    let input: Vec<String> = input
        .expect("PROMPT")
        .iter()
        .map(|x| match x {
            EntityValue::STRING(i) => i.clone(),
            _ => todo!(),
        })
        .collect();

    let mut model = GemmaModel::lazy(
        None, None, None, None, None, None, None, None, None, None, None,
    )?
    .lock()
    .map_anyhow_err()?;
    let sample_len = model.sample_len;
    let eos_token = model.eos_token;

    let results = input
        .iter()
        .map(|prompt| model.instance.invoke(prompt, sample_len, eos_token))
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

/*
pub async fn chat(prompt: &str) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let mut lazy = GemmaModel::lazy(
        None, None, None, None, None, None, None, None, None, None, None,
    )?
    .lock()
    .map_anyhow_err()?;
    lazy.instance.model.clear_kv_cache();

    let mut tokens = lazy
        .instance
        .tokenizer
        .encode(prompt, true)
        .map_err(anyhow::Error::msg)?
        .get_ids()
        .to_vec();

    let sample_len = lazy.sample_len;
    let eos_token = lazy.eos_token;

    let stream_tasks = stream! {
        for index in 0..sample_len {
            if let Some(text) = lazy.instance.loop_process(tokens.len(), index, &mut tokens, eos_token)? {
                let byte = bytes::Bytes::from(text);
                tokio::task::yield_now().await;
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

*/

#[derive(Deserialize, Debug, Clone)]
pub struct GemmaSpec {
    pub base_repo_id: Option<String>,
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

impl ActorActions for GemmaSpec {
    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["chat".to_string()])
    }

    fn kind(&self) -> String {
        "gemma".to_string()
    }

    fn start(&self) -> Result<()> {
        todo!()
    }

    fn invoke(
        &self,
        uuid: Uuid,
        request: &ActorInvokeInput,
    ) -> Result<onceuponai_actors_abstractions::ActorInvokeOutput> {
        todo!()
    }

    fn invoke_stream(
        &self,
        uuid: Uuid,
        request: &onceuponai_actors_abstractions::ActorInvokeInput,
        source: RemoteAddr,
    ) -> Result<()> {
        todo!()
    }
}
