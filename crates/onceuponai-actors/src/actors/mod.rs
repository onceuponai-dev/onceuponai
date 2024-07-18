pub mod custom_actor;
pub mod main_actor;
use crate::abstractions::{
    ActorActions, ActorInvokeRequest, ActorInvokeResponse, ActorMetadata, ActorObject,
};
use crate::llm::{e5::E5Spec, gemma::GemmaSpec, quantized::QuantizedSpec};
use actix::prelude::*;
use actix_telepathy::prelude::*;
use anyhow::Result;
use custom_actor::CustomActorSpec;
use log::debug;
use main_actor::{MainActor, MainActorSpec};
use onceuponai_abstractions::EntityValue;
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
                    if !is_stream {
                        let response = actor.invoke(request.task_id, &request.message).unwrap();
                        source.do_send(response)
                    } else {
                        actor
                            .invoke_stream(request.task_id, &request.message, source)
                            .unwrap();
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
