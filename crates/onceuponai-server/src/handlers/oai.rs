use super::actors::{base_invoke, Mappers};
use crate::models::InvokeRequest;
use crate::serve::AppState;
use actix_web::web;
use actix_web::Responder;
use anyhow::Result;
use onceuponai_abstractions::EntityValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatCompletionsRequest {
    pub model: String,
    pub messages: Vec<EntityValue>,
    pub stream: Option<bool>,
}

pub async fn v1_chat_completions(
    chat_completions_request: web::Json<ChatCompletionsRequest>,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let model: Vec<&str> = chat_completions_request.model.split('/').collect();
    let kind: String = model[0].to_string();
    let name: String = model[1].to_string();

    let invoke_request = InvokeRequest {
        config: HashMap::new(),
        data: HashMap::from([(
            "message".to_string(),
            chat_completions_request.messages.clone(),
        )]),
        stream: chat_completions_request.stream,
    };
    base_invoke(
        kind,
        name,
        app_state,
        invoke_request,
        Mappers::OaiChatCompletions,
    )
    .await
}
