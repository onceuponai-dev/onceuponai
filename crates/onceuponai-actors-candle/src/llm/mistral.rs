use actix_telepathy::RemoteAddr;
use anyhow::Result;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors::abstractions::{
    ActorActions, ActorError, ActorInvokeError, ActorInvokeFinish, ActorInvokeRequest,
    ActorInvokeResponse, ActorInvokeResult,
};
use onceuponai_candle::llm::mistral::MistralModel;
use onceuponai_core::common::ResultExt;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
pub struct MistralSpec {
    pub base_repo_id: Option<String>,
    pub tokenizer_repo: Option<String>,
    pub device: Option<String>,
    pub seed: Option<u64>,
    pub repeat_last_n: Option<usize>,
    pub repeat_penalty: Option<f32>,
    pub temp: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<usize>,
    pub hf_token: Option<String>,
    pub sample_len: Option<usize>,
}

impl ActorActions for MistralSpec {
    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["chat".to_string()])
    }

    fn kind(&self) -> String {
        "mistral".to_string()
    }

    fn init(&self) -> Result<()> {
        MistralModel::init(
            self.base_repo_id.clone(),
            self.tokenizer_repo.clone(),
            self.hf_token.clone(),
        )
    }

    fn start(&self) -> Result<()> {
        MistralModel::lazy(
            self.base_repo_id.clone(),
            self.tokenizer_repo.clone(),
            self.device.clone(),
            self.seed,
            self.repeat_last_n,
            self.repeat_penalty,
            self.temp,
            self.top_p,
            self.top_k,
            self.hf_token.clone(),
            self.sample_len,
        )?;
        Ok(())
    }

    fn invoke(&self, uuid: Uuid, request: &ActorInvokeRequest) -> Result<ActorInvokeResponse> {
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
                EntityValue::MESSAGE { role: _, content } => content.clone(),
                _ => todo!(),
            })
            .collect();

        let mut model = MistralModel::lazy(
            None, None, None, None, None, None, None, None, None, None, None,
        )?
        .lock()
        .map_anyhow_err()?;
        let sample_len = model.sample_len;
        let eos_token = model.eos_token;

        let results = input
            .iter()
            .map(|prompt| model.instance.invoke(prompt, sample_len, eos_token))
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

        Ok(ActorInvokeResponse::Success(result))
    }

    fn invoke_stream(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
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
                EntityValue::MESSAGE { role: _, content } => content.clone(),
                _ => todo!(),
            })
            .collect();

        let input = input[0].clone();

        let mut model = MistralModel::lazy(
            None, None, None, None, None, None, None, None, None, None, None,
        )?
        .lock()
        .map_anyhow_err()?;
        let sample_len: usize = model.sample_len;
        let eos_token = model.eos_token;
        model.instance.model.clear_kv_cache();

        let mut tokens = model
            .instance
            .tokenizer
            .encode(input, true)
            .map_err(anyhow::Error::msg)?
            .get_ids()
            .to_vec();

        let tokens_len = tokens.len();

        for index in 0..sample_len {
            if let Some(_text) =
                model
                    .instance
                    .loop_process(tokens.len(), index, &mut tokens, eos_token)?
            {
                let text = model
                    .instance
                    .tokenizer
                    .decode(&tokens[tokens_len + index..], true)
                    .map_err(anyhow::Error::msg)?;

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
