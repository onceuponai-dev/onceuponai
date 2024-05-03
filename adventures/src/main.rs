//#[cfg(feature = "accelerate")]
//extern crate accelerate_src;
//#[cfg(feature = "mkl")]
//extern crate intel_mkl_src;

pub mod common;
use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use actix_web::{error, web, App, HttpResponse, HttpServer, Responder};
use adventures::{E5Model, GemmaModel, GemmaState, QuantizedModel};
use anyhow::Result;
use arrow_array::StringArray;
use async_stream::stream;
use bytes::Bytes;
use candle_core::{DType, Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::llama::MAX_SEQ_LEN;
use clap::{arg, Command};
use derive_more::{Display, Error};
use futures::TryStreamExt;
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use num_traits::Zero;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

const GEMMA_2B_REPO_ID: &str = "google/gemma-2b-it";
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
                    arg!(--quantized <QUANTIZED> "quantized")
                        .required(false)
                        .help("use quantized model")
                        .default_value("false")
                        .value_parser(clap::value_parser!(bool)),
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
                .get_one::<bool>("quantized")
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
    gemma_state: web::Data<GemmaState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let context = find_context(request.prompt.to_string()).await.unwrap();
    let prompt = build_prompt_quantized(request.prompt.to_string(), context)
        .await
        .unwrap();

    let mut model = QUANTIZED_MODEL.get().unwrap().lock().await;

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
    let column = batch.column(0);
    let str_column = column.as_any().downcast_ref::<StringArray>().unwrap();
    let v = str_column.value(0);

    Ok(v.to_string())
}

async fn build_prompt(prompt: String, context: String) -> Result<String> {
    let prompt = format!(
        r#"
    <start_of_turn>user
You are a seller in a fantasy store. Use the following pieces of context to answer the question at the end. If you don't know the answer, just say that you don't know, don't try to make up an answer. Context: {}.
Question: {}
<end_of_turn>
<start_of_turn>model
        "#,
        context, prompt
    );
    Ok(prompt)
}

async fn build_prompt_quantized(prompt: String, context: String) -> Result<String> {
    let prompt = format!(
        r#"
        [INST]You are a seller in a fantasy store. Use the following pieces of context to answer the question at the end. If you don't know the answer, just say that you don't know, don't try to make up an answer. Context: {}.
Question: {}[/INST]
        "#,
        context, prompt
    );
    Ok(prompt)
}

pub async fn embeddings(
    embeddings_request: web::Json<EmbeddingsRequest>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let model = E5_MODEL.get().unwrap().lock().await;
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
) -> std::io::Result<()> {
    if let Some(v) = log_level {
        env_logger::init_from_env(env_logger::Env::new().default_filter_or(v));
    }

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
            None => panic!("DUPA EOS"), //yield Err(Box::new(GemmaError::Eos))
        };

        let _ = GEMMA_MODEL.set(Arc::new(Mutex::new(model))).is_ok();
    } else {
        let model = QuantizedModel::load().unwrap();
        let _ = QUANTIZED_MODEL.set(Arc::new(Mutex::new(model)));
    }

    let e5_model = E5Model::load().unwrap();
    let _ = E5_MODEL.set(Arc::new(Mutex::new(e5_model))).is_ok();

    let uri = "/tmp/fantasy-lancedb";
    let db = connect(uri).execute().await.unwrap();

    let tbl = db.open_table("fantasy_vectors").execute().await.unwrap();

    let _ = LANCEDB_TABLE.set(Arc::new(Mutex::new(tbl))).is_ok();

    println!("Embedde-rs server running on http://{host}:{port}");
    let mut server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/health", web::get().to(health))
            .route("/embeddings", web::post().to(embeddings))
            .route("/chat", web::get().to(chat))
            .route("/chat-quantized", web::get().to(chat_quantized))
            //.route("/", web::get().to(index))
            .service(fs::Files::new("/", "./public").show_files_listing())
            .app_data(web::Data::new(GemmaState { eos_token }))
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

    let e5_model = E5Model::load().unwrap();
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

    println!("{row_count}");

    //let _ = arrow::util::pretty::print_batches(&batches);

    let batch = batches.last().unwrap();
    let column = batch.column(0);
    let str_column = column.as_any().downcast_ref::<StringArray>().unwrap();
    let v = str_column.value(0);
    println!("{v}");

    Ok(())
}

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
