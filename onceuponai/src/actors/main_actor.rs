use super::{ActorInfo, ActorInfoRequest, ActorInfoResponse, ActorInvokeResponse, ActorObject};
use crate::actors::WorkerActor;
use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use actix_telepathy::prelude::*;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, net::SocketAddr};
use uuid::Uuid;

pub static CONNECTED_ACTORS: OnceCell<Arc<Mutex<HashMap<Uuid, ActorInfo>>>> = OnceCell::new();

#[derive(RemoteActor, Clone)]
#[remote_messages(ActorInfoResponse, ActorInvokeResponse)]
pub struct MainActor {
    pub uuid: Uuid,
    pub actor: ActorObject,
    pub own_addr: SocketAddr,
    pub remote_addr: RemoteAddr,
    pub connected_actors: HashMap<Uuid, ActorInfo>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MainActorConfig {
    pub server_host: String,
    pub server_port: u16,
    pub log_level: Option<String>,
    pub workers: Option<usize>,
}

impl Actor for MainActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.register(ctx.address().recipient());
        self.subscribe_system_async::<ClusterLog>(ctx);
        CONNECTED_ACTORS
            .set(Arc::new(Mutex::new(HashMap::new())))
            .unwrap();
    }
}

impl Handler<ActorInfoResponse> for MainActor {
    type Result = ();

    fn handle(&mut self, msg: ActorInfoResponse, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received model state: {:?}", msg);
        match msg {
            ActorInfoResponse::Success(actor_info) => {
                self.connected_actors
                    .insert(actor_info.uuid, actor_info.clone());
                CONNECTED_ACTORS
                    .get()
                    .expect("CONNECTED_MODELS")
                    .lock()
                    .unwrap()
                    .insert(actor_info.uuid, actor_info);
            }
            ActorInfoResponse::Failure(error) => debug!("{error:?}"),
        }
    }
}

impl Handler<ActorInvokeResponse> for MainActor {
    type Result = ();

    fn handle(&mut self, msg: ActorInvokeResponse, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received invoke response: {:?}", msg);
    }
}

impl Handler<ClusterLog> for MainActor {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ClusterLog::NewMember(node) => {
                debug!("New model joined the cluster. Node: {node:?}");
                if self.own_addr != node.socket_addr {
                    let model_addr = node.get_remote_addr(WorkerActor::ACTOR_ID.to_string());
                    model_addr.do_send(ActorInfoRequest {
                        source: self.remote_addr.clone(),
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
