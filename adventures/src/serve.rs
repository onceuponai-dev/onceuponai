use crate::auth::validate_jwt;
use crate::auth::{authstate_task, AuthState};
use crate::bot::BotIncommingMessage;
use crate::common::{OptionToResult, ResultExt};
use crate::llm::e5::E5Model;
use crate::llm::gemma::GemmaModel;
use crate::llm::quantized::QuantizedModel;
use crate::llm::rag::{build_prompt, find_context};
use crate::llm::rag::{init_lancedb, set_prompt_template};
use crate::llm::LLMState;
use crate::models::{EmbeddingsRequest, PromptRequest};
use actix_files as fs;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use num_traits::Zero;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

const INDEX_HTML: &str = include_str!("../public/index.html");

pub async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(INDEX_HTML)
}

pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

pub async fn chat(
    request: web::Query<PromptRequest>,
    llm_state: web::Data<LLMState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let context = find_context(request.prompt.to_string()).await.unwrap();
    let prompt = build_prompt(request.prompt.to_string(), context)
        .await
        .unwrap();

    crate::llm::gemma::chat(&prompt, request.sample_len, llm_state.eos_token).await
}

pub async fn chat_quantized(
    request: web::Query<PromptRequest>,
    llm_state: web::Data<LLMState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let context = find_context(request.prompt.to_string()).await.unwrap();
    let prompt = build_prompt(request.prompt.to_string(), context)
        .await
        .unwrap();

    crate::llm::quantized::chat(&prompt, request.sample_len, llm_state.eos_token).await
}

pub async fn embeddings(
    embeddings_request: web::Json<EmbeddingsRequest>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let embeddings_data = E5Model::embeddings(embeddings_request.input.clone()).await?;
    Ok(web::Json(embeddings_data))
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

pub async fn bot(
    request: web::Json<BotIncommingMessage>,
    req: HttpRequest,
    auth_state: web::Data<AuthState>,
) -> Result<impl Responder, Box<dyn Error>> {
    log::error!("BODY: {request:?}");
    let headers = req.headers();
    let token = headers
        .get("Authorization")
        .ok_or_err("AUTHORIZATION TOKEN HEADER")?
        .to_str()?
        .replace("Bearer ", "");

    let is_valid = validate_jwt(&token).await?;
    log::error!("IS VALID: {is_valid:?}");
    if is_valid {
        let access_token = auth_state.auth_token.read().await.access_token.to_string();
        crate::bot::bot_reply(&request.0, &access_token).await?;
    }

    Ok(HttpResponse::Ok())
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
    tokenizer_repo: &str,
    lancedb_uri: &str,
    lancedb_table: &str,
    e5_model_repo: &str,
    prompt_template: &str,
    device: &str,
) -> std::io::Result<()> {
    if let Some(v) = log_level {
        env_logger::init_from_env(env_logger::Env::new().default_filter_or(v));
    }

    let use_quantized = *use_quantized;
    let mut eos_token: u32 = 0;

    set_prompt_template(prompt_template, !use_quantized).map_io_err()?;
    if !use_quantized {
        let hf_token = std::env::var("HF_TOKEN").expect("HF_TOKEN");
        eos_token = GemmaModel::init(&hf_token, device).map_io_err()?;
    } else {
        eos_token =
            QuantizedModel::init(model_repo, model_file, tokenizer_repo, device).map_io_err()?;
    }

    E5Model::init(e5_model_repo, device).map_io_err()?;

    init_lancedb(lancedb_uri, lancedb_table)
        .await
        .map_io_err()?;

    //Q!: BACKGROUND TASK
    let authtoken = Arc::new(RwLock::new(
        crate::auth::refresh_authstate().await.map_io_err()?,
    ));
    let auth_token = Arc::clone(&authtoken);
    tokio::spawn(async move {
        authstate_task(auth_token).await;
    });

    println!("Server running on http://{host}:{port}");
    let mut server = HttpServer::new(move || {
        let mut app = App::new()
            .wrap(Logger::default())
            .route("/health", web::get().to(health))
            .route("/embeddings", web::post().to(embeddings))
            .route("/bot", web::post().to(bot))
            .app_data(web::Data::new(LLMState {
                eos_token,
                use_quantized,
            }))
            .app_data(web::Data::new(AuthState {
                auth_token: Arc::clone(&authtoken),
            }));
        //.route("/", web::get().to(index))

        if !use_quantized {
            app = app.route("/chat", web::get().to(chat))
        } else {
            app = app.route("/chat", web::get().to(chat_quantized));
        }

        app.service(fs::Files::new("/", "./public").show_files_listing())
    });

    if let Some(num_workers) = workers {
        if !num_workers.is_zero() {
            server = server.workers(*num_workers);
        }
    }

    server.bind((host, port))?.run().await
}
