use crate::models::PromptRequest;
use actix_web::{web, Responder};
use onceuponai_core::llm::rag::{build_prompt, find_context};
use std::error::Error;

pub async fn chat(request: web::Query<PromptRequest>) -> Result<impl Responder, Box<dyn Error>> {
    let context = find_context(request.prompt.to_string()).await.unwrap();
    let prompt = build_prompt(request.prompt.to_string(), context)
        .await
        .unwrap();

    crate::llm::gemma::chat(&prompt).await
}

pub async fn chat_quantized(
    request: web::Query<PromptRequest>,
) -> Result<impl Responder, Box<dyn Error>> {
    let context = find_context(request.prompt.to_string()).await.unwrap();
    let prompt = build_prompt(request.prompt.to_string(), context)
        .await
        .unwrap();

    crate::llm::quantized::chat(&prompt).await
}
