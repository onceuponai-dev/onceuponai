pub mod main_actor;
use crate::abstractions::{ActorActions, ActorInvokeRequest, ActorMetadata, ActorObject};
use actix::prelude::*;
use actix_telepathy::prelude::*;
use anyhow::Result;
use log::info;
use main_actor::{MainActor, MainActorSpec};
use onceuponai_abstractions::EntityValue;
use onceuponai_core::notifications::{Notification, NotificationLevel};
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

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
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

pub struct ActorBuilder {}

impl ActorBuilder {
    pub fn build_main(actor: ActorObject<MainActorSpec>) -> Result<MainActor> {
        let actor = actor.setup(MainActor::ACTOR_ID, None);
        let remote_addr = actor.metadata().remote_addr()?;
        Ok(MainActor {
            uuid: Uuid::new_v4(),
            remote_addr,
            connected_actors: HashMap::new(),
            own_addr: actor.own_addr()?,
            actor,
        })
    }

    pub fn build_worker<T>(metadata: ActorMetadata, actor_factory: T) -> Result<WorkerActor>
    where
        T: Fn() -> Box<dyn ActorActions>,
    {
        let actor = actor_factory();
        let metadata = metadata.setup(WorkerActor::ACTOR_ID, actor.features());
        let remote_addr = metadata.remote_addr()?;

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

        Ok(WorkerActor {
            uuid: Uuid::new_v4(),
            own_addr: metadata.own_addr()?,
            seed_addr: metadata.seed_addr()?,
            remote_addr,
            actor: actor_factory(),
            sender,
            metadata,
        })
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
        info!("MODEL INFO REQUEST: {:?}", msg);
        Notification::publish(
            &format!(
                "ACTOR {}/{} ({}) CONNECTED",
                metadata.name,
                self.actor.kind(),
                self.uuid
            ),
            NotificationLevel::Success,
        );
        msg.source.do_send(model_info)
    }
}

impl Handler<ActorInvokeRequest> for WorkerActor {
    type Result = ();

    fn handle(&mut self, msg: ActorInvokeRequest, _ctx: &mut Self::Context) -> Self::Result {
        info!("MODEL INVOKE REQUEST: {:?}", msg);

        self.sender
            .send(ActorInternalRequest {
                task_id: self.uuid,
                message: msg.clone(),
            })
            .unwrap();
    }
}
