pub mod custom_actor;
pub mod main_actor;
use crate::llm::{e5::E5Spec, gemma::GemmaSpec, quantized::QuantizedSpec};
use actix::prelude::*;
use actix_telepathy::prelude::*;
use anyhow::{anyhow, Result};
use custom_actor::{CustomActorRegistry, CustomActorSpec, CUSTOM_ACTOR_REGISTRY};
use log::debug;
use main_actor::{MainActor, MainActorSpec};
use onceuponai_abstractions::EntityValue;
use onceuponai_actors_abstractions::ActorMetadata;
use onceuponai_core::{common::ResultExt, config::read_config_str};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ActorError {
    FatalError(String),
    NetworkError(String),
    BadRequest(String),
}

// https://github.com/yummyml/yummy/blob/master/yummy-rs/yummy-delta/src/apply.rs
// https://github.com/yummyml/yummy/blob/master/yummy-rs/yummy-core/src/config.rs
// https://github.com/yummyml/yummy/blob/master/yummy-rs/yummy-delta/tests/config/01_bronze_tables.yaml

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
#[with_source(source)]
pub struct ActorInfo {
    pub uuid: Uuid,
    pub metadata: ActorMetadata,
    pub source: RemoteAddr,
    pub kind: String,
}

#[derive(RemoteMessage, Serialize, Deserialize, Clone)]
pub struct ModelRequest {
    pub uuid: Uuid,
    pub prompt: String,
}

#[derive(RemoteMessage, Serialize, Deserialize, Clone)]
pub struct ModelResponse {
    pub uuid: Uuid,
    pub response: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ActorObject {
    Main {
        metadata: ActorMetadata,
        spec: MainActorSpec,
    },
    Custom {
        metadata: ActorMetadata,
        spec: CustomActorSpec,
    },
    Gemma {
        metadata: ActorMetadata,
        spec: GemmaSpec,
    },
    Quantized {
        metadata: ActorMetadata,
        spec: QuantizedSpec,
    },
    E5 {
        metadata: ActorMetadata,
        spec: E5Spec,
    },
}

impl ActorObject {
    pub fn metadata(&self) -> ActorMetadata {
        match self {
            ActorObject::Gemma { metadata, spec: _ } => {
                let mut m = metadata.clone();
                m.features = Some(vec!["chat".to_string()]);
                m
            }

            ActorObject::Quantized { metadata, spec: _ } => {
                let mut m = metadata.clone();

                m.features = Some(vec!["chat".to_string()]);
                m
            }
            ActorObject::E5 { metadata, spec: _ } => {
                let mut m = metadata.clone();
                m.features = Some(vec!["embed".to_string()]);
                m
            }
            ActorObject::Main { metadata, spec: _ } => metadata.clone(),
            ActorObject::Custom { metadata, spec: _ } => metadata.clone(),
        }
    }

    pub fn kind(&self) -> String {
        match self {
            ActorObject::Gemma {
                metadata: _,
                spec: _,
            } => "gemma".to_string(),
            ActorObject::Quantized {
                metadata: _,
                spec: _,
            } => "quantized".to_string(),
            ActorObject::E5 {
                metadata: _,
                spec: _,
            } => "e5".to_string(),
            ActorObject::Main {
                metadata: _,
                spec: _,
            } => "main".to_string(),
            ActorObject::Custom {
                metadata: _,
                spec: _,
            } => "main".to_string(),
        }
    }

    fn actor_id(&self) -> &str {
        match self {
            ActorObject::Main {
                metadata: _,
                spec: _,
            } => MainActor::ACTOR_ID,
            _ => WorkerActor::ACTOR_ID,
        }
    }

    fn is_main(&self) -> bool {
        let is_main = matches!(
            self,
            ActorObject::Main {
                metadata: _,
                spec: _
            }
        );
        is_main
    }

    pub fn own_addr(&self) -> Result<SocketAddr> {
        let socket_addr: SocketAddr = self.metadata().actor_host.parse::<SocketAddr>()?;
        Ok(socket_addr)
    }

