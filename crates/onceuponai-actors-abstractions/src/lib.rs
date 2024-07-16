use actix_telepathy::RemoteAddr;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorMetadata {
    pub name: String,
    pub features: Option<Vec<String>>,
    pub actor_host: String,
    pub actor_seed: Option<String>,
}

pub trait ActorActions {
    fn actor_id(&self) -> &str;
    fn is_main(&self) -> bool;
    fn kind(&self) -> String;
    fn metadata(&self) -> ActorMetadata;
    fn own_addr(&self) -> Result<SocketAddr>;
    fn remote_addr(&self) -> Result<RemoteAddr>;
    fn seed_addr(&self) -> Result<SocketAddr>;
    fn start(&self) -> Result<()>;
    // fn invoke(&self, uuid: Uuid, request: ActorInvokeRequest) -> Result<ActorInvokeResponse>;
}

struct ActorConfig<T> {
    metadata: ActorMetadata,
    spec: T,
}

impl<T> ActorConfig<T> {
    fn actor_id(&self) -> &str {
        todo!()
    }

    fn metadata(&self) -> ActorMetadata {
        self.metadata.clone()
    }

    fn own_addr(&self) -> Result<SocketAddr> {
        let socket_addr: SocketAddr = self.metadata().actor_host.parse::<SocketAddr>()?;
        Ok(socket_addr)
    }

    fn remote_addr(&self) -> Result<RemoteAddr> {
        let socket_addr: SocketAddr = self.own_addr()?;
        let remote_addr = RemoteAddr::new_from_id(socket_addr, self.actor_id());
        Ok(remote_addr)
    }

    fn seed_addr(&self) -> Result<SocketAddr> {
        let socket_addr = self
            .metadata()
            .actor_seed
            .clone()
            .expect("SEED REQUIRED")
            .parse::<SocketAddr>()?;
        Ok(socket_addr)
    }
}
