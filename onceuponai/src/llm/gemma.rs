use actix_web::{HttpResponse, Responder};
use async_stream::stream;
use onceuponai_core::llm::gemma::GemmaModel;

pub async fn chat(prompt: &str) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let mut lazy = GemmaModel::lazy(
        None, None, None, None, None, None, None, None, None, None, None,
    )?
    .lock()
    .await;
    lazy.instance.model.clear_kv_cache();

    let mut tokens = lazy
        .instance
        .tokenizer
        .encode(prompt, true)
        .map_err(anyhow::Error::msg)?
        .get_ids()
        .to_vec();

    let sample_len = lazy.sample_len;
    let eos_token = lazy.eos_token;

    let stream_tasks = stream! {
        for index in 0..sample_len {
            if let Some(text) = lazy.instance.loop_process(tokens.len(), index, &mut tokens, eos_token)? {
                let byte = bytes::Bytes::from(text);
                tokio::task::yield_now().await;
                yield Ok::<bytes::Bytes, Box<dyn std::error::Error>>(byte);
            }
            else
            {
                break;
            }
        }
    };

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(Box::pin(stream_tasks)))
}
