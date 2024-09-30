use actix::prelude::*;
use actix_telepathy::prelude::*;
use anyhow::Result;
use async_trait::async_trait;
use onceuponai_abstractions::EntityValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use uuid::Uuid;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ActorError {
    FatalError(String),
    NetworkError(String),
    BadRequest(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorMetadata {
    pub name: String,
    pub features: Option<Vec<String>>,
    pub actor_id: Option<String>,
    pub actor_host: String,
    pub actor_seed: Option<String>,
    pub sidecar_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorInvokeResult {
    pub uuid: Uuid,
    pub task_id: Uuid,
    pub stream: bool,
    pub metadata: HashMap<String, EntityValue>,
    pub data: HashMap<String, Vec<EntityValue>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorInvokeFinish {
    pub uuid: Uuid,
    pub task_id: Uuid,
    pub stream: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorInvokeError {
    pub uuid: Uuid,
    pub task_id: Uuid,
    pub error: ActorError,
}

pub trait ActorKindActions: Clone + Send + Sync {
    fn actor(&self) -> Box<dyn ActorActions>;
    fn metadata(&self) -> ActorMetadata;
}

#[async_trait]
pub trait ActorActions: Send + Sync {
    fn features(&self) -> Option<Vec<String>> {
        None
    }

    fn is_main(&self) -> bool {
        false
    }

    fn kind(&self) -> String;

    fn init(&self) -> Result<()>;
    fn start(&self) -> Result<()>;
    async fn invoke(&self, uuid: Uuid, request: &ActorInvokeRequest)
        -> Result<ActorInvokeResponse>;

    #[allow(unused_variables)]
    async fn invoke_stream(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        unreachable!("invoke_stream method is not expected to be called.");
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub struct ActorObject<T>
where
    T: Clone + ActorActions,
{
    metadata: ActorMetadata,
    spec: T,
}

impl<T> ActorObject<T>
where
    T: Clone + ActorActions,
{
    pub fn new(metadata: ActorMetadata, spec: T) -> Self {
        ActorObject::<T> { metadata, spec }
    }

    pub fn setup(mut self, actor_id: &str, features: Option<Vec<String>>) -> Self {
        self.metadata = self.metadata.setup(actor_id, features);
        self
    }

    pub fn actor_id(&self) -> String {
        self.metadata.actor_id()
    }

    pub fn is_main(&self) -> bool {
        self.spec.is_main()
    }

    pub fn kind(&self) -> String {
        self.spec.kind()
    }

    pub fn metadata(&self) -> ActorMetadata {
        let mut m = self.metadata.clone();
        m.features = self.spec.features();
        m
    }

    pub fn own_addr(&self) -> Result<SocketAddr> {
        self.metadata.own_addr()
    }

    pub fn remote_addr(&self) -> Result<RemoteAddr> {
        self.metadata.remote_addr()
    }

    pub fn seed_addr(&self) -> Result<SocketAddr> {
        self.metadata.seed_addr()
    }

    pub fn spec(&self) -> T {
        self.spec.clone()
    }

    pub fn start(&self) -> Result<()> {
        self.spec.start()
    }
}

impl ActorMetadata {
    pub fn setup(mut self, actor_id: &str, features: Option<Vec<String>>) -> Self {
        self.actor_id = Some(actor_id.to_string());
        self.features = features;
        self
    }

    pub fn actor_id(&self) -> String {
        self.actor_id.clone().expect("ACTOR_ID")
    }

    pub fn own_addr(&self) -> Result<SocketAddr> {
        let socket_addr: SocketAddr = self.actor_host.parse::<SocketAddr>()?;
        Ok(socket_addr)
    }

    pub fn remote_addr(&self) -> Result<RemoteAddr> {
        let socket_addr: SocketAddr = self.own_addr()?;
        let remote_addr = RemoteAddr::new_from_id(socket_addr, &self.actor_id());
        Ok(remote_addr)
    }

    pub fn seed_addr(&self) -> Result<SocketAddr> {
        let socket_addr = self
            .actor_seed
            .clone()
            .expect("SEED REQUIRED")
            .parse::<SocketAddr>()?;
        Ok(socket_addr)
    }
}
