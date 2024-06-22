use crate::models::EmbeddingsRequest;
use actix_web::{web, Responder};
use onceuponai_core::llm::e5::E5Model;

pub async fn embeddings(
    embeddings_request: web::Json<EmbeddingsRequest>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let embeddings_data = E5Model::embeddings(embeddings_request.input.clone())?;
    Ok(web::Json(embeddings_data))
}
