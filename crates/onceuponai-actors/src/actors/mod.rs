pub mod main_actor;
use crate::abstractions::{
    ActorActions, ActorInvokeRequest, ActorKindActions, ActorMetadata, ActorObject,
};
use actix::prelude::*;
use actix_telepathy::prelude::*;
use anyhow::Result;
use log::info;
use main_actor::{MainActor, MainActorSpec};
use once_cell::sync::Lazy;
use onceuponai_abstractions::EntityValue;
use onceuponai_core::notifications::{Notification, NotificationLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{mpsc, Arc, Mutex};
use tokio::runtime::Builder;
use tokio::runtime::Runtime;
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

    pub fn build_worker<T>(metadata: ActorMetadata, actor_kind: T) -> Result<WorkerActor<T>>
    where
        T: ActorKindActions + Clone + Send + Sync + 'static,
    {
        let actor = actor_kind.clone().actor();
        let metadata = metadata.setup(WorkerActor::<T>::ACTOR_ID, actor.features());
        let remote_addr = metadata.remote_addr()?;

        Ok(WorkerActor {
            uuid: Uuid::new_v4(),
            own_addr: metadata.own_addr()?,
            seed_addr: metadata.seed_addr()?,
            remote_addr,
            actor_kind: Box::new(actor_kind),
            metadata,
        })
    }
}

#[derive(RemoteActor)]
#[remote_messages(ActorInfoRequest, ActorInvokeRequest)]
pub struct WorkerActor<T>
where
    T: ActorKindActions + Clone + Send + Sync + 'static,
{
    pub uuid: Uuid,
    pub metadata: ActorMetadata,
    pub actor_kind: Box<T>,
    pub own_addr: SocketAddr,
    pub seed_addr: SocketAddr,
    pub remote_addr: RemoteAddr,
}

impl<T> WorkerActor<T>
where
    T: ActorKindActions + Clone + Send + Sync + 'static,
{
    pub fn metadata(&self) -> ActorMetadata {
        self.metadata.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorInternalRequest {
    pub task_id: Uuid,
    pub message: ActorInvokeRequest,
}

impl<T> Actor for WorkerActor<T>
where
    T: ActorKindActions + Clone + Send + Sync + 'static,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.register(ctx.address().recipient());
    }
}

impl<T> Handler<ActorInfoRequest> for WorkerActor<T>
where
    T: ActorKindActions + Clone + Send + Sync + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: ActorInfoRequest, _ctx: &mut Self::Context) -> Self::Result {
        let metadata = self.metadata();
        let model_info = ActorInfo {
            uuid: self.uuid,
            metadata: metadata.clone(),
            source: self.remote_addr.clone(),
            kind: self.actor_kind.actor().kind(),
        };
        info!("MODEL INFO REQUEST: {:?}", msg);
        Notification::publish(
            &format!(
                "ACTOR {}/{} ({}) CONNECTED",
                metadata.name,
                self.actor_kind.actor().kind(),
                self.uuid
            ),
            NotificationLevel::Success,
        );
        msg.source.do_send(model_info)
    }
}

impl<T> Handler<ActorInvokeRequest> for WorkerActor<T>
where
    T: ActorKindActions + Clone + Send + Sync + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: ActorInvokeRequest, _ctx: &mut Self::Context) -> Self::Result {
        info!("MODEL INVOKE REQUEST: {:?}", msg);

        let actor = self.actor_kind.actor();
        let is_stream = msg.stream;
        let source = msg.source.clone();

        if !is_stream {
            actix_rt::spawn(async move {
                actor
                    .invoke(msg.task_id, &msg.clone(), source)
                    .await
                    .unwrap();
            });
        } else {
            actix_rt::spawn(async move {
                actor
                    .invoke_stream(msg.task_id, &msg.clone(), source)
                    .await
                    .unwrap();
            });
        }
    }
}
