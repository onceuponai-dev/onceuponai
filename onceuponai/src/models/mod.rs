use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct PromptRequest {
    pub prompt: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EmbeddingsRequest {
    pub input: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct AuthCallback {
    pub code: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct TokenLogin {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PATRequest {
    pub expiration_days: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PATResponse {
    pub personal_access_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PATClaims {
    pub sub: String,
    pub exp: usize,
}
