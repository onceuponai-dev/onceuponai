use crate::actors::{ActorInvokeRequest, ActorInvokeResponse, WorkerActor};
use actix_telepathy::{RemoteActor, RemoteAddr};
use anyhow::Result;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors_abstractions::{
    ActorActions, ActorError, ActorInvokeError, ActorInvokeFinish, ActorInvokeInput,
    ActorInvokeOutput, ActorInvokeResult,
};
use onceuponai_candle::llm::quantized::QuantizedModel;
use onceuponai_core::common::ResultExt;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
pub struct QuantizedSpec {
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

impl ActorActions for QuantizedSpec {
    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["chat".to_string()])
    }

    fn kind(&self) -> String {
        "quantized".to_string()
    }

    fn start(&self) -> Result<()> {
        QuantizedModel::lazy(
            self.model_repo.clone(),
            self.model_file.clone(),
            self.model_revision.clone(),
            self.tokenizer_repo.clone(),
            self.device.clone(),
            self.seed,
            self.repeat_last_n,
            self.repeat_penalty,
            self.temp,
            self.top_p,
            self.sample_len,
        )?;

        Ok(())
    }

    fn invoke(
        &self,
        uuid: Uuid,
        request: &ActorInvokeInput,
    ) -> Result<onceuponai_actors_abstractions::ActorInvokeOutput> {
        let input = request.data.get("message");

        if input.is_none() {
            return Ok(ActorInvokeOutput::Failure(ActorInvokeError {
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
            data: HashMap::from([(String::from("content"), results)]),
        };

        Ok(ActorInvokeOutput::Success(result))
    }

    fn invoke_stream(
        &self,
        uuid: Uuid,
        request: &onceuponai_actors_abstractions::ActorInvokeInput,
        source: RemoteAddr,
    ) -> Result<()> {
        let input = request.data.get("message");
        if input.is_none() {
            source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
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
                    data: HashMap::from([(
                        String::from("content"),
                        vec![EntityValue::STRING(text)],
                    )]),
                };

                let response = ActorInvokeResponse::Success(result);
                source.do_send(response);
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
        source.do_send(response);

        Ok(())
    }
}
