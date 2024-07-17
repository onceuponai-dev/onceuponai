extern crate onceuponai_actors_abstractions;
use crate::actors::{ActorInvokeRequest, ActorInvokeResponse};
use actix_telepathy::RemoteAddr;
use anyhow::Result;
use log::debug;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors_abstractions::{
    ActorActions, ActorError, ActorInvokeError, ActorInvokeInput, ActorInvokeOutput,
    ActorInvokeResult,
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
    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["embed".to_string()])
    }

    fn kind(&self) -> String {
        todo!()
    }

    fn start(&self) -> Result<()> {
        E5Model::lazy(self.model_repo.clone(), self.device.clone())?;
        debug!("MODEL STARTED");
        Ok(())
    }

    fn invoke(&self, uuid: Uuid, request: &ActorInvokeInput) -> Result<ActorInvokeOutput> {
        let input = request.data.get("input");

        if input.is_none() {
            return Ok(ActorInvokeOutput::Failure(ActorInvokeError {
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

        Ok(ActorInvokeOutput::Success(result))
    }
}
