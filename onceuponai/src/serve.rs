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
use onceuponai_core::llm::e5::E5Model;
use onceuponai_core::llm::gemma::GemmaModel;
use onceuponai_core::llm::quantized::QuantizedModel;
use onceuponai_core::llm::rag::{init_lancedb, set_prompt_template};
use onceuponai_core::llm::LLMState;

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
async fn get_session(req: HttpRequest, session: actix_session::Session) -> HttpResponse {
    let user_id: Option<String> = session.get("EMAIL").unwrap();
    if let Some(user_id) = user_id {
        HttpResponse::Ok().body(format!("User ID: {}", user_id))
    } else {
        HttpResponse::Ok().body("No session found")
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn serve(
    host: &str,
    port: u16,
    log_level: Option<&String>,
    workers: Option<&usize>,
    use_quantized: &bool,
    model_repo: &str,
    model_file: &str,
    tokenizer_repo: Option<String>,
    lancedb_uri: &str,
    lancedb_table: &str,
    e5_model_repo: &str,
    prompt_template: &str,
    device: Option<String>,
) -> std::io::Result<()> {
    if let Some(v) = log_level {
        env_logger::init_from_env(env_logger::Env::new().default_filter_or(v));
    }

    let use_quantized = *use_quantized;

    set_prompt_template(prompt_template, !use_quantized).map_io_err()?;
    if !use_quantized {
        GemmaModel::lazy(
            None,
            None,
            device.clone(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .map_io_err()?;
    } else {
        QuantizedModel::lazy(
            Some(model_repo.to_string()),
            Some(model_file.to_string()),
            None,
            tokenizer_repo,
            device.clone(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .map_io_err()?;
    }

    E5Model::lazy(Some(e5_model_repo.to_string()), device).map_io_err()?;

    init_lancedb(lancedb_uri, lancedb_table)
        .await
        .map_io_err()?;

    let secret_key = get_secret_key().map_io_err()?;
    println!("Server running on http://{host}:{port}");
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
            )
            .app_data(web::Data::new(LLMState { use_quantized }));
        //.route("/", web::get().to(index))

        let mut llm_scope = web::scope("/llm");

        if !use_quantized {
            llm_scope = llm_scope.route("/chat", web::get().to(chat))
        } else {
            llm_scope = llm_scope.route("/chat", web::get().to(chat_quantized));
        }

        app = app.service(llm_scope);

        app.service(fs::Files::new("/", "../onceuponai-ui/dist/").show_files_listing())
    });

    if let Some(num_workers) = workers {
        if !num_workers.is_zero() {
            server = server.workers(*num_workers);
        }
    }

    server.bind((host, port))?.run().await
}
