//#[cfg(feature = "accelerate")]
//extern crate accelerate_src;
//#[cfg(feature = "mkl")]
//extern crate intel_mkl_src;

pub mod common;
use crate::common::{OptionToResult, ResultExt};
use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use actix_web::{error, web, App, HttpResponse, HttpServer, Responder};
use adventures::{
    E5Model, GemmaModel, GemmaState, QuantizedModel, E5_MODEL_REPO, GEMMA_2B_REPO_ID,
};
use anyhow::Result;
use async_stream::stream;
use bytes::Bytes;
use candle_core::{DType, Tensor};
use candle_transformers::generation::LogitsProcessor;
use clap::{arg, Command};
use derive_more::{Display, Error};
use futures::TryStreamExt;
use lancedb::arrow::arrow_array::cast::as_string_array;
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use num_traits::Zero;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

//const GEMMA_7B_REPO_ID: &str = "google/gemma-7b";

const INDEX_HTML: &str = include_str!("../public/index.html");

fn cli() -> Command {
    Command::new("gemma")
        .about("gemma")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("serve")
                .about("serve gemma")
                .args(vec![
                    arg!(--host <HOST> "host")
                        .required(false)
                        .help("host")
                        .default_value("0.0.0.0"),
                    arg!(--port <PORT> "port")
                        .required(false)
                        .help("port")
                        .default_value("8080")
                        .value_parser(clap::value_parser!(u16)),
                    arg!(--loglevel <LOGLEVEL>)
                        .required(false)
                        .help("log level")
                        .default_value("error"),
                    arg!(--workers <WORKERS>)
                        .required(false)
                        .help("number of workers")
                        .default_value("0")
                        .value_parser(clap::value_parser!(usize)),
                    arg!(--hftoken <HFTOKEN>)
                        .required(false)
                        .help("number of workers")
                        .default_value(""),
                    arg!(--use-quantized <USEQUANTIZED> "use_quantized")
                        .required(false)
                        .help("use quantized model")
                        .default_value("true")
                        .value_parser(clap::value_parser!(bool)),
                    arg!(--model-repo <MODELREPO> "model_repo")
                        .required(false)
                        .help("hf model repo")
                        .default_value("TheBloke/Mistral-7B-Instruct-v0.2-GGUF"),
                    arg!(--model-file <MODELFILE> "model_file")
                        .required(false)
                        .help("model file")
                        .default_value("mistral-7b-instruct-v0.2.Q4_K_S.gguf"),
                    arg!(--tokenizer-repo <TOKENIZERREPO> "tokenizer_repo")
                        .required(false)
                        .help("tokenizer repo")
                        .default_value("mistralai/Mistral-7B-Instruct-v0.2"),
                    arg!(--lancedb-uri <LANCEDBURI> "lancedb_uri")
                        .required(false)
                        .help("lancedb uri")
                        .default_value("/tmp/fantasy-lancedb"),
                    arg!(--lancedb-table <LANCEDBTABLE> "lancedb_table")
                        .required(false)
                        .help("lancedb table")
                        .default_value("fantasy_vectors"),
                    arg!(--e5-model-repo <E5MODELREPO> "e5_model_repo")
                        .required(false)
                        .help("hf e5 model repo")
                        .default_value("intfloat/e5-small-v2"),
                    arg!(--prompt-template <PROMPTTEMPLACE> "prompt_template")
                        .required(false)
                        .help("prompt template")
                        .default_value("You are a seller in a fantasy store. Use the following pieces of context to answer the question at the end. If you don't know the answer, just say that you don't know, don't try to make up an answer. Context: {context}.
Question: {question}"),
                ])
                .arg_required_else_help(true),
        )
}

