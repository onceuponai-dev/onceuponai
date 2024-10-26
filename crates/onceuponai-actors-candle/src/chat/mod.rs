pub mod gemma;
pub mod mistral;
pub mod openai_chat;
pub mod quantized;

use anyhow::Result;

pub trait ChatModelActions {
    fn invoke(&mut self, prompt: &str) -> Result<String>;
}
