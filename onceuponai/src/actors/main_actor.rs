use super::{
    ActorInfo, ActorInfoRequest, ActorInvokeResponse, ActorObject, ActorStartInvokeRequest,
};
use crate::actors::{ActorInvokeRequest, WorkerActor};
use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use actix_telepathy::prelude::*;
use once_cell::sync::OnceCell;
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{collections::HashMap, net::SocketAddr};
use uuid::Uuid;

pub static CONNECTED_ACTORS: OnceCell<Arc<Mutex<HashMap<Uuid, ActorInfo>>>> = OnceCell::new();
pub static INVOKE_TASKS: OnceCell<Arc<Mutex<HashMap<Uuid, InvokeTask>>>> = OnceCell::new();

#[derive(Debug)]
pub struct InvokeTask {
    pub time: Instant,
    pub sender: mpsc::Sender<ActorInvokeResponse>,
}

#[derive(RemoteActor, Clone)]
#[remote_messages(ActorInfo, ActorInvokeResponse)]
pub struct MainActor {
    pub uuid: Uuid,
    pub actor: ActorObject,
    pub own_addr: SocketAddr,
    pub remote_addr: RemoteAddr,
    pub connected_actors: HashMap<Uuid, ActorInfo>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MainActorAuthConfig {
    pub oidc: Option<MainActorOidcConfig>,
    pub _auth_token: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MainActorConfig {
    pub server_host: String,
    pub server_port: u16,
    pub log_level: Option<String>,
    pub workers: Option<usize>,
    pub invoke_timeout: Option<u64>,
    pub session_key: Option<String>,
    pub personal_access_token_secret: Option<String>,
    pub auth: Option<MainActorAuthConfig>,
}

impl MainActorConfig {
    pub fn oidc(&self) -> MainActorOidcConfig {
        self.auth.clone().expect("AUTH").oidc.expect("OIDC")
    }

    pub fn _auth_token(&self) -> String {
        self.auth
            .clone()
            .expect("AUTH")
            ._auth_token
            .expect("_AUTH_TOKEN")
    }

    pub fn _auth_token_set(&mut self, _auth_token: String) {
        self.auth = Some(MainActorAuthConfig {
            _auth_token: Some(_auth_token),
            oidc: None,
        });
    }

    pub fn is_oidc(&self) -> bool {
        if let Some(auth) = self.auth.clone() {
            return auth.oidc.is_some();
        }

        false
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct MainActorOidcConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

impl Actor for MainActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.register(ctx.address().recipient());
        self.subscribe_system_async::<ClusterLog>(ctx);
        CONNECTED_ACTORS
            .set(Arc::new(Mutex::new(HashMap::new())))
            .unwrap();
        INVOKE_TASKS
            .set(Arc::new(Mutex::new(HashMap::new())))
            .unwrap();
    }
}

impl Handler<ActorInfo> for MainActor {
    type Result = ();

    fn handle(&mut self, actor_info: ActorInfo, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received model state: {:?}", actor_info);
        self.connected_actors
            .insert(actor_info.uuid, actor_info.clone());
        CONNECTED_ACTORS
            .get()
            .expect("CONNECTED_MODELS")
            .lock()
            .unwrap()
            .insert(actor_info.uuid, actor_info);
    }
}

impl Handler<ActorStartInvokeRequest> for MainActor {
    type Result = ();

    fn handle(&mut self, msg: ActorStartInvokeRequest, _ctx: &mut Self::Context) -> Self::Result {
        debug!("START INVOKE REQUEST: {:?}", msg);
        let kind = msg.kind;
        let name = msg.name;
        debug!("KIND/NAME: {kind:?}/{name:?}");

        let actors: Vec<ActorInfo> = self
            .connected_actors
            .iter()
            .filter(|a| a.1.kind == kind && a.1.metadata.name == name)
            .map(|a| a.1.clone())
            .collect();

        let worker_actor = actors
            .choose(&mut rand::thread_rng())
            .expect("WORKER_ACTOR");
        worker_actor.source.do_send(ActorInvokeRequest {
            task_id: msg.task_id,
            stream: msg.stream,
            config: msg.config,
            data: msg.data,
            source: self.remote_addr.clone(),
        });
    }
}

impl Handler<ActorInvokeResponse> for MainActor {
    type Result = ();

    fn handle(&mut self, msg: ActorInvokeResponse, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received invoke response: {:?}", msg);
        match &msg {
            ActorInvokeResponse::Failure(result) => {
                let mut tasks = INVOKE_TASKS.get().expect("INVOKE_TASKS").lock().unwrap();

                if let Some(task) = tasks.remove(&result.task_id) {
                    let _ = task.sender.send(msg);
                }
            }
            ActorInvokeResponse::Success(result) => {
                let mut tasks = INVOKE_TASKS.get().expect("INVOKE_TASKS").lock().unwrap();
                if !result.stream {
                    if let Some(task) = tasks.remove(&result.task_id) {
                        let _ = task.sender.send(msg);
                    }
                } else if let Some(task) = tasks.get(&result.task_id) {
                    let _ = task.sender.send(msg);
                }
            }
            ActorInvokeResponse::Finish(result) => {
                let mut tasks = INVOKE_TASKS.get().expect("INVOKE_TASKS").lock().unwrap();
                if let Some(task) = tasks.remove(&result.task_id) {
                    let _ = task.sender.send(msg);
                }
            }
        }
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
                debug!("MEMBER LEFT {:?}", addr);
                let actors: Vec<Uuid> = CONNECTED_ACTORS
                    .get()
                    .expect("CONNECTED_MODELS")
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|a| a.1.source.node.socket_addr == addr)
                    .map(|a| *a.0)
                    .collect();

                for actor in actors {
                    CONNECTED_ACTORS
                        .get()
                        .expect("CONNECTED_MODELS")
                        .lock()
                        .unwrap()
                        .remove(&actor);
                }
            }
        }
    }
}

impl ClusterListener for MainActor {}
