mod handlers;
mod models;
mod session;
use actix::Addr;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use handlers::actors::{connected_actors, invoke};
use handlers::auth::generate_pat_token;
use handlers::oai::v1_chat_completions;
use onceuponai_actors::actors::main_actor::{MainActor, MainActorSpec};
use onceuponai_actors::cluster::start_main_cluster;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

async fn index() -> impl Responder {
    HttpResponse::Ok().json(json!({"hello": "world"}))
}

pub fn init(config: Arc<Mutex<TauriAppConfig>>) -> io::Result<()> {
    let tauri_state = web::Data::new(TauriAppState {
        config: config.clone(),
    });

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

        if let Some(v) = res.0.log_level.clone() {
            env_logger::init_from_env(env_logger::Env::new().default_filter_or(v));
        }

        let app_state = web::Data::new(AppState {
            spec: res.0,
            addr: res.1,
        });

        HttpServer::new(move || {
            let mut app = App::new()
                .wrap(Logger::default())
                .app_data(tauri_state.clone())
                .app_data(app_state.clone())
                .route("/health", web::get().to(health))
                .route("/api/hello", web::get().to(index));

            app = app.service(
                web::scope("/api")
                    // .guard(auth_guard.clone())
                    .route("/actors", web::get().to(connected_actors))
                    .route("/invoke/{kind}/{name}", web::post().to(invoke))
                    .route("/user", web::get().to(handlers::users::user))
                    .route(
                        "/user/personal-token",
                        web::post().to(handlers::auth::personal_token),
                    ),
            );

            app = app.service(
                web::scope("v1")
                    // .guard(auth_guard)
                    .route("/chat/completions", web::post().to(v1_chat_completions)),
            );

            app
        })
        .bind("0.0.0.0:8080")?
        .run()
        .await // Await the server future
    })
}
