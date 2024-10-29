use actix_telepathy::RemoteAddr;
use anyhow::Result;
use async_trait::async_trait;
use either::Either;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors::abstractions::{
    openai::{ChatCompletionRequest, Message},
    ActorActions, ActorError, ActorInvokeData, ActorInvokeError, ActorInvokeFinish,
    ActorInvokeRequest, ActorInvokeResponse, ActorInvokeResult,
};
use onceuponai_core::common::some_or_env;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

static HTTP_CLIENT: Lazy<Client> = Lazy::new(Client::new);

#[derive(Deserialize, Debug, Clone)]
pub struct OpenAIChatSpec {
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub max_tokens: Option<u32>,
    pub seed: Option<u64>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
}

#[async_trait]
impl ActorActions for OpenAIChatSpec {
    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["chat".to_string()])
    }

    fn kind(&self) -> String {
        "openaichat".to_string()
    }

    async fn init(&self) -> Result<()> {
        OpenAIChatModel::init(self.clone())
    }

    async fn start(&self) -> Result<()> {
        OpenAIChatModel::load(self.clone())?;

        println!("SPEC: {:?}", self);

        Ok(())
    }

    async fn invoke(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        let mut model = OpenAIChatModel::load(self.clone())?;
        let input: ChatCompletionRequest = match request.data.clone() {
            ActorInvokeData::ChatCompletion(chat_completion_request) => {
                model.map_request(chat_completion_request)?
            }
            _ => {
                source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));

                return Ok(());
            }
        };

        let messages = model.map_request(input)?;
        let result = model.invoke(messages).await?;

        let results = vec![EntityValue::STRING(result)];

        let result = ActorInvokeResult {
            uuid,
            task_id: request.task_id,
            stream: request.stream,
            metadata: HashMap::new(),
            data: HashMap::from([(String::from("content"), results)]),
        };

        source.do_send(ActorInvokeResponse::Success(result));
        Ok(())
    }

    async fn invoke_stream(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        let mut model = OpenAIChatModel::load(self.clone())?;
        let input: ChatCompletionRequest = match request.data.clone() {
            ActorInvokeData::ChatCompletion(chat_completion_request) => {
                model.map_request(chat_completion_request)?
            }
            _ => {
                source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));

                return Ok(());
            }
        };

        let res = model.prepare(input).await?;
        let mut stream = res.bytes_stream();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(data) => {
                    let chunk_str = String::from_utf8_lossy(&data);
                    let lines = chunk_str.split("data: ");
                    for line in lines {
                        let trimmed = line.trim();

                        if !trimmed.is_empty() {
                            if let Ok(json) = serde_json::from_str::<Value>(trimmed) {
                                if let Some(choices) = json.get("choices") {
                                    if let Some(content) = choices[0]
                                        .get("delta")
                                        .and_then(|delta| delta.get("content"))
                                        .and_then(|c| c.as_str())
                                    {
                                        let result = ActorInvokeResult {
                                            uuid,
                                            task_id: request.task_id,
                                            stream: request.stream,
                                            metadata: HashMap::new(),
                                            data: HashMap::from([(
                                                String::from("content"),
                                                vec![EntityValue::STRING(content.to_string())],
                                            )]),
                                        };

                                        let response = ActorInvokeResponse::Success(result);
                                        source.do_send(response);
                                        actix_rt::task::yield_now().await;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Error reading chunk: {}", e),
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

#[derive(Serialize, Deserialize, Debug)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Choice {
    message: Message,
}

struct OpenAIChatModel {
    pub spec: OpenAIChatSpec,
    pub client: &'static Client,
}

impl OpenAIChatModel {
    pub fn map_request(&self, input: ChatCompletionRequest) -> Result<ChatCompletionRequest> {
        Ok(input)
    }

    pub async fn invoke(&mut self, request: ChatCompletionRequest) -> Result<String> {
        let mut request_body = request;
        request_body.model = self.spec.model.to_string();
        // request_body.max_tokens = self.spec.max_tokens;
        let api_key = some_or_env(self.spec.clone().api_key, "OPENAI_SECRET");

        let base_url = self
            .spec
            .clone()
            .api_key
            .unwrap_or("https://api.openai.com".to_string());

        let res = self
            .client
            .post(format!("{}/v1/chat/completions", base_url))
            .bearer_auth(api_key)
            .json(&request_body)
            .send()
            .await?;

        let text = res.text().await?;
        let response_body: ChatCompletionResponse = serde_json::from_str(&text)?;
        let output = response_body.choices[0].message.content.clone().0;
        match output {
            Either::Left(content) => Ok(content),
            Either::Right(_) => unreachable!(),
        }
    }

    pub async fn prepare(&mut self, request: ChatCompletionRequest) -> Result<Response> {
        let mut request_body = request;
        request_body.model = self.spec.model.to_string();

        let api_key = some_or_env(self.spec.clone().api_key, "OPENAI_SECRET");

        let base_url = self
            .spec
            .clone()
            .api_key
            .unwrap_or("https://api.openai.com".to_string());

        let res = self
            .client
            .post(format!("{}/v1/chat/completions", base_url))
            .bearer_auth(api_key)
            .json(&request_body)
            .send()
            .await?;

        Ok(res)
    }

    pub fn init(spec: OpenAIChatSpec) -> Result<()> {
        let _ = spec;
        Ok(())
    }

    pub fn load(spec: OpenAIChatSpec) -> Result<OpenAIChatModel> {
        Ok(OpenAIChatModel {
            spec,
            client: &HTTP_CLIENT,
        })
    }
}

/*
#[tokio::test]
async fn test_bielik() -> Result<()> {
    use std::env;
    let secret = env::var("OPENAI_SECRET").unwrap();
    let spec = OpenAIChatSpec {
        base_url: Some("https://api.openai.com".to_string()),
        api_key: Some(secret),
        model: "gpt-4o-mini".to_string(),
        max_tokens: None,
        seed: None,
        temperature: None,
        top_p: None,
    };

    let mut model = OpenAIChatModel::load(spec)?;
    let messages = vec![
        ChatMessage::system("you are helpfull assistant"),
        ChatMessage::user("Hello!"),
    ];
    let res = model.invoke(messages).await?;

    println!("RESPONSE: {}", res);
    Ok(())
}
*/