pub(crate) async fn commands() -> Result<()> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("serve", sub_sub_matches)) => {
            let host = sub_sub_matches.get_one::<String>("host").expect("required");
            let port = sub_sub_matches.get_one::<u16>("port").expect("required");
            let log_level = sub_sub_matches.get_one::<String>("loglevel");
            let workers = sub_sub_matches.get_one::<usize>("workers");
            let use_quantized = sub_sub_matches
                .get_one::<bool>("use_quantized")
                .expect("required");

            let model_repo = sub_sub_matches
                .get_one::<String>("model_repo")
                .expect("required");

            let model_file = sub_sub_matches
                .get_one::<String>("model_file")
                .expect("required");

            let tokenizer_repo = sub_sub_matches
                .get_one::<String>("tokenizer_repo")
                .expect("required");

            let lancedb_uri = sub_sub_matches
                .get_one::<String>("lancedb_uri")
                .expect("required");

            let lancedb_table = sub_sub_matches
                .get_one::<String>("lancedb_table")
                .expect("required");

            let e5_model_repo = sub_sub_matches
                .get_one::<String>("e5_model_repo")
                .expect("required");

            let prompt_template = sub_sub_matches
                .get_one::<String>("prompt_template")
                .expect("required");

            // let hftoken = sub_sub_matches
            //     .get_one::<String>("hftoken")
            //     .expect("required")
            //     .clone();

            let hftoken = env::var("HF_TOKEN")?;

            serve(
                host,
                *port,
                log_level,
                workers,
                Some(hftoken),
                use_quantized,
                model_repo,
                model_file,
                tokenizer_repo,
                lancedb_uri,
                lancedb_table,
                e5_model_repo,
                prompt_template,
            )
            .await?
        }
        _ => unreachable!(),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    commands().await
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct PromptRequest {
    prompt: String,
    sample_len: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EmbeddingsRequest {
    input: Vec<String>,
}

#[derive(Debug, Display, Error)]
#[display(fmt = "request error: {name}")]
pub struct LLMError {
    name: &'static String,
    status_code: u16,
}

impl error::ResponseError for LLMError {
    fn status_code(&self) -> StatusCode {
        let status_code: StatusCode =
            StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        status_code
    }
}

pub async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(INDEX_HTML)
}

pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

static GEMMA_MODEL: OnceCell<Arc<Mutex<GemmaModel>>> = OnceCell::new();

static QUANTIZED_MODEL: OnceCell<Arc<Mutex<QuantizedModel>>> = OnceCell::new();

static E5_MODEL: OnceCell<Arc<Mutex<E5Model>>> = OnceCell::new();

static PROMPT_TEMPLATE: OnceCell<Arc<Mutex<String>>> = OnceCell::new();

static LANCEDB_TABLE: OnceCell<Arc<Mutex<lancedb::Table>>> = OnceCell::new();

pub async fn chat(
    request: web::Query<PromptRequest>,
    gemma_state: web::Data<GemmaState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let context = find_context(request.prompt.to_string()).await.unwrap();
    let prompt = build_prompt(request.prompt.to_string(), context)
        .await
        .unwrap();

    let mut model = GEMMA_MODEL.get().unwrap().lock().await;
    model.model.clear_kv_cache();

    let mut tokens = model
        .tokenizer
        .encode(prompt.clone(), true)
        .map_err(anyhow::Error::msg)?
        .get_ids()
        .to_vec();

    let stream_tasks = stream! {

        for index in 0..request.sample_len {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let start_pos = tokens.len().saturating_sub(context_size);
            let ctxt = &tokens[start_pos..];
            let input = Tensor::new(ctxt, &model.device)?.unsqueeze(0)?;
            let logits = model.model.forward(&input, start_pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?;
            let logits = if model.repeat_penalty == 1. {
                logits
            } else {
                let start_at = tokens.len().saturating_sub(model.repeat_last_n);
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    model.repeat_penalty,
                    &tokens[start_at..],
                )?
            };

            let next_token = model.logits_processor.sample(&logits)?;
            tokens.push(next_token);
            if next_token == gemma_state.eos_token {
                break;
            }

            tokio::task::yield_now().await;
            let tt = &model.tokenizer.decode(&[next_token], true).map_err(anyhow::Error::msg)?;
            println!("{tt}");
            let byte = Bytes::from(tt.clone());
            yield Ok::<Bytes, Box<dyn Error>>(byte);
        }

    };

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(Box::pin(stream_tasks)))
}

