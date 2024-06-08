use actix_web::{HttpResponse, Responder};
use async_stream::stream;
use onceuponai_core::llm::quantized::QuantizedModel;

pub async fn chat(prompt: &str) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let mut lazy = QuantizedModel::lazy(
        None, None, None, None, None, None, None, None, None, None, None,
    )?
    .lock()
    .await;

    let repeat_penalty: f32 = 1.1;
    let repeat_last_n: usize = 64;
    let seed = lazy.seed.clone();
    let temp = lazy.temp.clone();
    let top_p = lazy.top_p.clone();

    let prep = lazy
        .instance
        .prepare(prompt, Some(seed), Some(temp), top_p)?;
    let prompt_tokens_len = prep.0;
    let mut all_tokens = prep.1;
    let mut logits_processor = prep.2;

    let sample_len = lazy.sample_len;
    let eos_token = lazy.eos_token;

    let stream_tasks = stream! {
        let mut previous_text = String::new();
        for index in 0..sample_len {

            if let Some(current_text) = lazy.instance.loop_process(prompt_tokens_len, index, repeat_penalty, repeat_last_n, &mut all_tokens, &mut logits_processor, eos_token)? {
                let text = current_text.split_at(previous_text.len()).1.to_string();
                previous_text = current_text;
                let byte = bytes::Bytes::from(text);
                yield Ok::<bytes::Bytes, Box<dyn std::error::Error>>(byte);
            } else {
                break;
            }
        }

    };

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(Box::pin(stream_tasks)))
}
