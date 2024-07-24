use actix::Addr;
use onceuponai_actors::actors::main_actor::{MainActor, MainActorSpec};
use onceuponai_actors::cluster::start_main_cluster;
use onceuponai_server::handlers::auth::generate_pat_token;
use serde::{Deserialize, Serialize};
use std::{
    io,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub struct TauriAppState {
    pub config: Arc<Mutex<TauriAppConfig>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TauriAppConfig {
    pub personal_token: String,
    pub base_url: String,
    pub actor_seed: String,
    pub actor_base_host: String,
    pub actor_next_port: u16,
}

pub struct AppState {
    pub addr: Addr<MainActor>,
    pub spec: MainActorSpec,
}

pub fn init(config: Arc<Mutex<TauriAppConfig>>) -> io::Result<()> {
    let mut shared_config = config.lock().unwrap();

    actix_rt::System::new().block_on(async {
        let file = String::from("/home/jovyan/rust-src/onceuponai/examples/main.yaml");
        let res = start_main_cluster(&file)
            .await
            .unwrap()
            .expect("MAIN ACTOR SPEC");

        let secret = res
            .0
            .personal_access_token_secret
            .clone()
            .expect("PERSONAL_ACCESS_TOKEN_SECRET");

        let personal_token = generate_pat_token(&secret, "root", 30);
        shared_config.base_url = format!("http://localhost:{}", res.0.server_port);
        shared_config.personal_token = personal_token;
        shared_config.actor_seed = res.2.clone().actor_host;
        let host_split: Vec<&str> = res.2.actor_host.split(':').collect();
        shared_config.actor_base_host = host_split[0].to_string();
        shared_config.actor_next_port = host_split[1].parse().unwrap();

        drop(shared_config);
        onceuponai_server::serve::serve(res.0, res.1).await
    })
}
