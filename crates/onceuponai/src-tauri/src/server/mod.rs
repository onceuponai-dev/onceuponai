use crate::MainArgs;
use actix::Addr;
use onceuponai_actors::abstractions::ActorMetadata;
use onceuponai_actors::actors::main_actor::{
    MainActor, MainActorAuthConfig, MainActorOidcConfig, MainActorSpec,
};
use onceuponai_actors::cluster::start_main_cluster;
use onceuponai_core::common::{
    env_or_some, env_or_some_or_fn, generate_token, random_base64, ResultExt,
};
use onceuponai_server::handlers::auth::generate_pat_token;
use onceuponai_server::session::generate_cookie_key;
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
    pub auth_token: String,
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

pub fn init(config: Option<Arc<Mutex<TauriAppConfig>>>, main_args: MainArgs) -> io::Result<()> {
    let metadata = ActorMetadata {
        actor_host: main_args.actor_host,
        name: "main_actor".to_string(),
        features: None,
        actor_id: None,
        actor_seed: None,
        sidecar_id: None,
    };

    let auth = if main_args.oidc {
        Some(MainActorAuthConfig {
            oidc: Some(MainActorOidcConfig {
                client_id: env_or_some("OIDC_CLIENT_ID", main_args.oidc_client_id),
                issuer_url: env_or_some("OIDC_ISSUER_URL", main_args.oidc_issuer_url),
                client_secret: env_or_some("OIDC_CLIENT_SECRET", main_args.oidc_client_secret),
                redirect_url: env_or_some("OIDC_REDIRECT_URL", main_args.oidc_redirect_url),
            }),
            _auth_token: None,
        })
    } else {
        None
    };

    let spec = MainActorSpec {
        server_host: main_args.host,
        server_port: main_args.port,
        log_level: Some(main_args.log_level),
        workers: Some(main_args.workers),
        invoke_timeout: Some(main_args.invoke_timeout),
        session_key: Some(env_or_some_or_fn(
            "SESSION_KEY",
            main_args.session_key,
            || random_base64(64),
        )),
        personal_access_token_secret: Some(env_or_some_or_fn(
            "TOKEN_SECRET",
            main_args.personal_access_token_secret,
            || random_base64(64),
        )),
        auth,
    };

    if let Some(conf) = config {
        let mut shared_config = conf.lock().map_io_err()?;

        actix_rt::System::new().block_on(async {
            let res = start_main_cluster(metadata, spec)
                .await
                .unwrap()
                .expect("MAIN ACTOR SPEC");

            let secret = res
                .0
                .personal_access_token_secret
                .clone()
                .expect("PERSONAL_ACCESS_TOKEN_SECRET");

            let auth_token = generate_token(50);
            let personal_token = generate_pat_token(&secret, "root", 30);
            shared_config.base_url = format!("http://localhost:{}", res.0.server_port);
            shared_config.auth_token = auth_token.clone();
            shared_config.personal_token = personal_token;
            shared_config.actor_seed = res.2.clone().actor_host;
            let host_split: Vec<&str> = res.2.actor_host.split(':').collect();
            shared_config.actor_base_host = host_split[0].to_string();
            shared_config.actor_next_port = host_split[1].parse().unwrap();
            drop(shared_config);

            onceuponai_server::serve::serve(res.0, res.1, auth_token).await
        })
    } else {
        let secret = spec
            .personal_access_token_secret
            .clone()
            .expect("PERSONAL_ACCESS_TOKEN_SECRET");

        let auth_token = generate_token(50);
        let personal_token = generate_pat_token(&secret, "root", 30);
        println!("PERSONAL TOKEN: {personal_token}");

        actix_rt::System::new().block_on(async {
            let res = start_main_cluster(metadata, spec)
                .await
                .unwrap()
                .expect("MAIN ACTOR SPEC");

            onceuponai_server::serve::serve(res.0, res.1, auth_token).await
        })
    }
}