pub async fn chat_quantized(
    request: web::Query<PromptRequest>,
) -> Result<impl Responder, Box<dyn Error>> {
    let context = find_context(request.prompt.to_string()).await.unwrap();
    let prompt = build_prompt(request.prompt.to_string(), context)
        .await
        .unwrap();

    let mut model = QUANTIZED_MODEL
        .get()
        .ok_or_err("QUANTIZED_MODEL")?
        .lock()
        .await;

    let mut prompt_tokens = model
        .tokenizer
        .encode(prompt.clone(), true)
        .map_err(anyhow::Error::msg)?
        .get_ids()
        .to_vec();

    let sample_len: usize = 1000;
    let seed: u64 = 299792458;
    let temperature: Option<f64> = Some(0.8);
    let top_p: Option<f64> = None;
    let repeat_penalty: f32 = 1.1;
    let repeat_last_n: usize = 64;

    let mut all_tokens = vec![];
    let mut logits_processor = LogitsProcessor::new(seed, temperature, top_p);

    //let device = &Device::Cpu;
    let input = Tensor::new(prompt_tokens.as_slice(), &model.device)?.unsqueeze(0)?;
    let logits = model.model.forward(&input, 0)?;
    let logits = logits.squeeze(0)?;
    let mut next_token = logits_processor.sample(&logits)?;

    all_tokens.push(next_token);
    let t = model
        .tokenizer
        .decode(&[next_token], true)
        .map_err(anyhow::Error::msg)?;

    print!("{t} ");

    let eos_token = "</s>";

    let eos_token = *model.tokenizer.get_vocab(true).get(eos_token).unwrap();

    let stream_tasks = stream! {
        let mut previous_text = String::new();
        for index in 0..request.sample_len {
            let input = Tensor::new(&[next_token], &model.device)?.unsqueeze(0)?;
            let logits = model.model.forward(&input, prompt_tokens.len() + index)?;
            let logits = logits.squeeze(0)?;
            let logits = if repeat_penalty == 1. {
                logits
            } else {
                let start_at = all_tokens.len().saturating_sub(repeat_last_n);
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    repeat_penalty,
                    &all_tokens[start_at..],
                )?
            };
            next_token = logits_processor.sample(&logits)?;
            all_tokens.push(next_token);

            tokio::task::yield_now().await;

            if next_token == eos_token {
                break;
            };

            let current_text = model
                .tokenizer
                .decode(&all_tokens, true)
                .map_err(anyhow::Error::msg)?;


            let text = current_text.split_at(previous_text.len()).1.to_string();
            previous_text = current_text;
            print!("{text}");

            let byte = Bytes::from(text);
            yield Ok::<Bytes, Box<dyn Error>>(byte);
        }

    };

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(Box::pin(stream_tasks)))
}

async fn find_context(prompt: String) -> Result<String> {
    let model = E5_MODEL.get().unwrap().lock().await;
    let embeddings_data = model.forward(vec![prompt])?;
    let emb = embeddings_data.last().unwrap().clone();

    let tbl = LANCEDB_TABLE.get().unwrap().lock().await;
    let batches = tbl
        .query()
        .nearest_to(emb)?
        .limit(2)
        .execute()
        .await
        .unwrap()
        .try_collect::<Vec<_>>()
        .await?;

    let batch = batches.last().unwrap();
    let column = batch.column_by_name("item").unwrap();
    let str_column = as_string_array(column);
    let v = str_column.value(0);

    Ok(v.to_string())
}

