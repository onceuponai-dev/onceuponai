use crate::actors::main_actor::MainActor;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use log::debug;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::main_actor::ModelInfo;

// https://github.com/yummyml/yummy/blob/master/yummy-rs/yummy-delta/src/apply.rs
// https://github.com/yummyml/yummy/blob/master/yummy-rs/yummy-core/src/config.rs
// https://github.com/yummyml/yummy/blob/master/yummy-rs/yummy-delta/tests/config/01_bronze_tables.yaml

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

#[derive(RemoteMessage, Serialize, Deserialize, Clone, Debug)]
pub struct ModelState {
    pub uuid: Uuid,
    pub name: String,
    pub kind: ModelKind,
}

#[derive(RemoteMessage, Serialize, Deserialize, Clone, Debug)]
pub enum ModelKind {
    Gemma,
    Quantized,
    E5,
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug)]
pub struct ModelInfoRequest {
    pub uuid: Uuid,
    pub main_actor_addr: RemoteAddr,
}

#[derive(RemoteActor)]
#[remote_messages(ModelInfoRequest)]
pub struct ModelActor {
    pub uuid: Uuid,
    pub name: String,
    pub kind: ModelKind,
    pub remote_addr: RemoteAddr,
    pub main_actor_addr: Addr<MainActor>,
}

impl Actor for ModelActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        debug!("HELLO FROM LLM");
        debug!("INITIALIZE LLM");
        std::thread::sleep(std::time::Duration::from_secs(5));

        //MOVE THIS INTO cluster::mod
        self.uuid = Uuid::new_v4();
        self.kind = ModelKind::Quantized;
        self.name = "Bielik-7b".to_string();

        self.register(ctx.address().recipient());
    }
}

impl Handler<ModelInfoRequest> for ModelActor {
    type Result = ();

    fn handle(&mut self, msg: ModelInfoRequest, _ctx: &mut Self::Context) -> Self::Result {
        let model_info = ModelInfo {
            uuid: msg.uuid,
            name: self.name.clone(),
            kind: self.kind.clone(),
            addr: self.remote_addr.clone(),
        };

        msg.main_actor_addr.do_send(model_info)
    }
}
