use anyhow::Result;
use log::debug;
use onceuponai_abstractions::EntityValue;
use onceuponai_candle::llm::e5::E5Model;
use onceuponai_core::common::ResultExt;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::actors::{
    ActorError, ActorInvokeError, ActorInvokeRequest, ActorInvokeResponse, ActorInvokeResult,
};

#[derive(Deserialize, Debug, Clone)]
pub struct E5Spec {
    pub model_repo: Option<String>,
    pub device: Option<String>,
}

pub fn start(spec: E5Spec) -> Result<()> {
    E5Model::lazy(spec.model_repo, spec.device)?;
    debug!("MODEL STARTED");
    Ok(())
}

pub fn invoke(uuid: Uuid, request: ActorInvokeRequest) -> Result<ActorInvokeResponse> {
    let input = request.data.get("input");

    if input.is_none() {
        return Ok(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER INPUT COLUMN WITH Vec<String>".to_string(),
            ),
        }));
    }

    let input: Vec<String> = input
        .expect("INPUT")
        .iter()
        .map(|x| match x {
            EntityValue::STRING(i) => i.clone(),
            _ => todo!(),
        })
        .collect();

    let embeddings_data: Vec<EntityValue> = E5Model::lazy(None, None)?
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

    Ok(ActorInvokeResponse::Success(result))
}