    pub fn seed_addr(&self) -> Result<SocketAddr> {
        let socket_addr = self
            .metadata()
            .actor_seed
            .clone()
            .expect("SEED REQUIRED")
            .parse::<SocketAddr>()?;
        Ok(socket_addr)
    }

    pub fn remote_addr(&self) -> Result<RemoteAddr> {
        let socket_addr: SocketAddr = self.own_addr()?;
        let remote_addr = RemoteAddr::new_from_id(socket_addr, self.actor_id());
        Ok(remote_addr)
    }

    pub fn start(&self) -> Result<()> {
        match self {
            ActorObject::Gemma { metadata: _, spec } => crate::llm::gemma::start(spec.clone()),
            ActorObject::E5 { metadata: _, spec } => crate::llm::e5::start(spec.clone()),
            ActorObject::Quantized { metadata: _, spec } => {
                crate::llm::quantized::start(spec.clone())
            }
            ActorObject::Main {
                metadata: _,
                spec: _,
            } => Ok(()),
            ActorObject::Custom { metadata: _, spec } => {
                let registry = CUSTOM_ACTOR_REGISTRY.get_or_init(CustomActorRegistry::new);
                let custom_actor = registry.create(&spec.name).expect("Custom actor not found");
                custom_actor.start();
                Ok(())
            }
        }?;

        Ok(())
    }

    pub fn invoke(&self, uuid: Uuid, request: &ActorInvokeRequest) -> Result<ActorInvokeResponse> {
        let response = match self {
            ActorObject::Gemma {
                metadata: _,
                spec: _,
            } => crate::llm::gemma::invoke(uuid, request.clone()),
            ActorObject::Quantized {
                metadata: _,
                spec: _,
            } => crate::llm::quantized::invoke(uuid, request.clone()),
            ActorObject::E5 {
                metadata: _,
                spec: _,
            } => crate::llm::e5::invoke(uuid, request.clone()),
            ActorObject::Main {
                metadata: _,
                spec: _,
            } => Ok(ActorInvokeResponse::Failure(ActorInvokeError {
                uuid,
                task_id: request.task_id,
                error: ActorError::FatalError(String::from("MAIN ACTOR CAN'T BE INVOKED")),
            })),
            ActorObject::Custom { metadata: _, spec } => {
                let registry = CUSTOM_ACTOR_REGISTRY.get_or_init(CustomActorRegistry::new);
                let custom_actor = registry.create(&spec.name).expect("Custom actor not found");
                custom_actor.invoke(uuid, request.clone())
            }
        }?;

        Ok(response)
    }

