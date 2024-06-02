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
