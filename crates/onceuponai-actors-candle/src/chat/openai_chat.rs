use actix_telepathy::RemoteAddr;
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors::abstractions::{
    ActorActions, ActorError, ActorInvokeError, ActorInvokeFinish, ActorInvokeRequest,
    ActorInvokeResponse, ActorInvokeResult,
};
use onceuponai_core::common::some_or_env;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
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
        "openaiChat".to_string()
    }

    fn init(&self) -> Result<()> {
        OpenAIChatModel::init(self.clone())
    }

    fn start(&self) -> Result<()> {
        OpenAIChatModel::load(self.clone())?;

        println!("SPEC: {:?}", self);

        Ok(())
    }

    async fn invoke(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
    ) -> Result<ActorInvokeResponse> {
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

        let mut model = OpenAIChatModel::load(self.clone())?;

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

        Ok(ActorInvokeResponse::Success(result))
    }

    async fn invoke_stream(
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

        let mut model = OpenAIChatModel::load(self.clone())?;

        let messages = model.map_request(input)?;
        let res = model.prepare(messages).await?;
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
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessage {
    role: String,
    content: String,
}

impl ChatMessage {
    fn system(content: &str) -> ChatMessage {
        ChatMessage {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    fn user(content: &str) -> ChatMessage {
        ChatMessage {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Choice {
    message: ChatMessage,
}

struct OpenAIChatModel {
    pub spec: OpenAIChatSpec,
    pub client: &'static Client,
}

impl OpenAIChatModel {
    pub fn map_request(&self, input: Option<&Vec<EntityValue>>) -> Result<Vec<ChatMessage>> {
        let messages: Vec<ChatMessage> = input
            .expect("MESSAGE")
            .iter()
            .map(|x| match x {
                EntityValue::MESSAGE { role, content } => ChatMessage {
                    role: role.clone(),
                    content: content.clone(),
                },
                _ => todo!(),
            })
            .collect();
        Ok(messages)
    }

    pub async fn invoke(&mut self, messages: Vec<ChatMessage>) -> Result<String> {
        let request_body = ChatCompletionRequest {
            model: self.spec.model.to_string(),
            messages,
            max_tokens: self.spec.max_tokens,
            stream: Some(false),
        };

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

        let response_body: ChatCompletionResponse = res.json().await?;
        let output = &response_body.choices[0].message.content;
        Ok(output.to_string())
    }

    pub async fn prepare(&mut self, messages: Vec<ChatMessage>) -> Result<Response> {
        let request_body = ChatCompletionRequest {
            model: self.spec.model.to_string(),
            messages,
            stream: Some(true),
            max_tokens: self.spec.max_tokens,
        };

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

#[tokio::test]
async fn test_bielik() -> Result<()> {
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
