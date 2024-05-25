use actix_web::{error, http::StatusCode};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct PromptRequest {
    pub prompt: String,
    pub sample_len: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EmbeddingsRequest {
    pub input: Vec<String>,
}

#[derive(Debug, Display, Error)]
#[display(fmt = "request error: {name}")]
pub struct LLMError {
    name: &'static String,
    status_code: u16,
}

impl error::ResponseError for LLMError {
    fn status_code(&self) -> StatusCode {
        let status_code: StatusCode =
            StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        status_code
    }
}
