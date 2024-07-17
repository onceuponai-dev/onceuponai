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
use onceuponai_actors_abstractions::{
    ActorActions, ActorError, ActorInvokeError, ActorInvokeFinish, ActorInvokeInput,
    ActorInvokeOutput, ActorInvokeResult, ActorMetadata, ActorObject,
};
use onceuponai_core::{common::ResultExt, config::read_config_str};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc;
use uuid::Uuid;

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
pub enum ActorKind {
    Main(ActorObject<MainActorSpec>),
    Custom(ActorObject<CustomActorSpec>),
    Gemma(ActorObject<GemmaSpec>),
    Quantized(ActorObject<QuantizedSpec>),
    E5(ActorObject<E5Spec>),
}

impl ActorKind {
    pub fn actor(&self) -> Box<dyn ActorActions> {
        match self {
            ActorKind::Gemma(object) => Box::new(object.spec()),
            ActorKind::Quantized(object) => Box::new(object.spec()),
            ActorKind::E5(object) => Box::new(object.spec()),
            ActorKind::Main(object) => Box::new(object.spec()),
            ActorKind::Custom(object) => Box::new(object.spec()),
        }
    }

    pub fn metadata(&self) -> ActorMetadata {
        match self {
            ActorKind::Gemma(object) => object.metadata(),
            ActorKind::Quantized(object) => object.metadata(),
            ActorKind::E5(object) => object.metadata(),
            ActorKind::Main(object) => object.metadata(),
            ActorKind::Custom(object) => object.metadata(),
        }
    }
}

/*
impl ActorKind {
    pub fn metadata(&self) -> ActorMetadata {
        match self {
            ActorKind::Gemma(object) => object.metadata(),
            ActorKind::Quantized(object) => object.metadata(),
            ActorKind::E5(object) => object.metadata(),
            ActorKind::Main(object) => object.metadata(),
            ActorKind::Custom(object) => object.metadata(),
        }
    }

    pub fn kind(&self) -> String {
        match self {
            ActorKind::Gemma(object) => object.kind(),
            ActorKind::Quantized(object) => self.kind(),
            ActorKind::E5(object) => object.kind(),
            ActorKind::Main(object) => object.kind(),
            ActorKind::Custom(object) => object.kind(),
        }
    }

    fn actor_id(&self) -> &str {
        match self {
            ActorKind::Main(_) => MainActor::ACTOR_ID,
            _ => WorkerActor::ACTOR_ID,
        }
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
            ActorKind::Gemma(object) => crate::llm::gemma::start(object.spec()),
            ActorKind::E5(object) => crate::llm::e5::start(object.spec()),
            ActorKind::Quantized(object) => crate::llm::quantized::start(object.spec()),
            ActorKind::Main(_) => Ok(()),
            ActorKind::Custom(object) => {
                let registry = CUSTOM_ACTOR_REGISTRY.get_or_init(CustomActorRegistry::new);
                let custom_actor = registry
                    .create(&object.spec().name)
                    .expect("Custom actor not found");
                custom_actor.start();
                Ok(())
            }
        }?;

        Ok(())
    }

    pub fn invoke(&self, uuid: Uuid, request: &ActorInvokeRequest) -> Result<ActorInvokeResponse> {
        let response = match self {
            ActorKind::Gemma(_) => crate::llm::gemma::invoke(uuid, request.clone()),
            ActorKind::Quantized(_) => crate::llm::quantized::invoke(uuid, request.clone()),
            ActorKind::E5(_) => crate::llm::e5::invoke(uuid, request.clone()),
            ActorKind::Main(_) => Ok(ActorInvokeResponse::Failure(ActorInvokeError {
                uuid,
                task_id: request.task_id,
                error: ActorError::FatalError(String::from("MAIN ACTOR CAN'T BE INVOKED")),
            })),
            ActorKind::Custom(object) => {
                todo!();
                // let registry = CUSTOM_ACTOR_REGISTRY.get_or_init(CustomActorRegistry::new);
                // let custom_actor = registry
                // .create(&object.spec().name)
                // .expect("Custom actor not found");
                // custom_actor.invoke(uuid, request.clone())
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
            ActorKind::Gemma(_) => todo!(),
            ActorKind::Quantized(_) => {
                crate::llm::quantized::invoke_stream(uuid, request.clone(), callback)
            }
            ActorKind::E5(_) => Err(anyhow!("E5 ACTOR NOT SUPPORT STREAM")),
            ActorKind::Main(_) => Err(anyhow!("MAIN ACTOR NOT SUPPORT STREAM")),
            ActorKind::Custom(_) => Err(anyhow!("MAIN ACTOR NOT SUPPORT STREAM")),
        }
    }
}
*/

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

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
pub enum ActorInvokeResponse {
    Success(ActorInvokeResult),
    Finish(ActorInvokeFinish),
    Failure(ActorInvokeError),
}

