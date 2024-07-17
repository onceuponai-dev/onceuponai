extern crate onceuponai_actors_abstractions;
use crate::actors::{ActorInvokeRequest, ActorInvokeResponse, WorkerActor};
use actix_telepathy::RemoteActor;
use anyhow::Result;
use log::debug;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors_abstractions::{
    ActorActions, ActorError, ActorInvokeError, ActorInvokeResult, ActorObject,
};
use onceuponai_candle::llm::e5::E5Model;
use onceuponai_core::common::ResultExt;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
pub struct E5Spec {
    pub model_repo: Option<String>,
    pub device: Option<String>,
}

impl ActorActions for E5Spec {
    fn actor_id(&self) -> &str {
        WorkerActor::ACTOR_ID
    }

    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["embed".to_string()])
    }

    fn kind(&self) -> String {
        todo!()
    }

    fn start(&self) -> Result<()> {
        todo!()
    }

    fn invoke(
        &self,
        uuid: Uuid,
        request: onceuponai_actors_abstractions::ActorInvokeInput,
    ) -> Result<onceuponai_actors_abstractions::ActorInvokeOutput> {
        todo!()
    }

    fn invoke_stream<F>(
        &self,
        uuid: Uuid,
        request: &onceuponai_actors_abstractions::ActorInvokeInput,
        callback: F,
    ) -> Result<()>
    where
        F: FnMut(onceuponai_actors_abstractions::ActorInvokeOutput),
    {
        todo!()
    }
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
