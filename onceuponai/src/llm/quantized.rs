use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use async_stream::stream;
use onceuponai_core::{
    common::ResultExt, common_models::EntityValue, llm::quantized::QuantizedModel,
};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::actors::{
    ActorError, ActorInvokeError, ActorInvokeFinish, ActorInvokeRequest, ActorInvokeResponse,
    ActorInvokeResult,
};

pub fn start(spec: QuantizedConfig) -> Result<()> {
    QuantizedModel::lazy(
        spec.model_repo,
        spec.model_file,
        spec.model_revision,
        spec.tokenizer_repo,
        spec.device,
        spec.seed,
        spec.repeat_last_n,
        spec.repeat_penalty,
        spec.temp,
        spec.top_p,
        spec.sample_len,
    )?;

    Ok(())
}

pub fn invoke(uuid: Uuid, request: ActorInvokeRequest) -> Result<ActorInvokeResponse> {
    let input = request.data.get("message");

    if input.is_none() {
        return Ok(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));
    }

    let input: Vec<String> = input
        .expect("MESSAGE")
        .iter()
        .map(|x| match x {
            EntityValue::MESSAGE { role, content } => content.clone(),
            _ => todo!(),
        })
        .collect();

    let mut model = QuantizedModel::lazy(
        None, None, None, None, None, None, None, None, None, None, None,
    )?
    .lock()
    .map_anyhow_err()?;
    let eos_token = model.eos_token;
    let seed = model.seed;
    let repeat_last_n = model.repeat_last_n;
    let repeat_penalty = model.repeat_penalty;
    let temp = model.temp;
    let top_p = model.top_p;
    let sample_len = model.sample_len;

    let results = input
        .iter()
        .map(|prompt| {
            model.instance.invoke(
                prompt,
                sample_len,
                eos_token,
                Some(seed),
                Some(repeat_last_n),
                Some(repeat_penalty),
                Some(temp),
                top_p,
            )
        })
        .collect::<Result<Vec<String>, _>>()?;

    let results = results
        .iter()
        .map(|r| EntityValue::STRING(r.clone()))
        .collect::<Vec<EntityValue>>();

    let result = ActorInvokeResult {
        uuid,
        task_id: request.task_id,
        stream: request.stream,
        metadata: HashMap::new(),
        data: HashMap::from([(String::from("results"), results)]),
    };

    Ok(ActorInvokeResponse::Success(result))
}

pub fn invoke_stream<F>(uuid: Uuid, request: ActorInvokeRequest, mut callback: F) -> Result<()>
where
    F: FnMut(ActorInvokeResponse),
{
    let input = request.data.get("message");
    if input.is_none() {
        callback(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));

        return Ok(());
    }

    let input: Vec<String> = input
        .expect("MESSAGE")
        .iter()
        .map(|x| match x {
            EntityValue::MESSAGE { role, content } => content.clone(),
            _ => todo!(),
        })
        .collect();

    let input = input[0].clone();

    let mut model = QuantizedModel::lazy(
        None, None, None, None, None, None, None, None, None, None, None,
    )?
    .lock()
    .map_anyhow_err()?;
    let seed = model.seed;
    let repeat_last_n = model.repeat_last_n;
    let repeat_penalty = model.repeat_penalty;
    let temp = model.temp;
    let top_p = model.top_p;

    let prep = model
        .instance
        .prepare(&input, Some(seed), Some(temp), top_p)?;
    let prompt_tokens_len = prep.0;
    let mut all_tokens = prep.1;
    let mut logits_processor = prep.2;

    let sample_len = model.sample_len;
    let eos_token = model.eos_token;

    let mut previous_text = String::new();
    for index in 0..sample_len {
        if let Some(current_text) = model.instance.loop_process(
            prompt_tokens_len,
            index,
            repeat_penalty,
            repeat_last_n,
            &mut all_tokens,
            &mut logits_processor,
            eos_token,
        )? {
            let text = current_text.split_at(previous_text.len()).1.to_string();
            previous_text = current_text;

            let result = ActorInvokeResult {
                uuid,
                task_id: request.task_id,
                stream: request.stream,
                metadata: HashMap::new(),
                data: HashMap::from([(String::from("results"), vec![EntityValue::STRING(text)])]),
            };

            let response = ActorInvokeResponse::Success(result);

            callback(response);
        } else {
            break;
        }
    }

    let result = ActorInvokeFinish {
        uuid,
        task_id: request.task_id,
        stream: request.stream,
    };

    let response = ActorInvokeResponse::Finish(result);
    callback(response);

    Ok(())
}

pub async fn chat(prompt: &str) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let mut lazy = QuantizedModel::lazy(
        None, None, None, None, None, None, None, None, None, None, None,
    )?
    .lock()?;

    let repeat_penalty: f32 = 1.1;
    let repeat_last_n: usize = 64;
    let seed = lazy.seed;
    let temp = lazy.temp;
    let top_p = lazy.top_p;

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
                tokio::task::yield_now().await;
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

#[derive(Deserialize, Debug, Clone)]
pub struct QuantizedConfig {
    pub model_repo: Option<String>,
    pub model_file: Option<String>,
    pub model_revision: Option<String>,
    pub tokenizer_repo: Option<String>,
    pub device: Option<String>,
    pub seed: Option<u64>,
    pub repeat_last_n: Option<usize>,
    pub repeat_penalty: Option<f32>,
    pub temp: Option<f64>,
    pub top_p: Option<f64>,
    pub sample_len: Option<usize>,
}
