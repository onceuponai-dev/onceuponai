use super::model_actor::ModelKind;
use crate::actors::model_actor::{ModelActor, ModelInfoRequest};
use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use actix_telepathy::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(RemoteMessage, Serialize, Deserialize, Debug)]
pub struct ModelInfo {
    pub uuid: Uuid,
    pub name: String,
    pub kind: ModelKind,
    pub addr: RemoteAddr,
}

#[derive(RemoteActor)]
#[remote_messages(ModelInfo)]
pub struct MainActor {
    pub uuid: Uuid,
    pub own_addr: SocketAddr,
    pub remote_addr: RemoteAddr,
    pub models: Vec<ModelInfo>,
}

impl Actor for MainActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.register(ctx.address().recipient());
        self.subscribe_system_async::<ClusterLog>(ctx);
        self.models = Vec::new();
    }
}

impl Handler<ModelInfo> for MainActor {
    type Result = ();

    fn handle(&mut self, msg: ModelInfo, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received model state: {:?}", msg);
        self.models.push(msg);
    }
}

impl Handler<ClusterLog> for MainActor {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ClusterLog::NewMember(node) => {
                debug!("New model joined the cluster. Node: {node:?}");
                if self.own_addr != node.socket_addr {
                    let model_addr = node.get_remote_addr(ModelActor::ACTOR_ID.to_string());
                    model_addr.do_send(ModelInfoRequest {
                        uuid: self.uuid,
                        main_actor_addr: self.remote_addr.clone(),
                    });
                }
            }
            ClusterLog::MemberLeft(addr) => {
                debug!("member left {:?}", addr);
            }
        }
    }
}

impl ClusterListener for MainActor {}
