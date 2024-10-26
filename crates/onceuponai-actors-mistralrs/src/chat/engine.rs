// based on:
// https://github.com/EricLBuehler/mistral.rs/blob/master/mistralrs-server/src/main.rs
use super::parser::parse_chat_completion_request;
use actix_telepathy::RemoteAddr;
use anyhow::Result;
use async_trait::async_trait;
use log::{info, warn};
use mistralrs::{
    get_model_dtype, get_tgt_non_granular_index, initialize_logging, paged_attn_supported,
    DefaultSchedulerMethod, Device, DeviceLayerMapMetadata, DeviceMapMetadata, IsqType, Loader,
    LoaderBuilder, MemoryGpuConfig, MistralRs, MistralRsBuilder, ModelDType, ModelSelected,
    PagedAttentionConfig, Response, SchedulerConfig, TokenSource,
};
use once_cell::sync::OnceCell;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors::abstractions::{
    ActorActions, ActorError, ActorInvokeData, ActorInvokeError, ActorInvokeFinish,
    ActorInvokeRequest, ActorInvokeResponse, ActorInvokeResult,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::{num::NonZeroUsize, path::PathBuf};
use tokio::sync::mpsc::channel;
use uuid::Uuid;

static MISTRALRS_INSTANCE: OnceCell<Arc<MistralrsModel>> = OnceCell::new();

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
    /// Model Selected: plain, xlora, lora, gguf, xloragguf, loragguf, ggml, xloraggml, loraggml, visionplain, diffusionplain
    pub model_selected: Option<String>,
    /// Model ID to load from. This may be a HF hub repo or a local path.
    pub model_id: Option<String>,
    /// Model filename local path
    pub tokenizer_json: Option<String>,
    /// Model revision from HF
    pub model_revision: Option<String>,
    /// Model architecture: mistral, gemma, mixtral, llama, phi2, phi3, qwen2, gemma2, starcoder2, phi3.5moe
    pub model_architecture: Option<String>,
    /// Model dtype: auto, bf16, f16, f32
    pub model_dtype: Option<String>,
    pub quantized_model_id: Option<String>,
    pub quantized_filename: Option<String>,
    pub tok_model_id: Option<String>,
    pub topology: Option<String>,
    pub organization: Option<String>,
    pub write_uqff: Option<String>,
    pub from_uqff: Option<String>,
    /// Model ID to load LoRA from. This may be a HF hub repo or a local path.
    pub adapters_model_id: Option<String>,
    pub order: Option<String>,
    pub xlora_model_id: Option<String>,
    pub tgt_non_granular_index: Option<usize>,
    pub gqa: Option<usize>,
    pub vision_model_architecture: Option<String>,
    pub diffusion_model_architecture: Option<String>,
}

impl MistralrsSpec {
    async fn invoke_base(
        &self,
        uuid: Uuid,
        invoke_request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        let state = MISTRALRS_INSTANCE.get().unwrap().clone();
        let input = match invoke_request.data.clone() {
            ActorInvokeData::ChatCompletion(chat_completion_request) => chat_completion_request,
            _ => {
                source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: invoke_request.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));

                return Ok(());
            }
        };

        let (tx, mut rx) = channel(10_000);
        let (request, _is_streaming) =
            match parse_chat_completion_request(input, state.mistralrs.clone(), tx).await {
                Ok(x) => x,
                Err(e) => {
                    println!("ERROR {:?}", e);
                    source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
                        uuid,
                        task_id: invoke_request.task_id,
                        error: ActorError::FatalError(format!("{}", e)),
                    }));
                    return Ok(());
                }
            };
        let sender = state.mistralrs.get_sender().unwrap();
        if let Err(e) = sender.send(request).await {
            println!("ERROR {:?}", e);
            source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
                uuid,
                task_id: invoke_request.task_id,
                error: ActorError::FatalError(format!("{}", e)),
            }));
            return Ok(());
        }

        loop {
            if let Ok(resp) = rx.try_recv() {
                match resp {
                    Response::ModelError(msg, _) => {
                        source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
                            uuid,
                            task_id: invoke_request.task_id,
                            error: ActorError::FatalError(msg),
                        }));
                        break;
                    }
                    Response::ValidationError(e) => {
                        source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
                            uuid,
                            task_id: invoke_request.task_id,
                            error: ActorError::FatalError(format!("{}", e)),
                        }));
                        break;
                    }
                    Response::InternalError(e) => {
                        source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
                            uuid,
                            task_id: invoke_request.task_id,
                            error: ActorError::FatalError(format!("{}", e)),
                        }));
                        break;
                    }
                    Response::Chunk(response) => {
                        if response.choices.iter().all(|x| x.finish_reason.is_some()) {
                            let result = ActorInvokeFinish {
                                uuid,
                                task_id: invoke_request.task_id,
                                stream: invoke_request.stream,
                            };
                            let response = ActorInvokeResponse::Finish(result);
                            source.do_send(response);
                            break;
                        }

                        let content = response.choices[0].clone().delta.content;
                        let response = ActorInvokeResponse::Success(ActorInvokeResult {
                            uuid,
                            task_id: invoke_request.task_id,
                            stream: invoke_request.stream,
                            metadata: HashMap::new(),
                            data: HashMap::from([(
                                String::from("content"),
                                vec![EntityValue::STRING(content)],
                            )]),
                        });

                        source.do_send(response);
                        actix_rt::task::yield_now().await;
                    }
                    Response::Done(response) => {
                        let content = response.choices[0].clone().message.content.unwrap();
                        let response = ActorInvokeResponse::Success(ActorInvokeResult {
                            uuid,
                            task_id: invoke_request.task_id,
                            stream: invoke_request.stream,
                            metadata: HashMap::new(),
                            data: HashMap::from([(
                                String::from("content"),
                                vec![EntityValue::STRING(content)],
                            )]),
                        });

                        source.do_send(response);
                        actix_rt::task::yield_now().await;
                    }
                    Response::CompletionDone(_) => unreachable!(),
                    Response::CompletionModelError(_, _) => unreachable!(),
                    Response::CompletionChunk(_) => unreachable!(),
                    Response::ImageGeneration(_) => unreachable!(),
                }
            }
        }

        Ok(())
    }
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

    async fn start(&self) -> Result<()> {
        let spec = self.clone();
        tokio::task::spawn_local(async move { MistralrsModel::lazy(spec).await.unwrap() }).await?;

        println!("SPEC: {:?}", self);

        Ok(())
    }

    async fn invoke(
        &self,
        uuid: Uuid,
        invoke_request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        self.invoke_base(uuid, invoke_request, source).await?;
        Ok(())
    }

    async fn invoke_stream(
        &self,
        uuid: Uuid,
        invoke_request: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        self.invoke_base(uuid, invoke_request, source).await?;
        Ok(())
    }
}

