use crate::parse_device;
use actix_telepathy::RemoteAddr;
use anyhow::Result;
use async_trait::async_trait;
use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_llama as model;
use either::Either;
use model::ModelWeights;
use once_cell::sync::OnceCell;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors::abstractions::openai::ChatCompletionRequest;
use onceuponai_actors::abstractions::{
    ActorActions, ActorError, ActorInvokeData, ActorInvokeError, ActorInvokeFinish,
    ActorInvokeRequest, ActorInvokeResponse, ActorInvokeResult,
};
use onceuponai_core::common::{hf_hub_get, hf_hub_get_path, OptionToResult, ResultExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;
use uuid::Uuid;

static QUANTIZED_INSTANCE: OnceCell<Arc<Mutex<QuantizedModel>>> = OnceCell::new();

#[derive(Deserialize, Debug, Clone)]
pub struct QuantizedSpec {
    pub model_repo: Option<String>,
    pub model_file: Option<String>,
    pub model_revision: Option<String>,
    pub tokenizer_repo: Option<String>,
    pub device: Option<String>,
    pub seed: Option<u64>,
    pub repeat_last_n: Option<usize>,
    pub repeat_penalty: Option<f32>,
    pub temp: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<usize>,
    pub sample_len: Option<usize>,
    pub gqa: Option<usize>,
    pub force_dmmv: Option<bool>,
    pub eos_token: Option<String>,
    pub prompt_format: Option<PromptFormat>,
    pub hf_token: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub enum PromptFormat {
    Mistral,
    Zephyr,
    OpenChat,
}

#[async_trait]
impl ActorActions for QuantizedSpec {
    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["chat".to_string()])
    }

    fn kind(&self) -> String {
        "quantized".to_string()
    }

    async fn init(&self) -> Result<()> {
        QuantizedModel::init(self.clone())
    }

    async fn start(&self) -> Result<()> {
        QuantizedModel::lazy(self.clone())?;

        println!("SPEC: {:?}", self);

        Ok(())
    }

    async fn invoke(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        let mut model = QuantizedModel::lazy(self.clone())?
            .lock()
            .map_anyhow_err()?;
        let input: String = match request.data.clone() {
            ActorInvokeData::ChatCompletion(chat_completion_request) => {
                model.map_request(chat_completion_request)?
            }
            _ => {
                source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));

                return Ok(());
            }
        };

        let text = model.invoke(&input)?;
        let result = ActorInvokeResult {
            uuid,
            task_id: request.task_id,
            stream: request.stream,
            metadata: HashMap::new(),
            data: HashMap::from([(String::from("content"), vec![EntityValue::STRING(text)])]),
        };

        source.do_send(ActorInvokeResponse::Success(result));
        Ok(())
    }

    async fn invoke_stream(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        let mut model = QuantizedModel::lazy(self.clone())?
            .lock()
            .map_anyhow_err()?;
        let input: String = match request.data.clone() {
            ActorInvokeData::ChatCompletion(chat_completion_request) => {
                model.map_request(chat_completion_request)?
            }
            _ => {
                source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));

                return Ok(());
            }
        };

        let repeat_last_n = model.repeat_last_n;
        let repeat_penalty = model.repeat_penalty;

        let prep = model.prepare(&input)?;

        let prompt_tokens_len = prep.0;
        let mut all_tokens = prep.1;
        let mut logits_processor = prep.2;

        let sample_len = model.sample_len;
        let eos_token = model.eos_token;

        let mut previous_text = String::new();
        for index in 0..sample_len {
            if let Some(current_text) = model.loop_process(
                prompt_tokens_len,
                index,
                repeat_penalty,
                repeat_last_n,
                &mut all_tokens,
                &mut logits_processor,
                eos_token,
            )? {
                let text = current_text.split_at(previous_text.len()).1.to_string();
                previous_text = current_text;

                let result = ActorInvokeResult {
                    uuid,
                    task_id: request.task_id,
                    stream: request.stream,
                    metadata: HashMap::new(),
                    data: HashMap::from([(
                        String::from("content"),
                        vec![EntityValue::STRING(text)],
                    )]),
                };

                let response = ActorInvokeResponse::Success(result);
                source.do_send(response);
            } else {
                break;
            }
        }

        let result = ActorInvokeFinish {
            uuid,
            task_id: request.task_id,
            stream: request.stream,
        };

        let response = ActorInvokeResponse::Finish(result);
        source.do_send(response);

        Ok(())
    }
}

pub struct QuantizedModel {
    pub spec: QuantizedSpec,
    pub model: ModelWeights,
    pub tokenizer: Tokenizer,
    pub eos_token: u32,
    pub seed: u64,
    pub repeat_last_n: usize,
    pub repeat_penalty: f32,
    pub temp: f64,
    pub top_p: Option<f64>,
    pub top_k: Option<usize>,
    pub sample_len: usize,
    pub device: Device,
}

