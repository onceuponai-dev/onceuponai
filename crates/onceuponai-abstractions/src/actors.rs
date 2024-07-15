use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActorMetadata {
    pub name: String,
    pub features: Option<Vec<String>>,
    pub actor_host: String,
    pub actor_seed: Option<String>,
}

struct ActorConfig<T> {
    metadata: ActorMetadata,
    spec: T,
}

impl<T> ActorConfig<T> {
    fn metadata(&self) {}
}
