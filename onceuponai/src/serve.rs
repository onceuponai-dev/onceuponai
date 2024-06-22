use crate::actors::main_actor::{self, MainActor, MainActorConfig};
use crate::config::Config;
use crate::handlers::chat::{chat, chat_quantized};
use crate::handlers::embeddings::embeddings;
use crate::handlers::{self, health};
use actix_files as fs;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::middleware::Logger;
use actix_web::{cookie::Key, web, App, HttpRequest, HttpResponse, HttpServer};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use num_traits::Zero;
use onceuponai_core::common::ResultExt;

fn get_secret_key() -> Result<Key> {
    let key = Config::get().session_key.to_string();
    let k = general_purpose::STANDARD.decode(key)?;
    Ok(Key::from(&k))
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn vectorize(
    _log_level: Option<&String>,
    _lancedb_uri: &str,
    _lancedb_table: &str,
    _e5_model_repo: &str,
) -> std::io::Result<()> {
    todo!()
}

// Handler for getting a session value
async fn get_session(_req: HttpRequest, session: actix_session::Session) -> HttpResponse {
    let user_id: Option<String> = session.get("EMAIL").unwrap();
    if let Some(user_id) = user_id {
        HttpResponse::Ok().body(format!("User ID: {}", user_id))
    } else {
        HttpResponse::Ok().body("No session found")
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn serve(spec: MainActorConfig) -> std::io::Result<()> {
    if let Some(v) = spec.log_level {
        env_logger::init_from_env(env_logger::Env::new().default_filter_or(v));
    }

    let secret_key = get_secret_key().map_io_err()?;
    println!(
        "Server running on http://{}:{}",
        spec.server_host, spec.server_port
    );
    let mut server = HttpServer::new(move || {
        let mut app = App::new()
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .wrap(Logger::default())
            .route("/health", web::get().to(health))
            .route("/embeddings", web::post().to(embeddings))
            .route("/get", web::get().to(get_session))
            .route("/auth", web::get().to(handlers::auth::auth))
            .route(
                "/auth-callback",
                web::get().to(handlers::auth::auth_callback),
            );

        let mut llm_scope = web::scope("/llm");
        llm_scope = llm_scope.route("/chat", web::get().to(chat));

        app = app.service(llm_scope);

        app.service(fs::Files::new("/", "../onceuponai-ui/dist/").show_files_listing())
    });

    if let Some(num_workers) = spec.workers {
        if !num_workers.is_zero() {
            server = server.workers(num_workers);
        }
    }

    server
        .bind((spec.server_host, spec.server_port))?
        .run()
        .await
}