impl QuantizedModel {
    pub fn map_request(&self, input: ChatCompletionRequest) -> Result<String> {
        let input = match input.messages {
            Either::Left(mut left) => left
                .iter_mut()
                .map(|x| match x.content.deref() {
                    Either::Left(content) => match &self.spec.prompt_format {
                        Some(PromptFormat::Mistral) => match x.role.as_str() {
                            "user" => format!("<s>[INST] {} [/INST]", content),
                            "model" => format!("\"{}\"</s>", content),
                            _ => content.clone(),
                        },
                        Some(PromptFormat::Zephyr) => match x.role.as_str() {
                            "user" => format!("<|user|>\n{}\n</s>", content),
                            "model" => format!("<|assistant|>model\n{}\n</s>", content),
                            _ => content.clone(),
                        },
                        Some(PromptFormat::OpenChat) => match x.role.as_str() {
                            "user" => format!("GPT4 Correct User: {}<|end_of_turn|>", content),
                            "model" => {
                                format!("GPT4 Correct Assistant: {}<|end_of_turn|>", content)
                            }
                            _ => content.clone(),
                        },
                        None => content.clone(),
                    },
                    Either::Right(_) => unimplemented!(),
                })
                .collect::<Vec<_>>()
                .join(" "),
            Either::Right(_) => unimplemented!(),
        };
        Ok(input)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn invoke(&mut self, prompt: &str) -> Result<String> {
        let repeat_penalty: f32 = self.repeat_penalty;
        let repeat_last_n: usize = self.repeat_last_n;

        let prep = self.prepare(prompt)?;
        let prompt_tokens_len = prep.0;
        let mut all_tokens = prep.1;
        let mut logits_processor = prep.2;

        let mut previous_text = String::new();
        for index in 0..self.sample_len {
            if let Some(current_text) = self.loop_process(
                prompt_tokens_len,
                index,
                repeat_penalty,
                repeat_last_n,
                &mut all_tokens,
                &mut logits_processor,
                self.eos_token,
            )? {
                previous_text = current_text;
            } else {
                break;
            }
        }

        Ok(previous_text)
    }

    pub fn prepare(&mut self, prompt: &str) -> Result<(usize, Vec<u32>, LogitsProcessor)> {
        let prompt_tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(anyhow::Error::msg)?
            .get_ids()
            .to_vec();

        let seed: u64 = self.spec.seed.unwrap_or(299792458);
        let temperature = self.spec.temp.unwrap_or(0.8);

        let mut all_tokens = vec![];
        // let mut logits_processor = LogitsProcessor::new(seed, Some(temperature), top_p);
        let mut logits_processor = {
            let sampling = if temperature <= 0. {
                Sampling::ArgMax
            } else {
                match (self.spec.top_k, self.spec.top_p) {
                    (None, None) => Sampling::All { temperature },
                    (Some(k), None) => Sampling::TopK { k, temperature },
                    (None, Some(p)) => Sampling::TopP { p, temperature },
                    (Some(k), Some(p)) => Sampling::TopKThenTopP { k, p, temperature },
                }
            };
            LogitsProcessor::from_sampling(seed, sampling)
        };

        let input = Tensor::new(prompt_tokens.as_slice(), &self.device)?.unsqueeze(0)?;
        let logits = self.model.forward(&input, 0)?;
        let logits = logits.squeeze(0)?;
        let next_token = logits_processor.sample(&logits)?;

        all_tokens.push(next_token);
        let prompt_tokens_len = prompt_tokens.len();

        Ok((prompt_tokens_len, all_tokens, logits_processor))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn loop_process(
        &mut self,
        prompt_tokens_len: usize,
        index: usize,
        repeat_penalty: f32,
        repeat_last_n: usize,
        all_tokens: &mut Vec<u32>,
        logits_processor: &mut LogitsProcessor,
        eos_token: u32,
    ) -> Result<Option<String>> {
        let next_token = *all_tokens.last().expect("Wrong ALL_TOKENS");
        let input = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
        let logits = self.model.forward(&input, prompt_tokens_len + index)?;
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
        let next_token = logits_processor.sample(&logits)?;
        all_tokens.push(next_token);

        if next_token == eos_token {
            return Ok(None);
        };

        let current_text = self
            .tokenizer
            .decode(all_tokens, true)
            .map_err(anyhow::Error::msg)?;

        Ok(Some(current_text))
    }

    pub fn lazy<'a>(spec: QuantizedSpec) -> Result<&'a Arc<Mutex<QuantizedModel>>> {
        if QUANTIZED_INSTANCE.get().is_none() {
            let model = QuantizedModel::load(spec.clone())?;

            let _ = QUANTIZED_INSTANCE.set(Arc::new(Mutex::new(model))).is_ok();
        };

        Ok(QUANTIZED_INSTANCE.get().expect("QUANTIZED_INSTANCE"))
    }

    pub fn init(spec: QuantizedSpec) -> Result<()> {
        let model_repo = &spec.model_repo.expect("model_repo");
        let model_file = &spec.model_file.expect("model_file");

        let _model_path = if model_file.starts_with("file://") {
            std::path::PathBuf::from(model_file.replace("file://", ""))
        } else {
            hf_hub_get_path(
                model_repo,
                model_file,
                spec.hf_token.clone(),
                spec.model_revision,
            )?
        };

        let tokenizer_repo = spec.tokenizer_repo.unwrap_or(model_repo.to_string());

        let _tokenizer = if tokenizer_repo.starts_with("file://") {
            std::fs::read(tokenizer_repo.replace("file://", ""))?
        } else {
            hf_hub_get(&tokenizer_repo, "tokenizer.json", spec.hf_token, None)?
        };

        Ok(())
    }

    #[allow(unused)]
    #[allow(clippy::too_many_arguments)]
    pub fn load(spec: QuantizedSpec) -> Result<QuantizedModel> {
        let spec_clone = spec.clone();
        #[cfg(feature = "cuda")]
        candle_core::quantized::cuda::set_force_dmmv(spec.force_dmmv.unwrap_or(false));

        #[cfg(feature = "cuda")]
        candle_core::cuda::set_gemm_reduced_precision_f16(true);

        #[cfg(feature = "cuda")]
        candle_core::cuda::set_gemm_reduced_precision_bf16(true);

        let model_repo = spec.model_repo.expect("model_repo");
        let model_file = spec.model_file.expect("model_file");
        let model_path = if model_file.starts_with("file://") {
            std::path::PathBuf::from(model_file.replace("file://", ""))
        } else {
            hf_hub_get_path(
                &model_repo,
                &model_file,
                spec.hf_token.clone(),
                spec.model_revision,
            )?
        };

        let tokenizer_repo = spec.tokenizer_repo.unwrap_or(model_repo.to_string());

        let tokenizer = if tokenizer_repo.starts_with("file://") {
            std::fs::read(tokenizer_repo.replace("file://", ""))?
        } else {
            hf_hub_get(&tokenizer_repo, "tokenizer.json", spec.hf_token, None)?
        };

        let device = parse_device(spec.device)?;
        let tokenizer = Tokenizer::from_bytes(&tokenizer).map_anyhow_err()?;

        let mut file = std::fs::File::open(&model_path)?;
        let model = match model_path.extension().and_then(|ex| ex.to_str()) {
            Some("gguf") => {
                let model_content = gguf_file::Content::read(&mut file)?;
                ModelWeights::from_gguf(model_content, &mut file, &device)?
            }
            Some("ggml" | "bin") | Some(_) | None => {
                let model_content = ggml_file::Content::read(&mut file, &device)?;
                ModelWeights::from_ggml(model_content, spec.gqa.expect("GQA"))?
            }
        };

        let eos_token = match spec.eos_token {
            Some(eos) => eos,
            None => "</s>".to_string(),
        };

        let vocab = tokenizer.get_vocab(true).clone();
        let eos_token = *vocab.get(&eos_token).ok_or_err("EOS_TOKEN")?;

        let seed: u64 = spec.seed.unwrap_or(299792458);
        let temp = spec.temp.unwrap_or(0.8);
        let repeat_penalty: f32 = spec.repeat_penalty.unwrap_or(1.1);
        let repeat_last_n: usize = spec.repeat_last_n.unwrap_or(64);
        let sample_len = spec.sample_len.unwrap_or(1000);

        Ok(QuantizedModel {
            spec: spec_clone,
            model,
            tokenizer,
            device,
            eos_token,
            seed,
            temp,
            repeat_last_n,
            repeat_penalty,
            top_k: spec.top_k,
            top_p: spec.top_p,
            sample_len,
        })
    }
}

/*
#[tokio::test]
async fn test_bielik() -> Result<()> {
    let mut bielik = QuantizedModel::load(
        "speakleash/Bielik-7B-Instruct-v0.1-GGUF",
        "bielik-7b-instruct-v0.1.Q4_K_S.gguf",
        None,
        Some("speakleash/Bielik-7B-Instruct-v0.1".to_string()),
        Some("cuda".to_string()),
        None,
        None,
        None,
    )?;

    let eos_token = "</s>";
    let vocab = bielik.tokenizer.get_vocab(true).clone();
    let eos_token = *vocab.get(eos_token).ok_or_err("EOS_TOKEN")?;

    let resp = bielik.invoke(
        "Jak ugotować żurek ?",
        500,
        eos_token,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    println!("RESPONSE: {resp}");
    Ok(())
}

#[tokio::test]
async fn test_phi3() -> Result<()> {
    let mut phi3 = QuantizedModel::load(
        "microsoft/Phi-3-mini-4k-instruct-gguf",
        "Phi-3-mini-4k-instruct-q4.gguf",
        Some("5eef2ce24766d31909c0b269fe90c817a8f263fb".to_string()),
        Some("microsoft/Phi-3-mini-4k-instruct".to_string()),
        Some("cuda".to_string()),
        None,
        None,
        None,
    )?;

    let resp = phi3.invoke(
        "Write loop in python",
        500,
        200,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    println!("RESPONSE: {resp}");
    Ok(())
}
    */
