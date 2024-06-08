//use crate::bot::BotIncommingMessage;
use crate::models::{EmbeddingsRequest, PromptRequest};
use actix_files as fs;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use num_traits::Zero;
use onceuponai_core::common::{OptionToResult, ResultExt};
use onceuponai_core::llm::e5::E5Model;
use onceuponai_core::llm::gemma::GemmaModel;
use onceuponai_core::llm::quantized::QuantizedModel;
use onceuponai_core::llm::rag::{build_prompt, find_context};
use onceuponai_core::llm::rag::{init_lancedb, set_prompt_template};
use onceuponai_core::llm::LLMState;
use std::error::Error;

const INDEX_HTML: &str = include_str!("../public/index.html");

pub async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(INDEX_HTML)
}

pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

pub async fn chat(request: web::Query<PromptRequest>) -> Result<impl Responder, Box<dyn Error>> {
    let context = find_context(request.prompt.to_string()).await.unwrap();
    let prompt = build_prompt(request.prompt.to_string(), context)
        .await
        .unwrap();

    crate::llm::gemma::chat(&prompt).await
}

pub async fn chat_quantized(
    request: web::Query<PromptRequest>,
) -> Result<impl Responder, Box<dyn Error>> {
    let context = find_context(request.prompt.to_string()).await.unwrap();
    let prompt = build_prompt(request.prompt.to_string(), context)
        .await
        .unwrap();

    crate::llm::quantized::chat(&prompt).await
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

    println!("Server running on http://{host}:{port}");
    let mut server = HttpServer::new(move || {
        let mut app = App::new()
            .wrap(Logger::default())
            .route("/health", web::get().to(health))
            .route("/embeddings", web::post().to(embeddings))
            .app_data(web::Data::new(LLMState { use_quantized }));
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
