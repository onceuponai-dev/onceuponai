use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    fn metadata(&self) -> ActorMetadata;
    fn kind(&self) -> String;
    fn start(&self);
    // fn invoke(&self, uuid: Uuid, request: ActorInvokeRequest) -> Result<ActorInvokeResponse>;
    fn is_main(&self) -> bool;
}

struct ActorConfig<T> {
    metadata: ActorMetadata,
    spec: T,
}

impl<T> ActorConfig<T> {
    fn metadata(&self) -> ActorMetadata {
        self.metadata.clone()
    }
}
