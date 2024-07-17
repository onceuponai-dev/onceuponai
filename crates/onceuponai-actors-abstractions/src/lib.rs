use actix_telepathy::RemoteAddr;
use anyhow::Result;
use onceuponai_abstractions::EntityValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use uuid::Uuid;

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
    pub actor_host: String,
    pub actor_seed: Option<String>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorInvokeInput {
    pub task_id: Uuid,
    pub source: RemoteAddr,
    pub stream: bool,
    pub config: HashMap<String, EntityValue>,
    pub data: HashMap<String, Vec<EntityValue>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ActorInvokeOutput {
    Success(ActorInvokeResult),
    Finish(ActorInvokeFinish),
    Failure(ActorInvokeError),
}

pub trait ActorActions {
    fn actor_id(&self) -> &str;

    fn features(&self) -> Option<Vec<String>> {
        None
    }

    fn is_main(&self) -> bool {
        false
    }

    fn kind(&self) -> String;
    fn start(&self) -> Result<()>;
    fn invoke(&self, uuid: Uuid, request: ActorInvokeInput) -> Result<ActorInvokeOutput>;
    fn invoke_stream<F>(&self, uuid: Uuid, request: &ActorInvokeInput, callback: F) -> Result<()>
    where
        F: FnMut(ActorInvokeOutput);
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
    pub fn actor_id(&self) -> &str {
        self.spec.actor_id()
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
        let socket_addr: SocketAddr = self.metadata().actor_host.parse::<SocketAddr>()?;
        Ok(socket_addr)
    }

    pub fn remote_addr(&self) -> Result<RemoteAddr> {
        let socket_addr: SocketAddr = self.own_addr()?;
        let remote_addr = RemoteAddr::new_from_id(socket_addr, self.actor_id());
        Ok(remote_addr)
    }

    pub fn seed_addr(&self) -> Result<SocketAddr> {
        let socket_addr = self
            .metadata()
            .actor_seed
            .clone()
            .expect("SEED REQUIRED")
            .parse::<SocketAddr>()?;
        Ok(socket_addr)
    }

    pub fn spec(&self) -> T {
        self.spec.clone()
    }

    pub fn start(&self) -> Result<()> {
        self.spec.start()
    }

    pub fn invoke(&self, uuid: Uuid, request: ActorInvokeInput) -> Result<ActorInvokeOutput> {
        self.spec.invoke(uuid, request)
    }

    pub fn invoke_stream<F>(
        &self,
        uuid: Uuid,
        request: &ActorInvokeInput,
        callback: F,
    ) -> Result<()>
    where
        F: FnMut(ActorInvokeOutput),
    {
        self.spec.invoke_stream(uuid, request, callback)
    }
}
