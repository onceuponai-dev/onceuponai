pub mod main_actor;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use log::debug;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// https://github.com/yummyml/yummy/blob/master/yummy-rs/yummy-delta/src/apply.rs
// https://github.com/yummyml/yummy/blob/master/yummy-rs/yummy-core/src/config.rs
// https://github.com/yummyml/yummy/blob/master/yummy-rs/yummy-delta/tests/config/01_bronze_tables.yaml

#[derive(RemoteMessage, Serialize, Deserialize, Debug)]
pub struct ActorInfo {
    pub uuid: Uuid,
    pub metadata: ActorMetadata,
    pub addr: RemoteAddr,
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
pub struct ActorMetadata {
    pub name: String,
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

pub struct GemmaConfig {}

pub enum ActorObject {
    Gemma {
        metadata: ActorMetadata,
        spec: GemmaConfig,
    },
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug)]
#[with_source(source)]
pub struct ActorInfoRequest {
    pub source: RemoteAddr,
}

#[derive(RemoteActor)]
#[remote_messages(ActorInfoRequest)]
pub struct ActorWrapper {
    pub uuid: Uuid,
    pub actor: ActorObject,
    pub remote_addr: RemoteAddr,
}

impl Actor for ActorWrapper {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.register(ctx.address().recipient());
    }
}

impl Handler<ActorInfoRequest> for ActorWrapper {
    type Result = ();

    fn handle(&mut self, msg: ActorInfoRequest, _ctx: &mut Self::Context) -> Self::Result {
        let metadata = match &self.actor {
            ActorObject::Gemma { metadata, spec: _ } => metadata,
        };
        let model_info = ActorInfo {
            uuid: self.uuid,
            metadata: metadata.clone(),
            addr: self.remote_addr.clone(),
        };
        debug!("MODEL INFO REQUEST: {:?}", msg);

        msg.source.do_send(model_info)
    }
}
