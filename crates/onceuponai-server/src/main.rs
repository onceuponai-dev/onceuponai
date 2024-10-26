use clap::Parser;
use onceuponai_actors::abstractions::ActorMetadata;
use onceuponai_actors::actors::main_actor::{
    MainActorAuthConfig, MainActorOidcConfig, MainActorSpec,
};
use onceuponai_actors::cluster::start_main_cluster;
use onceuponai_core::common::{
    env_or_some, env_or_some_or_fn, generate_token, random_base64, ResultExt,
};
use onceuponai_server::handlers::auth::generate_pat_token;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about)]
pub struct MainArgs {
    #[clap(long, default_value = "127.0.0.1:1992")]
    actor_host: String,
    #[clap(long, default_value = "0.0.0.0")]
    host: String,
    #[clap(long, default_value = "8080")]
    port: u16,
    #[clap(long, default_value = "info")]
    log_level: String,
    #[clap(long, default_value = "0")]
    workers: usize,
    #[clap(long, default_value = "60")]
    invoke_timeout: u64,
    #[clap(long)]
    session_key: Option<String>,
    #[clap(long)]
    personal_access_token_secret: Option<String>,
    #[clap(long, default_value_t = false)]
    oidc: bool,
    #[clap(long)]
    oidc_issuer_url: Option<String>,
    #[clap(long)]
    oidc_client_id: Option<String>,
    #[clap(long)]
    oidc_client_secret: Option<String>,
    #[clap(long)]
    oidc_redirect_url: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let main_args = MainArgs::parse();
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

    let secret = spec
        .personal_access_token_secret
        .clone()
        .expect("PERSONAL_ACCESS_TOKEN_SECRET");

    let auth_token = generate_token(50);
    let personal_token = generate_pat_token(&secret, "root", 30);
    println!("PERSONAL TOKEN: {personal_token}");

    let res = start_main_cluster(metadata, spec)
        .await
        .map_io_err()?
        .expect("MAIN ACTOR SPEC");

    onceuponai_server::serve::serve(res.0, res.1, auth_token).await
}
