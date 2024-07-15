use super::{ActorInvokeRequest, ActorInvokeResponse};
use anyhow::Result;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

pub trait CustomActor: Send + Sync {
    fn start(&self);
    fn invoke(&self, uuid: Uuid, request: ActorInvokeRequest) -> Result<ActorInvokeResponse>;
}

#[derive(Deserialize, Debug, Clone)]
pub struct CustomActorSpec {
    pub name: String,
}

type CustomActorFactory = fn() -> Box<dyn CustomActor>;

pub struct CustomActorRegistry {
    registry: RwLock<HashMap<String, CustomActorFactory>>,
}

impl CustomActorRegistry {
    pub fn new() -> Self {
        CustomActorRegistry {
            registry: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, name: &str, factory: CustomActorFactory) {
        self.registry
            .write()
            .unwrap()
            .insert(name.to_string(), factory);
    }

    pub fn create(&self, name: &str) -> Option<Box<dyn CustomActor>> {
        self.registry
            .read()
            .unwrap()
            .get(name)
            .map(|&factory| factory())
    }
}

pub static CUSTOM_ACTOR_REGISTRY: OnceCell<CustomActorRegistry> = OnceCell::new();