async fn build_prompt(prompt: String, context: String) -> Result<String> {
    let prompt_template = PROMPT_TEMPLATE
        .get()
        .ok_or_err("PROMPT_TEMPLATE")?
        .lock()
        .await
        .to_string();

    let prompt = prompt_template
        .replace("{context}", &context)
        .replace("{question}", &prompt);
    Ok(prompt)
}

pub async fn embeddings(
    embeddings_request: web::Json<EmbeddingsRequest>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let model = E5_MODEL.get().ok_or_err("QUANTIZED_MODEL")?.lock().await;
    let embeddings_data = model.forward(embeddings_request.input.clone())?;
    Ok(web::Json(embeddings_data))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn serve(
    host: &str,
    port: u16,
    log_level: Option<&String>,
    workers: Option<&usize>,
    hf_token: Option<String>,
    use_quantized: &bool,
    model_repo: &str,
    model_file: &str,
    tokenizer_repo: &str,
    lancedb_uri: &str,
    lancedb_table: &str,
    e5_model_repo: &str,
    prompt_template: &str,
) -> std::io::Result<()> {
    if let Some(v) = log_level {
        env_logger::init_from_env(env_logger::Env::new().default_filter_or(v));
    }

    let use_quantized = *use_quantized;
    let mut eos_token = 0;
    if !use_quantized {
        let model = GemmaModel::load(
            GEMMA_2B_REPO_ID,
            None,
            299792458,
            Some(0.8),
            None,
            1.1,
            64,
            hf_token,
        )
        .unwrap();

        eos_token = match model.tokenizer.get_vocab(true).get("<eos>").copied() {
            Some(token) => token,
            None => {
                return Err(anyhow::anyhow!("EOS token not found in vocabulary")).map_io_err()?
            }
        };

        let _ = GEMMA_MODEL.set(Arc::new(Mutex::new(model))).is_ok();

        let prompt_template = format!(
            r#"
        <start_of_turn>user
        {}
        <end_of_turn>
        <start_of_turn>model
        "#,
            prompt_template
        );

        let _ = PROMPT_TEMPLATE
            .set(Arc::new(Mutex::new(prompt_template)))
            .is_ok();
    } else {
        let model = QuantizedModel::load(model_repo, model_file, tokenizer_repo).map_io_err()?;
        let _ = QUANTIZED_MODEL.set(Arc::new(Mutex::new(model)));

        let prompt_template = format!(
            r#"
        <start_of_turn>user
        [INST]{}[/INST]
        <end_of_turn>
        <start_of_turn>model
        "#,
            prompt_template
        );

        let _ = PROMPT_TEMPLATE
            .set(Arc::new(Mutex::new(prompt_template)))
            .is_ok();
    }

    let e5_model = E5Model::load(e5_model_repo).unwrap();
    let _ = E5_MODEL.set(Arc::new(Mutex::new(e5_model))).is_ok();

    let db = connect(lancedb_uri).execute().await.map_io_err()?;

    let tbl = db.open_table(lancedb_table).execute().await.map_io_err()?;

    let _ = LANCEDB_TABLE.set(Arc::new(Mutex::new(tbl))).is_ok();

    println!("Server running on http://{host}:{port}");
    let mut server = HttpServer::new(move || {
        let mut app = App::new()
            .wrap(Logger::default())
            .route("/health", web::get().to(health))
            .route("/embeddings", web::post().to(embeddings))
            //.route("/", web::get().to(index))
            .service(fs::Files::new("/", "./public").show_files_listing());

        if !use_quantized {
            app = app
                .route("/chat", web::get().to(chat))
                .app_data(web::Data::new(GemmaState { eos_token }));
        } else {
            app = app.route("/chat", web::get().to(chat_quantized));
        }

        app
    });

    if let Some(num_workers) = workers {
        if !num_workers.is_zero() {
            server = server.workers(*num_workers);
        }
    }

    server.bind((host, port))?.run().await
}

#[tokio::test]
async fn test_lancedb() -> Result<()> {
    let uri = "/tmp/fantasy-lancedb";
    let db = connect(uri).execute().await.unwrap();

    let e5_model = E5Model::load(E5_MODEL_REPO).unwrap();
    let ii = e5_model.forward(vec!["Adventure with a dragon".to_string()])?;
    let iii = ii.last().unwrap().clone();

    let tbl = db.open_table("fantasy_vectors").execute().await.unwrap();
    let batches = tbl
        .query()
        .nearest_to(iii)?
        .limit(2)
        .execute()
        .await
        .unwrap()
        .try_collect::<Vec<_>>()
        .await?;

    let row_count = batches.iter().map(|batch| batch.num_rows()).sum::<usize>();

    println!("ROW_COUNT: {row_count}");

    //let _ = arrow::util::pretty::print_batches(&batches);

    let batch = batches.last().unwrap();

    println!("BATCH {batch:?}");
    let column = batch.column_by_name("item").unwrap();

    println!("COLUMN {column:?}");
    //let column = batch.column(0);
    let str_column = as_string_array(column);
    let v = str_column.value(0);
    println!("{v:?}");

    Ok(())
}

/*
#[tokio::test]
async fn test_quantized() -> Result<()> {
    use candle_transformers::generation::LogitsProcessor;
    use candle_transformers::models::quantized_llama as model;

    let mut model = QuantizedModel::load()?;
    println!("TEST");

    //let prompt_str = "import socket\n\ndef ping_exponential_backoff(host: str):";
    let prompt_str = "[INST] What is your favourite condiment? [/INST]";

    let tokens = model
        .tokenizer
        .encode(prompt_str, true)
        .map_err(anyhow::Error::msg)?;

    let prompt_tokens = [tokens.get_ids()].concat();
    let sample_len: usize = 1000;
    let seed: u64 = 299792458;
    let temperature: Option<f64> = Some(0.8);
    let top_p: Option<f64> = None;
    let repeat_penalty: f32 = 1.1;
    let repeat_last_n: usize = 64;

    let to_sample = sample_len.saturating_sub(1);
    let prompt_tokens = if prompt_tokens.len() + to_sample > model::MAX_SEQ_LEN - 10 {
        let to_remove = prompt_tokens.len() + to_sample + 10 - model::MAX_SEQ_LEN;
        prompt_tokens[prompt_tokens.len().saturating_sub(to_remove)..].to_vec()
    } else {
        prompt_tokens
    };
    let mut all_tokens = vec![];
    let mut logits_processor = LogitsProcessor::new(seed, temperature, top_p);

    //let device = &Device::Cpu;
    let input = Tensor::new(prompt_tokens.as_slice(), &model.device)?.unsqueeze(0)?;
    let logits = model.model.forward(&input, 0)?;
    let logits = logits.squeeze(0)?;
    let mut next_token = logits_processor.sample(&logits)?;

    all_tokens.push(next_token);
    let t = model
        .tokenizer
        .decode(&[next_token], true)
        .map_err(anyhow::Error::msg)?;

    print!("{t} ");

    let eos_token = "</s>";

    let eos_token = *model.tokenizer.get_vocab(true).get(eos_token).unwrap();

    for (_sampled, index) in (0..to_sample).enumerate() {
        let input = Tensor::new(&[next_token], &model.device)?.unsqueeze(0)?;
        let logits = model.model.forward(&input, prompt_tokens.len() + index)?;
        let logits = logits.squeeze(0)?;
        let logits = if repeat_penalty == 1. {
            logits
        } else {
            let start_at = all_tokens.len().saturating_sub(repeat_last_n);
            candle_transformers::utils::apply_repeat_penalty(
                &logits,
                repeat_penalty,
                &all_tokens[start_at..],
            )?
        };
        next_token = logits_processor.sample(&logits)?;
        all_tokens.push(next_token);

        let t = model
            .tokenizer
            .decode(&[next_token], true)
            .map_err(anyhow::Error::msg)?;

        print!("{t} ");

        if next_token == eos_token {
            break;
        };
    }

    Ok(())
}
*/