pub struct MistralrsModel {
    pub spec: MistralrsSpec,
    pub mistralrs: Arc<MistralRs>,
    pub device: Device,
}

impl MistralrsModel {
    pub async fn lazy<'a>(spec: MistralrsSpec) -> Result<&'a Arc<MistralrsModel>> {
        if MISTRALRS_INSTANCE.get().is_none() {
            let model = match MistralrsModel::load(spec.clone()).await {
                Ok(m) => m,
                Err(e) => {
                    info!("{:?}", e);
                    anyhow::bail!("{}", e)
                }
            };

            let _ = MISTRALRS_INSTANCE.set(Arc::new(model)).is_ok();
        };

        Ok(MISTRALRS_INSTANCE.get().expect("QUANTIZED_INSTANCE"))
    }

    pub fn init(_spec: MistralrsSpec) -> Result<()> {
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

        let arch = if let Some(ma) = spec.model_architecture {
            serde_json::from_str(&format!("\"{}\"", ma))?
        } else {
            None
        };

        let dtype = if let Some(mdt) = spec.model_dtype {
            serde_json::from_str(&format!("\"{}\"", mdt))?
        } else {
            ModelDType::Auto
        };

        let organization = if let Some(o) = spec.organization {
            Some(serde_json::from_str(&format!("\"{}\"", o))?)
        } else {
            None
        };

        let write_uqff = spec.write_uqff.map(PathBuf::from);
        let from_uqff = spec.from_uqff.map(PathBuf::from);
        let topology = spec.topology;
        let tokenizer_json = spec.tokenizer_json;

        let model = match spec.model_selected.unwrap().as_str() {
            "plain" => ModelSelected::Plain {
                model_id: spec.model_id.expect("model_id"),
                tokenizer_json,
                arch,
                dtype,
                topology,
                organization,
                write_uqff,
                from_uqff,
            },
            "xlora" => ModelSelected::XLora {
                model_id: spec.model_id,
                tokenizer_json,
                xlora_model_id: spec.xlora_model_id.expect("xlora_model_id"),
                order: spec.order.expect("order"),
                tgt_non_granular_index: spec.tgt_non_granular_index,
                arch,
                dtype,
                topology,
                write_uqff,
                from_uqff,
            },
            "lora" => ModelSelected::Lora {
                model_id: spec.model_id,
                tokenizer_json,
                adapters_model_id: spec.adapters_model_id.expect("adapters_model_id"),
                order: spec.order.expect("order"),
                arch,
                dtype,
                topology,
                write_uqff,
                from_uqff,
            },
            "gguf" => ModelSelected::GGUF {
                tok_model_id: spec.tok_model_id,
                quantized_model_id: spec.quantized_model_id.expect("quantized_model_id"),
                quantized_filename: spec.quantized_filename.expect("quantized_filename"),
                topology,
            },
            "xloragguf" => ModelSelected::XLoraGGUF {
                tok_model_id: spec.tok_model_id,
                quantized_model_id: spec.quantized_model_id.expect("quantized_model_id"),
                quantized_filename: spec.quantized_filename.expect("quantized_filename"),
                xlora_model_id: spec.xlora_model_id.expect("xlora_model_id"),
                order: spec.order.expect("order"),
                tgt_non_granular_index: spec.tgt_non_granular_index,
                topology,
            },
            "loragguf" => ModelSelected::LoraGGUF {
                tok_model_id: spec.tok_model_id,
                quantized_model_id: spec.quantized_model_id.expect("quantized_model_id"),
                quantized_filename: spec.quantized_filename.expect("quantized_filename"),
                adapters_model_id: spec.adapters_model_id.expect("adapters_model_id"),
                order: spec.order.expect("order"),
                topology,
            },
            "ggml" => ModelSelected::GGML {
                tok_model_id: spec.tok_model_id.expect("tok_model_id"),
                quantized_model_id: spec.quantized_model_id.expect("quantized_model_id"),
                quantized_filename: spec.quantized_filename.expect("quantized_filename"),
                topology,
                tokenizer_json,
                gqa: spec.gqa.expect("gqa"),
            },
            "xloraggml" => ModelSelected::XLoraGGML {
                tok_model_id: spec.tok_model_id,
                quantized_model_id: spec.quantized_model_id.expect("quantized_model_id"),
                quantized_filename: spec.quantized_filename.expect("quantized_filename"),
                topology,
                tokenizer_json,
                gqa: spec.gqa.expect("gqa"),
                order: spec.order.expect("order"),
                xlora_model_id: spec.xlora_model_id.expect("xlora_model_id"),
                tgt_non_granular_index: spec.tgt_non_granular_index,
            },
            "loraggml" => ModelSelected::LoraGGML {
                tok_model_id: spec.tok_model_id,
                quantized_model_id: spec.quantized_model_id.expect("quantized_model_id"),
                quantized_filename: spec.quantized_filename.expect("quantized_filename"),
                topology,
                tokenizer_json,
                gqa: spec.gqa.expect("gqa"),
                order: spec.order.expect("order"),
                adapters_model_id: spec.adapters_model_id.expect("adapters_model_id"),
            },
            "visionplain" => ModelSelected::VisionPlain {
                model_id: spec.model_id.expect("model_id"),
                tokenizer_json,
                arch: serde_json::from_str(&format!(
                    "\"{}\"",
                    &spec
                        .vision_model_architecture
                        .expect("model_vision_architecture")
                ))?,
                dtype,
                topology,
                write_uqff,
                from_uqff,
            },
            "diffusionplain" => ModelSelected::DiffusionPlain {
                model_id: spec.model_id.expect("model_id"),
                arch: serde_json::from_str(&format!(
                    "\"{}\"",
                    &spec
                        .diffusion_model_architecture
                        .expect("model_vision_architecture")
                ))?,
                dtype,
            },
            _ => unreachable!(),
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

        let token_source = if let Some(hf_token) = spec.hf_token {
            TokenSource::Literal(hf_token)
        } else if let Ok(value) = std::env::var("HF_TOKEN") {
            TokenSource::EnvVar("HF_TOKEN".to_string())
        } else {
            TokenSource::CacheToken
        };

        let in_situ_quant = if let Some(isq) = spec.in_situ_quant {
            match isq.as_str() {
                "Q4_0" => Some(IsqType::Q4_0),
                "Q4_1" => Some(IsqType::Q4_1),
                "Q5_0" => Some(IsqType::Q5_0),
                "Q5_1" => Some(IsqType::Q5_1),
                "Q8_0" => Some(IsqType::Q8_0),
                "Q8_1" => Some(IsqType::Q8_1),
                "Q2K" => Some(IsqType::Q2K),
                "Q3K" => Some(IsqType::Q3K),
                "Q4K" => Some(IsqType::Q4K),
                "Q5K" => Some(IsqType::Q5K),
                "Q6K" => Some(IsqType::Q6K),
                "Q8K" => Some(IsqType::Q8K),
                "HQQ8" => Some(IsqType::HQQ8),
                "HQQ4" => Some(IsqType::HQQ4),
                _ => None,
            }
        } else {
            None
        };

        let pipeline = loader.load_model_from_hf(
            spec_clone.model_revision.clone(),
            token_source,
            &dtype,
            &device,
            false,
            mapper,
            in_situ_quant,
            cache_config,
        )?;
        info!("Model loaded.");

        let scheduler_config = if cache_config.is_some() {
            let metadata = pipeline.lock().await.get_metadata();
            // Handle case where we may have device mapping
            if let Some(ref cache_config) = metadata.cache_config {
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