    pub fn invoke_stream<F>(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
        mut callback: F,
    ) -> Result<()>
    where
        F: FnMut(ActorInvokeResponse),
    {
        match self {
            ActorObject::Gemma {
                metadata: _,
                spec: _,
            } => todo!(),
            ActorObject::Quantized {
                metadata: _,
                spec: _,
            } => crate::llm::quantized::invoke_stream(uuid, request.clone(), callback),
            ActorObject::E5 {
                metadata: _,
                spec: _,
            } => Err(anyhow!("E5 ACTOR NOT SUPPORT STREAM")),
            ActorObject::Main {
                metadata: _,
                spec: _,
            } => Err(anyhow!("MAIN ACTOR NOT SUPPORT STREAM")),
            ActorObject::Custom { metadata: _, spec } => {
                Err(anyhow!("MAIN ACTOR NOT SUPPORT STREAM"))
            }
        }
    }
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug)]
#[with_source(source)]
pub struct ActorInfoRequest {
    pub source: RemoteAddr,
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
pub struct ActorStartInvokeRequest {
    pub task_id: Uuid,
    pub kind: String,
    pub name: String,
    pub stream: bool,
    pub config: HashMap<String, EntityValue>,
    pub data: HashMap<String, Vec<EntityValue>>,
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
#[with_source(source)]
pub struct ActorInvokeRequest {
    pub task_id: Uuid,
    pub source: RemoteAddr,
    pub stream: bool,
    pub config: HashMap<String, EntityValue>,
    pub data: HashMap<String, Vec<EntityValue>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorInvokeResult {
    pub uuid: Uuid,
    pub task_id: Uuid,
    pub stream: bool,
    pub metadata: HashMap<String, EntityValue>,
    pub data: HashMap<String, Vec<EntityValue>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorInvokeFinish {
    pub uuid: Uuid,
    pub task_id: Uuid,
    pub stream: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorInvokeError {
    pub uuid: Uuid,
    pub task_id: Uuid,
    pub error: ActorError,
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
pub enum ActorInvokeResponse {
    Success(ActorInvokeResult),
    Finish(ActorInvokeFinish),
    Failure(ActorInvokeError),
}

pub enum ActorInstance {
    Main(MainActor),
    Worker(WorkerActor),
}

pub struct ActorBuilder {}

impl ActorBuilder {
    pub async fn build(path: &String) -> Result<ActorInstance> {
        let configuration_str = read_config_str(path, Some(true)).await.map_anyhow_err()?;
        // let mut actors = Vec::new();

        let actor: ActorObject = serde_yaml::from_str(&configuration_str)?;
        let remote_addr = actor.remote_addr()?;

        let actor_box = if actor.is_main() {
            ActorInstance::Main(MainActor {
                uuid: Uuid::new_v4(),
                remote_addr,
                connected_actors: HashMap::new(),
                own_addr: actor.own_addr()?,
                actor,
            })
        } else {
            let (sender, rx) = mpsc::channel::<ActorInternalRequest>();

            let a = actor.clone();
            std::thread::spawn(move || {
                while let Ok(request) = rx.recv() {
                    if !request.message.stream {
                        let response = a.invoke(request.task_id, &request.message).unwrap();
                        request.message.source.do_send(response)
                    } else {
                        a.invoke_stream(
                            request.task_id,
                            &request.message,
                            |response: ActorInvokeResponse| {
                                request.message.source.do_send(response);
                            },
                        )
                        .unwrap();
                    }
                }
            });

            ActorInstance::Worker(WorkerActor {
                uuid: Uuid::new_v4(),
                own_addr: actor.own_addr()?,
                seed_addr: actor.seed_addr()?,
                remote_addr,
                actor,
                sender,
            })
        };

        Ok(actor_box)
    }
}

#[derive(RemoteActor, Debug, Clone)]
#[remote_messages(ActorInfoRequest, ActorInvokeRequest)]
pub struct WorkerActor {
    pub uuid: Uuid,
    pub actor: ActorObject,
    pub own_addr: SocketAddr,
    pub seed_addr: SocketAddr,
    pub remote_addr: RemoteAddr,
    pub sender: mpsc::Sender<ActorInternalRequest>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorInternalRequest {
    pub task_id: Uuid,
    pub message: ActorInvokeRequest,
}

impl Actor for WorkerActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.actor.start().unwrap();
        self.register(ctx.address().recipient());
    }
}

impl Handler<ActorInfoRequest> for WorkerActor {
    type Result = ();

    fn handle(&mut self, msg: ActorInfoRequest, _ctx: &mut Self::Context) -> Self::Result {
        let metadata = self.actor.metadata();
        let model_info = ActorInfo {
            uuid: self.uuid,
            metadata: metadata.clone(),
            source: self.remote_addr.clone(),
            kind: self.actor.kind(),
        };
        debug!("MODEL INFO REQUEST: {:?}", msg);
        msg.source.do_send(model_info)
    }
}

impl Handler<ActorInvokeRequest> for WorkerActor {
    type Result = ();

    fn handle(&mut self, msg: ActorInvokeRequest, _ctx: &mut Self::Context) -> Self::Result {
        debug!("MODEL INVOKE REQUEST: {:?}", msg);

        self.sender
            .send(ActorInternalRequest {
                task_id: self.uuid,
                message: msg.clone(),
            })
            .unwrap();
    }
}
