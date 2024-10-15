use either::Either;
use mistralrs::{ImageGenerationResponseFormat, Tool, ToolChoice};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Deref};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageInnerContent(
    #[serde(with = "either::serde_untagged")] Either<String, HashMap<String, String>>,
);

impl Deref for MessageInnerContent {
    type Target = Either<String, HashMap<String, String>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageContent(
    #[serde(with = "either::serde_untagged")]
    pub  Either<String, Vec<HashMap<String, MessageInnerContent>>>,
);

impl Deref for MessageContent {
    type Target = Either<String, Vec<HashMap<String, MessageInnerContent>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub content: MessageContent,
    pub role: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StopTokens {
    Multi(Vec<String>),
    Single(String),
}

fn default_false() -> bool {
    false
}

fn default_1usize() -> usize {
    1
}

fn default_720usize() -> usize {
    720
}

fn default_1280usize() -> usize {
    1280
}

fn default_model() -> String {
    "default".to_string()
}

fn default_response_format() -> ImageGenerationResponseFormat {
    ImageGenerationResponseFormat::Url
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Grammar {
    #[serde(rename = "regex")]
    Regex(String),
    #[serde(rename = "yacc")]
    Yacc(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionRequest {
    #[serde(with = "either::serde_untagged")]
    pub messages: Either<Vec<Message>, String>,
    #[serde(default = "default_model")]
    pub model: String,
    pub logit_bias: Option<HashMap<u32, f32>>,
    #[serde(default = "default_false")]
    pub logprobs: bool,
    pub top_logprobs: Option<usize>,
    pub max_tokens: Option<usize>,
    #[serde(rename = "n")]
    #[serde(default = "default_1usize")]
    pub n_choices: usize,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    #[serde(rename = "stop")]
    pub stop_seqs: Option<StopTokens>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub stream: Option<bool>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<ToolChoice>,

    // mistral.rs additional
    pub top_k: Option<usize>,
    pub grammar: Option<Grammar>,
    pub adapters: Option<Vec<String>>,
    pub min_p: Option<f64>,
    pub dry_multiplier: Option<f32>,
    pub dry_base: Option<f32>,
    pub dry_allowed_length: Option<usize>,
    pub dry_sequence_breakers: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompletionRequest {
    #[serde(default = "default_model")]
    pub model: String,
    pub prompt: String,
    #[serde(default = "default_1usize")]
    pub best_of: usize,
    #[serde(rename = "echo")]
    #[serde(default = "default_false")]
    pub echo_prompt: bool,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub logit_bias: Option<HashMap<u32, f32>>,
    pub logprobs: Option<usize>,
    pub max_tokens: Option<usize>,
    #[serde(rename = "n")]
    #[serde(default = "default_1usize")]
    pub n_choices: usize,
    #[serde(rename = "stop")]
    pub stop_seqs: Option<StopTokens>,
    pub stream: Option<bool>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub suffix: Option<String>,
    #[serde(rename = "user")]
    pub _user: Option<String>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<ToolChoice>,

    // mistral.rs additional
    pub top_k: Option<usize>,
    pub grammar: Option<Grammar>,
    pub adapters: Option<Vec<String>>,
    pub min_p: Option<f64>,
    pub dry_multiplier: Option<f32>,
    pub dry_base: Option<f32>,
    pub dry_allowed_length: Option<usize>,
    pub dry_sequence_breakers: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageGenerationRequest {
    #[serde(default = "default_model")]
    pub model: String,
    pub prompt: String,
    #[serde(rename = "n")]
    #[serde(default = "default_1usize")]
    pub n_choices: usize,
    #[serde(default = "default_response_format")]
    pub response_format: ImageGenerationResponseFormat,
    #[serde(default = "default_720usize")]
    pub height: usize,
    #[serde(default = "default_1280usize")]
    pub width: usize,
}
