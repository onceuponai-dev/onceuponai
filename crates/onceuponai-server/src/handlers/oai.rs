use super::actors::{base_invoke, Mappers};
use crate::models::InvokeRequest;
use crate::serve::AppState;
use actix_web::web;
use actix_web::Responder;
use anyhow::Result;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors::abstractions::openai::ChatCompletionRequest;
use onceuponai_actors::abstractions::ActorInvokeData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatCompletionsRequest {
    pub model: String,
    pub messages: Vec<EntityValue>,
    pub stream: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EmbeddingsRequest {
    pub model: String,
    pub input: Vec<EntityValue>,
    pub encoding_format: Option<bool>,
}

pub async fn v1_chat_completions(
    chat_completions_request: web::Json<ChatCompletionRequest>,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let model: Vec<&str> = chat_completions_request.model.split('/').collect();
    let kind: String = model[0].to_string();
    let name: String = model[1].to_string();

    let invoke_request = InvokeRequest {
        config: HashMap::new(),
        stream: chat_completions_request.stream,
        data: ActorInvokeData::ChatCompletion(chat_completions_request.0),
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

pub async fn v1_embeddings(
    embeddings_request: web::Json<EmbeddingsRequest>,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let model: Vec<&str> = embeddings_request.model.split('/').collect();
    let kind: String = model[0].to_string();
    let name: String = model[1].to_string();

    let invoke_request = InvokeRequest {
        config: HashMap::new(),
        data: ActorInvokeData::Entity(HashMap::from([(
            "input".to_string(),
            embeddings_request.input.clone(),
        )])),
        stream: Some(false),
    };
    base_invoke(
        kind,
        name,
        app_state,
        invoke_request,
        Mappers::OaiEmbeddings,
    )
    .await
}
