use crate::common::OptionToResult;
use crate::llm::e5::E5Model;
use anyhow::Result;
use futures::TryStreamExt;
use lancedb::arrow::arrow_array::cast::as_string_array;
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::Mutex;

static PROMPT_TEMPLATE: OnceCell<Arc<Mutex<String>>> = OnceCell::new();

static LANCEDB_TABLE: OnceCell<Arc<Mutex<lancedb::Table>>> = OnceCell::new();

pub async fn init_lancedb(lancedb_uri: &str, lancedb_table: &str) -> Result<()> {
    let db = connect(lancedb_uri).execute().await?;
    let tbl = db.open_table(lancedb_table).execute().await?;
    let _ = LANCEDB_TABLE.set(Arc::new(Mutex::new(tbl))).is_ok();
    Ok(())
}

pub fn set_prompt_template(prompt_template: &str, is_gemma: bool) -> Result<()> {
    let prompt_template = if is_gemma {
        format!(
            r#"
        <start_of_turn>user
        {}
        <end_of_turn>
        <start_of_turn>model
        "#,
            prompt_template
        )
    } else {
        format!(
            r#"
        [INST]{}[/INST]
        "#,
            prompt_template
        )
    };

    let _ = PROMPT_TEMPLATE
        .set(Arc::new(Mutex::new(prompt_template)))
        .is_ok();

    Ok(())
}

pub async fn find_context(prompt: String) -> Result<String> {
    let embeddings_data = E5Model::embeddings(vec![prompt]).await?;
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

pub async fn build_prompt(prompt: String, context: String) -> Result<String> {
    let prompt_template = PROMPT_TEMPLATE
        .get()
        .ok_or_err("PROMPT_TEMPLATE")?
        .lock()
        .await
        .to_string();

    let prompt = prompt_template
        .replace("{context}", &context)
        .replace("{question}", &prompt);
    println!("\x1b[93m{}\x1b[0m", prompt);
    Ok(prompt)
}

#[tokio::test]
async fn test_lancedb() -> Result<()> {
    let uri = "/tmp/fantasy-lancedb";
    let db = connect(uri).execute().await.unwrap();

    let e5_model = E5Model::load(crate::llm::e5::E5_MODEL_REPO, "cpu").unwrap();
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
