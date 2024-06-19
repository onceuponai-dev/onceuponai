use super::{ActorInfo, ActorInfoRequest};
use crate::actors::ActorWrapper;
use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use actix_telepathy::prelude::*;
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(RemoteActor)]
#[remote_messages(ActorInfo)]
pub struct MainActor {
    pub uuid: Uuid,
    pub own_addr: SocketAddr,
    pub remote_addr: RemoteAddr,
    pub models: Vec<ActorInfo>,
}

impl Actor for MainActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.register(ctx.address().recipient());
        self.subscribe_system_async::<ClusterLog>(ctx);
        self.models = Vec::new();
    }
}

impl Handler<ActorInfo> for MainActor {
    type Result = ();

    fn handle(&mut self, msg: ActorInfo, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received model state: {:?}", msg);
        self.models.push(msg);
    }
}

impl Handler<ClusterLog> for MainActor {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ClusterLog::NewMember(node) => {
                debug!("New model joined the cluster. Node: {node:?}");
                if self.own_addr != node.socket_addr {
                    let model_addr = node.get_remote_addr(ActorWrapper::ACTOR_ID.to_string());
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