impl From<ActorInvokeRequest> for ActorInvokeInput {
    fn from(value: ActorInvokeRequest) -> Self {
        ActorInvokeInput {
            task_id: value.task_id,
            source: value.source,
            stream: value.stream,
            config: value.config,
            data: value.data,
        }
    }
}

impl From<ActorInvokeOutput> for ActorInvokeResponse {
    fn from(value: ActorInvokeOutput) -> Self {
        match value {
            ActorInvokeOutput::Success(val) => ActorInvokeResponse::Success(val),
            ActorInvokeOutput::Finish(val) => ActorInvokeResponse::Finish(val),
            ActorInvokeOutput::Failure(val) => ActorInvokeResponse::Failure(val),
        }
    }
}

pub enum ActorInstance {
    Main(MainActor),
    Worker(WorkerActor),
}

pub struct ActorBuilder {}

impl ActorBuilder {
    pub async fn build(path: &String) -> Result<ActorInstance> {
        let configuration_str = read_config_str(path, Some(true)).await.map_anyhow_err()?;

        let actor_kind: ActorKind = serde_yaml::from_str(&configuration_str)?;

        let actor_box = if let ActorKind::Main(actor) = actor_kind.clone() {
            let actor = actor.setup(MainActor::ACTOR_ID, None);
            let remote_addr = actor.metadata().remote_addr()?;
            ActorInstance::Main(MainActor {
                uuid: Uuid::new_v4(),
                remote_addr,
                connected_actors: HashMap::new(),
                own_addr: actor.own_addr()?,
                actor,
            })
        } else {
            let actor = actor_kind.clone().actor();
            let metadata = actor_kind
                .metadata()
                .setup(WorkerActor::ACTOR_ID, actor.features());
            let remote_addr = metadata.remote_addr()?;

            let act = actor_kind.actor();
            let (sender, rx) = mpsc::channel::<ActorInternalRequest>();

            std::thread::spawn(move || {
                while let Ok(request) = rx.recv() {
                    let is_stream = request.message.stream;
                    let source = request.message.source.clone();
                    let req: &ActorInvokeInput = &request.message.into();
                    if !is_stream {
                        let response = actor.invoke(request.task_id, req).unwrap();
                        let resp: ActorInvokeResponse = response.into();
                        source.do_send(resp)
                    } else {
                        actor.invoke_stream(request.task_id, req, source).unwrap();
                    }
                }
            });

            ActorInstance::Worker(WorkerActor {
                uuid: Uuid::new_v4(),
                own_addr: metadata.own_addr()?,
                seed_addr: metadata.seed_addr()?,
                remote_addr,
                actor: act,
                sender,
                metadata,
            })
        };

        Ok(actor_box)
    }
}

#[derive(RemoteActor)]
#[remote_messages(ActorInfoRequest, ActorInvokeRequest)]
pub struct WorkerActor {
    pub uuid: Uuid,
    pub metadata: ActorMetadata,
    pub actor: Box<dyn ActorActions>,
    pub own_addr: SocketAddr,
    pub seed_addr: SocketAddr,
    pub remote_addr: RemoteAddr,
    pub sender: mpsc::Sender<ActorInternalRequest>,
}

impl WorkerActor {
    pub fn metadata(&self) -> ActorMetadata {
        self.metadata.clone()
    }
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
        let metadata = self.metadata();
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
