use crate::abstractions::{ActorActions, ActorInvokeRequest, ActorInvokeResponse};
use anyhow::Result;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

pub trait CustomActor: Send + Sync {
    fn start(&self);
    fn invoke(&self, uuid: Uuid, request: &ActorInvokeRequest) -> Result<ActorInvokeResponse>;
}

#[derive(Deserialize, Debug, Clone)]
pub struct CustomActorSpec {
    pub name: String,
}

impl ActorActions for CustomActorSpec {
    fn kind(&self) -> String {
        "custom".to_string()
    }

    fn start(&self) -> Result<()> {
        let registry = CUSTOM_ACTOR_REGISTRY.get_or_init(CustomActorRegistry::new);
        let custom_actor = registry.create(&self.name).expect("Custom actor not found");
        custom_actor.start();
        Ok(())
    }

    fn invoke(&self, uuid: Uuid, request: &ActorInvokeRequest) -> Result<ActorInvokeResponse> {
        let registry = CUSTOM_ACTOR_REGISTRY.get_or_init(CustomActorRegistry::new);
        let custom_actor = registry.create(&self.name).expect("Custom actor not found");
        custom_actor.invoke(uuid, request)
    }
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
