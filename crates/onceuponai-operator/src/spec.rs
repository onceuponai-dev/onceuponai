use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "onceuponai.dev", version = "v1", kind = "Main", namespaced)]
pub struct MainSpec {
    pub replicas: i32,
    pub image: String,
    pub actor_host: String,
    pub actor_seed: Option<String>,
    pub server_host: String,       //actix
    pub server_port: u16,          //actix
    pub log_level: Option<String>, //actix
    pub workers: Option<usize>,    //actix
    pub invoke_timeout: Option<u64>,
    pub session_key: Option<String>,
    pub personal_access_token_secret: Option<String>,
    pub auth_token: Option<String>,
    pub oidc_issuer_url: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_redirect_url: Option<String>,
    // pub auth: Option<MainActorAuthConfig>,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "onceuponai.dev", version = "v1", kind = "Custom", namespaced)]
pub struct CustomSpec {
    pub replicas: i32,
    pub image: String,
    pub actor_host: String,
    pub actor_seed: Option<String>,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "onceuponai.dev", version = "v1", kind = "Gemma", namespaced)]
pub struct GemmaSpec {
    pub replicas: i32,
    pub image: String,
    pub actor_host: String,
    pub actor_seed: Option<String>,
    pub base_repo_id: Option<String>,
    pub tokenizer_repo: Option<String>,
    pub device: Option<String>,
    pub seed: Option<u64>,
    pub repeat_last_n: Option<usize>,
    pub repeat_penalty: Option<f32>,
    pub temp: Option<f64>,
    pub top_p: Option<f64>,
    pub hf_token: Option<String>,
    pub use_flash_attn: Option<bool>,
    pub sample_len: Option<usize>,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "onceuponai.dev",
    version = "v1",
    kind = "Quantized",
    namespaced
)]
pub struct QuantizedSpec {
    pub replicas: i32,
    pub image: String,
    pub actor_host: String,
    pub actor_seed: Option<String>,
    pub model_repo: Option<String>,
    pub model_file: Option<String>,
    pub model_revision: Option<String>,
    pub tokenizer_repo: Option<String>,
    pub device: Option<String>,
    pub seed: Option<u64>,
    pub repeat_last_n: Option<usize>,
    pub repeat_penalty: Option<f32>,
    pub temp: Option<f64>,
    pub top_p: Option<f64>,
    pub sample_len: Option<usize>,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "onceuponai.dev", version = "v1", kind = "E5", namespaced)]
pub struct E5Spec {
    pub replicas: i32,
    pub image: String,
    pub actor_host: String,
    pub actor_seed: Option<String>,
    pub model_repo: Option<String>,
    pub device: Option<String>,
}
