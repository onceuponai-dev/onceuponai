use crate::parse_device;
use actix_telepathy::RemoteAddr;
use anyhow::Result;
use async_trait::async_trait;
use log::{info, warn};
use mistralrs::{
    get_model_dtype, get_tgt_non_granular_index, initialize_logging, paged_attn_supported,
    DefaultSchedulerMethod, Device, DeviceLayerMapMetadata, DeviceMapMetadata, IsqType, Loader,
    LoaderBuilder, MemoryGpuConfig, MistralRs, MistralRsBuilder, ModelSelected,
    PagedAttentionConfig, SchedulerConfig, TokenSource,
};
use once_cell::sync::OnceCell;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors::abstractions::{
    ActorActions, ActorError, ActorInvokeError, ActorInvokeFinish, ActorInvokeRequest,
    ActorInvokeResponse, ActorInvokeResult,
};
use onceuponai_core::common::{hf_hub_get, hf_hub_get_path, OptionToResult, ResultExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use uuid::Uuid;

static QUANTIZED_INSTANCE: OnceCell<Arc<MistralrsModel>> = OnceCell::new();

#[derive(Deserialize, Debug, Clone)]
pub struct MistralrsSpec {
    pub seed: Option<u64>,
    pub truncate_sequence: Option<bool>,
    pub max_seqs: Option<usize>,
    pub no_kv_cache: Option<bool>,
    pub chat_template: Option<String>,
    pub prefix_cache_n: Option<usize>,
    pub num_device_layers: Option<String>,
    pub in_situ_quant: Option<String>,
    pub paged_attn_gpu_mem: Option<usize>,
    pub paged_attn_gpu_mem_usage: Option<f32>,
    pub paged_ctxt_len: Option<usize>,
    pub paged_attn_block_size: Option<usize>,
    pub no_paged_attn: Option<bool>,
    pub throughput_log: Option<bool>,
    pub prompt_batchsize: Option<usize>,
    pub hf_token: Option<String>,
    pub device: Option<String>,
    pub model_selected: Option<String>,
    pub model_repo: Option<String>,
    pub model_file: Option<String>,
    pub model_revision: Option<String>,
    pub model_architecture: Option<String>, //plain
    pub model_dtype: Option<String>,        //plain
    pub tokenizer_repo: Option<String>,
    pub topology: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub enum PromptFormat {
    Mistral,
    Zephyr,
    OpenChat,
}

#[async_trait]
impl ActorActions for MistralrsSpec {
    fn features(&self) -> Option<Vec<String>> {
        Some(vec!["chat".to_string()])
    }

    fn kind(&self) -> String {
        "mistralrs".to_string()
    }

    fn init(&self) -> Result<()> {
        MistralrsModel::init(self.clone())
    }

    fn start(&self) -> Result<()> {
        MistralrsModel::lazy(self.clone())?;

        println!("SPEC: {:?}", self);

        Ok(())
    }

    async fn invoke(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
    ) -> Result<ActorInvokeResponse> {
        let input = request.data.get("message");

        if input.is_none() {
            return Ok(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));
        }

        let input: Vec<String> = input
            .expect("MESSAGE")
            .iter()
            .map(|x| match x {
                EntityValue::MESSAGE { role: _, content } => content.clone(),
                _ => todo!(),
            })
            .collect();

        let mut model = MistralrsModel::lazy(self.clone())?
            .lock()
            .map_anyhow_err()?;

        let results = input
            .iter()
            .map(|prompt| model.invoke(prompt))
            .collect::<Result<Vec<String>, _>>()?;

        let results = results
            .iter()
            .map(|r| EntityValue::STRING(r.clone()))
            .collect::<Vec<EntityValue>>();

        let result = ActorInvokeResult {
            uuid,
            task_id: request.task_id,
            stream: request.stream,
            metadata: HashMap::new(),
            data: HashMap::from([(String::from("content"), results)]),
        };

        Ok(ActorInvokeResponse::Success(result))
    }

    async fn invoke_stream(
        &self,
        uuid: Uuid,
        request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        let input = request.data.get("message");
        if input.is_none() {
            source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));

            return Ok(());
        }

        let input = input
            .expect("MESSAGE")
            .iter()
            .map(|x| match x {
                EntityValue::MESSAGE { role, content } => match &self.prompt_format {
                    Some(PromptFormat::Mistral) => match role.as_str() {
                        "user" => format!("<s>[INST] {} [/INST]", content),
                        "model" => format!("\"{}\"</s>", content),
                        _ => content.clone(),
                    },
                    Some(PromptFormat::Zephyr) => match role.as_str() {
                        "user" => format!("<|user|>\n{}\n</s>", content),
                        "model" => format!("<|assistant|>model\n{}\n</s>", content),
                        _ => content.clone(),
                    },
                    Some(PromptFormat::OpenChat) => match role.as_str() {
                        "user" => format!("GPT4 Correct User: {}<|end_of_turn|>", content),
                        "model" => format!("GPT4 Correct Assistant: {}<|end_of_turn|>", content),
                        _ => content.clone(),
                    },
                    None => content.clone(),
                },
                _ => todo!(),
            })
            .collect::<Vec<_>>()
            .join(" ");

        let mut model = MistralrsModel::lazy(self.clone())?
            .lock()
            .map_anyhow_err()?;
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

pub struct MistralrsModel {
    pub spec: MistralrsSpec,
    pub mistralrs: Arc<MistralRs>,
    pub device: Device,
}

impl MistralrsModel {
    #[allow(clippy::too_many_arguments)]
    pub fn invoke(&mut self, prompt: &str) -> Result<String> {
        let (tx, mut rx) = channel(10_000);
        let sender = self.mistralrs.get_sender().unwrap();

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

    pub async fn lazy<'a>(spec: MistralrsSpec) -> Result<&'a Arc<MistralrsModel>> {
        if QUANTIZED_INSTANCE.get().is_none() {
            let model = MistralrsModel::load(spec.clone()).await?;

            let _ = QUANTIZED_INSTANCE.set(Arc::new(model)).is_ok();
        };

        Ok(QUANTIZED_INSTANCE.get().expect("QUANTIZED_INSTANCE"))
    }

    pub fn init(spec: MistralrsSpec) -> Result<()> {
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
    pub async fn load(spec: MistralrsSpec) -> Result<MistralrsModel> {
        let spec_clone = spec.clone();
        initialize_logging();

        #[cfg(not(feature = "flash-attn"))]
        let use_flash_attn = false;
        #[cfg(feature = "flash-attn")]
        let use_flash_attn = true;

        let model_repo = spec.model_repo.expect("model_repo");
        let model_file = spec.model_file.expect("model_file");

        //TODO: IMPLEMENT
        let model = match spec.model_selected.unwrap().as_str() {
            "gguf" => ModelSelected::GGUF {
                tok_model_id: None,
                quantized_model_id: model_repo.clone(),
                quantized_filename: model_file.clone(),
                topology: None,
            },
            _ => todo!(),
        };

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

        let tgt_non_granular_index = get_tgt_non_granular_index(&model);
        let dtype = get_model_dtype(&model)?;

        let max_seqs = if tgt_non_granular_index.is_some() {
            1
        } else {
            spec.max_seqs.unwrap_or(16)
        };

        let prompt_batchsize = match spec.prompt_batchsize {
            Some(0) => {
                anyhow::bail!("`prompt_batchsize` must be a strictly positive integer, got 0.",)
            }
            Some(x) => Some(NonZeroUsize::new(x).unwrap()),
            None => None,
        };

        let loader: Box<dyn Loader> = LoaderBuilder::new(model)
            .with_no_kv_cache(spec.no_kv_cache.unwrap_or(false))
            .with_chat_template(spec.chat_template)
            .with_use_flash_attn(use_flash_attn)
            .with_prompt_batchsize(prompt_batchsize)
            .build()?;

        #[cfg(feature = "metal")]
        let device = Device::new_metal(0)?;
        #[cfg(not(feature = "metal"))]
        let device = Device::cuda_if_available(0)?;

        if let Some(seed) = spec.seed {
            device.set_seed(seed)?;
        }

        info!(
            "avx: {}, neon: {}, simd128: {}, f16c: {}",
            candle_core::utils::with_avx(),
            candle_core::utils::with_neon(),
            candle_core::utils::with_simd128(),
            candle_core::utils::with_f16c()
        );
        info!("Sampling method: penalties -> temperature -> topk -> topp -> minp -> multinomial");
        if use_flash_attn {
            info!("Using flash attention.");
        }
        if use_flash_attn && loader.get_kind().is_quantized() {
            warn!("Using flash attention with a quantized model has no effect!")
        }
        info!("Model kind is: {}", loader.get_kind().to_string());

        // Parse device mapper
        let mapper = if let Some(device_layers) = spec.num_device_layers {
            let device_layers: Vec<&str> = device_layers.split(";").collect();
            if device_layers.len() == 1 && device_layers[0].parse::<usize>().is_ok() {
                let layers = device_layers[0].parse::<usize>().unwrap();
                DeviceMapMetadata::from_num_device_layers(vec![DeviceLayerMapMetadata {
                    ordinal: 0,
                    layers,
                }])
            } else {
                let mut mapping = Vec::new();
                for layer in device_layers {
                    let split = layer.splitn(2, ':').collect::<Vec<_>>();
                    if split.len() < 2 {
                        panic!("Expected layer to be of format ORD:NUM, got {layer}");
                    }
                    let ord = split[0]
                        .parse::<usize>()
                        .unwrap_or_else(|_| panic!("Failed to parse {} as integer.", split[0]));
                    let num = split[1]
                        .parse::<usize>()
                        .unwrap_or_else(|_| panic!("Failed to parse {} as integer.", split[1]));
                    for DeviceLayerMapMetadata { ordinal, layers: _ } in &mapping {
                        if *ordinal == ord {
                            panic!("Duplicate ordinal {ord}");
                        }
                    }
                    mapping.push(DeviceLayerMapMetadata {
                        ordinal: ord,
                        layers: num,
                    });
                }
                DeviceMapMetadata::from_num_device_layers(mapping)
            }
        } else {
            DeviceMapMetadata::dummy()
        };

        // Allocate 0.5 GB of CPU memory just as a placeholder.
        // Nothing happens here as we have no `swap_out`, see `_preempt_by_swap`.
        let cache_config = match (
            spec.paged_attn_block_size,
            spec.paged_attn_gpu_mem,
            spec.paged_attn_gpu_mem_usage,
            spec.paged_ctxt_len,
            paged_attn_supported(),
            spec.no_paged_attn.unwrap_or(false),
        ) {
            (block_size, None, None, None, true, false) => Some(PagedAttentionConfig::new(
                block_size,
                512,
                MemoryGpuConfig::Utilization(0.9), // NOTE(EricLBuehler): default is to use 90% of memory
            )?),
            (block_size, None, None, Some(ctxt), true, false) => Some(PagedAttentionConfig::new(
                block_size,
                512,
                MemoryGpuConfig::ContextSize(ctxt),
            )?),
            (block_size, None, Some(f), None, true, false) => Some(PagedAttentionConfig::new(
                block_size,
                512,
                MemoryGpuConfig::Utilization(f),
            )?),
            (block_size, Some(m), None, None, true, false) => Some(PagedAttentionConfig::new(
                block_size,
                512,
                MemoryGpuConfig::Amount(m),
            )?),
            (block_size, Some(_m), Some(f), None, true, false) => {
                info!("Both memory size, and usage were specified, defaulting to the usage value.");
                Some(PagedAttentionConfig::new(
                    block_size,
                    512,
                    MemoryGpuConfig::Utilization(f),
                )?)
            }
            (block_size, Some(_m), None, Some(ctxt), true, false) => {
                info!("All memory size and ctxt len, defaulting to the context len value.");
                Some(PagedAttentionConfig::new(
                    block_size,
                    512,
                    MemoryGpuConfig::ContextSize(ctxt),
                )?)
            }
            (block_size, None, Some(f), Some(_ctxt), true, false) => {
                info!("Both ctxt len and usage were specified, defaulting to the usage value.");
                Some(PagedAttentionConfig::new(
                    block_size,
                    512,
                    MemoryGpuConfig::Utilization(f),
                )?)
            }
            (_, _, _, _, _, _) => None,
        };

        let pipeline = loader.load_model_from_hf(
            spec_clone.model_revision.clone(),
            TokenSource::CacheToken,
            &dtype,
            &device,
            false,
            mapper,
            None, //spec.in_situ_quant,
            cache_config,
        )?;
        info!("Model loaded.");

        let scheduler_config = if cache_config.is_some() {
            // Handle case where we may have device mapping
            if let Some(ref cache_config) = pipeline.lock().await.get_metadata().cache_config {
                SchedulerConfig::PagedAttentionMeta {
                    max_num_seqs: max_seqs,
                    config: cache_config.clone(),
                }
            } else {
                SchedulerConfig::DefaultScheduler {
                    method: DefaultSchedulerMethod::Fixed(max_seqs.try_into().unwrap()),
                }
            }
        } else {
            SchedulerConfig::DefaultScheduler {
                method: DefaultSchedulerMethod::Fixed(max_seqs.try_into().unwrap()),
            }
        };
        // Throughput logging in the server
        let builder = MistralRsBuilder::new(pipeline, scheduler_config)
            .with_opt_log(None)
            .with_truncate_sequence(spec.truncate_sequence.unwrap_or(false))
            .with_no_kv_cache(spec.no_kv_cache.unwrap_or(false))
            .with_prefix_cache_n(spec.prefix_cache_n.unwrap_or(16));

        let builder = if spec.throughput_log.unwrap_or(false) {
            builder.with_throughput_logging()
        } else {
            builder
        };
        let mistralrs = builder.build();

        Ok(MistralrsModel {
            spec: spec_clone,
            mistralrs,
            device,
        })
    }
}
