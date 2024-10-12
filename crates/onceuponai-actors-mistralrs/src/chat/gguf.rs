use super::{
    openai::{
        ChatCompletionRequest, Grammar, Message, MessageContent, MessageInnerContent, StopTokens,
    },
    util,
};
use actix_telepathy::RemoteAddr;
use anyhow::{Context, Result};
use async_trait::async_trait;
use either::Either;
use indexmap::IndexMap;
use log::{info, warn};
use mistralrs::{
    get_model_dtype, get_tgt_non_granular_index, initialize_logging, paged_attn_supported,
    Constraint, DefaultSchedulerMethod, Device, DeviceLayerMapMetadata, DeviceMapMetadata,
    DrySamplingParams, Loader, LoaderBuilder, MemoryGpuConfig, MistralRs, MistralRsBuilder,
    ModelSelected, NormalRequest, PagedAttentionConfig, Request, RequestMessage, Response,
    SamplingParams, SchedulerConfig, StopTokens as InternalStopTokens, TokenSource,
};
use once_cell::sync::OnceCell;
use onceuponai_abstractions::EntityValue;
use onceuponai_actors::abstractions::{
    ActorActions, ActorError, ActorInvokeError, ActorInvokeFinish, ActorInvokeRequest,
    ActorInvokeResponse, ActorInvokeResult,
};
use onceuponai_core::common::{hf_hub_get, hf_hub_get_path};
use serde::Deserialize;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::{collections::HashMap, ops::Deref};
use tokio::sync::mpsc::{channel, Sender};
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

    async fn start(&self) -> Result<()> {
        let spec = self.clone();
        tokio::task::spawn_local(async move { MistralrsModel::lazy(spec).await.unwrap() }).await?;

        println!("SPEC: {:?}", self);

        Ok(())
    }

    async fn invoke(
        &self,
        uuid: Uuid,
        requesti: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        Ok(())
    }

    async fn invoke_stream(
        &self,
        uuid: Uuid,
        requesti: &ActorInvokeRequest,
        source: RemoteAddr,
    ) -> Result<()> {
        let state = MISTRALRS_INSTANCE.get().unwrap().clone();
        let input = requesti.data.get("message");

        if input.is_none() {
            source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
            uuid,
            task_id: requesti.task_id,
            error: ActorError::BadRequest(
                "REQUEST MUST CONTAINER MESSAGE COLUMN WITH Vec<MESSAGE { role: String, content: String }>".to_string(),
            ),
        }));
            return Ok(());
        }

        let messages: Vec<Message> = input
            .expect("MESSAGE")
            .iter()
            .map(|x| match x {
                EntityValue::MESSAGE { role, content } => Message {
                    role: role.clone(),
                    name: None,
                    content: super::openai::MessageContent(Either::Left(content.to_string())),
                },
                _ => todo!(),
            })
            .collect();
        let oairequest = ChatCompletionRequest {
            messages: Either::Left(messages),
            model: "".to_string(),
            logit_bias: None,
            logprobs: false,
            top_logprobs: None,
            max_tokens: None,
            n_choices: 1,
            presence_penalty: None,
            frequency_penalty: None,
            stop_seqs: None,
            temperature: None,
            top_p: None,
            stream: Some(true),
            tools: None,
            tool_choice: None,
            top_k: None,
            grammar: None,
            adapters: None,
            min_p: None,
            dry_multiplier: None,
            dry_base: None,
            dry_allowed_length: None,
            dry_sequence_breakers: None,
        };

        let (tx, mut rx) = channel(10_000);
        let (request, is_streaming) =
            match parse_request(oairequest, state.mistralrs.clone(), tx).await {
                Ok(x) => x,
                Err(e) => {
                    println!("ERROR {:?}", e);
                    return Ok(());
                    // let e = anyhow::Error::msg(e.to_string());
                    // MistralRs::maybe_log_error(state, &*e);
                    // return ChatCompletionResponder::InternalError(e.into());
                }
            };
        let sender = state.mistralrs.get_sender().unwrap();
        if let Err(e) = sender.send(request).await {
            println!("ERROR {:?}", e);
            return Ok(());
            // let e = anyhow::Error::msg(e.to_string());
            // MistralRs::maybe_log_error(state, &*e);
            // return ChatCompletionResponder::InternalError(e.into());
        }

        loop {
            if let Ok(resp) = rx.try_recv() {
                match resp {
                    Response::ModelError(msg, _) => {
                        source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
                            uuid,
                            task_id: requesti.task_id,
                            error: ActorError::FatalError(msg),
                        }));
                        break;
                    }
                    Response::ValidationError(e) => {
                        source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
                            uuid,
                            task_id: requesti.task_id,
                            error: ActorError::FatalError(format!("{}", e)),
                        }));
                        break;
                    }
                    Response::InternalError(e) => {
                        source.do_send(ActorInvokeResponse::Failure(ActorInvokeError {
                            uuid,
                            task_id: requesti.task_id,
                            error: ActorError::FatalError(format!("{}", e)),
                        }));
                        break;
                    }
                    Response::Chunk(response) => {
                        if response.choices.iter().all(|x| x.finish_reason.is_some()) {
                            let result = ActorInvokeFinish {
                                uuid,
                                task_id: requesti.task_id,
                                stream: requesti.stream,
                            };
                            let response = ActorInvokeResponse::Finish(result);
                            source.do_send(response);
                            break;
                        }

                        let content = response.choices[0].clone().delta.content;
                        let response = ActorInvokeResponse::Success(ActorInvokeResult {
                            uuid,
                            task_id: requesti.task_id,
                            stream: requesti.stream,
                            metadata: HashMap::new(),
                            data: HashMap::from([(
                                String::from("content"),
                                vec![EntityValue::STRING(content)],
                            )]),
                        });

                        source.do_send(response);
                        actix_rt::task::yield_now().await;
                    }
                    Response::Done(_) => unreachable!(),
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

async fn parse_request(
    oairequest: ChatCompletionRequest,
    state: Arc<MistralRs>,
    tx: Sender<Response>,
) -> Result<(Request, bool)> {
    let repr = serde_json::to_string(&oairequest).expect("Serialization of request failed.");
    MistralRs::maybe_log_request(state.clone(), repr);

    let stop_toks = match oairequest.stop_seqs {
        Some(StopTokens::Multi(m)) => Some(InternalStopTokens::Seqs(m)),
        Some(StopTokens::Single(s)) => Some(InternalStopTokens::Seqs(vec![s])),
        None => None,
    };
    let messages = match oairequest.messages {
        Either::Left(req_messages) => {
            let mut messages = Vec::new();
            let mut image_urls = Vec::new();
            for message in req_messages {
                match message.content.deref() {
                    Either::Left(content) => {
                        let mut message_map: IndexMap<
                            String,
                            Either<String, Vec<IndexMap<String, String>>>,
                        > = IndexMap::new();
                        message_map.insert("role".to_string(), Either::Left(message.role));
                        message_map
                            .insert("content".to_string(), Either::Left(content.to_string()));
                        messages.push(message_map);
                    }
                    Either::Right(image_messages) => {
                        if image_messages.len() != 2 {
                            anyhow::bail!(
                                "Expected 2 items for the content of a message with an image."
                            );
                        }
                        if message.role != "user" {
                            anyhow::bail!(
                                "Role for an image message must be `user`, but it is {}",
                                message.role
                            );
                        }

                        let mut items = Vec::new();
                        for image_message in image_messages {
                            if image_message.len() != 2 {
                                anyhow::bail!("Expected 2 items for the sub-content of a message with an image.");
                            }
                            if !image_message.contains_key("type") {
                                anyhow::bail!("Expected `type` key in input message.");
                            }
                            if image_message["type"].is_right() {
                                anyhow::bail!("Expected string value in `type`.");
                            }
                            items.push(image_message["type"].as_ref().unwrap_left().clone())
                        }

                        fn get_content_and_url(
                            text_idx: usize,
                            url_idx: usize,
                            image_messages: &[HashMap<String, MessageInnerContent>],
                        ) -> Result<(String, String)> {
                            if image_messages[text_idx]["text"].is_right() {
                                anyhow::bail!("Expected string value in `text`.");
                            }
                            let content = image_messages[text_idx]["text"]
                                .as_ref()
                                .unwrap_left()
                                .clone();
                            if image_messages[url_idx]["image_url"].is_left()
                                || !image_messages[url_idx]["image_url"]
                                    .as_ref()
                                    .unwrap_right()
                                    .contains_key("url")
                            {
                                anyhow::bail!("Expected content of format {{`type`: `text`, `text`: ...}} and {{`type`: `url`, `image_url`: {{`url`: ...}}}}")
                            }
                            let url = image_messages[url_idx]["image_url"].as_ref().unwrap_right()
                                ["url"]
                                .clone();
                            Ok((content, url))
                        }
                        let mut message_map: IndexMap<
                            String,
                            Either<String, Vec<IndexMap<String, String>>>,
                        > = IndexMap::new();
                        message_map.insert("role".to_string(), Either::Left(message.role));
                        let (content, url) = if items[0] == "text" {
                            get_content_and_url(0, 1, image_messages)?
                        } else {
                            get_content_and_url(1, 0, image_messages)?
                        };

                        let mut content_map = Vec::new();
                        let mut content_image_map = IndexMap::new();
                        content_image_map.insert("type".to_string(), "image".to_string());
                        content_map.push(content_image_map);
                        let mut content_text_map = IndexMap::new();
                        content_text_map.insert("type".to_string(), "text".to_string());
                        content_text_map.insert("text".to_string(), content);
                        content_map.push(content_text_map);

                        message_map.insert("content".to_string(), Either::Right(content_map));
                        messages.push(message_map);
                        image_urls.push(url);
                    }
                }
            }
            if !image_urls.is_empty() {
                let mut images = Vec::new();
                for url_unparsed in image_urls {
                    let image = util::parse_image_url(&url_unparsed)
                        .await
                        .with_context(|| {
                            format!("Failed to parse image resource: {}", url_unparsed)
                        })?;

                    images.push(image);
                }
                RequestMessage::VisionChat { messages, images }
            } else {
                RequestMessage::Chat(messages)
            }
        }
        Either::Right(prompt) => {
            let mut messages = Vec::new();
            let mut message_map: IndexMap<String, Either<String, Vec<IndexMap<String, String>>>> =
                IndexMap::new();
            message_map.insert("role".to_string(), Either::Left("user".to_string()));
            message_map.insert("content".to_string(), Either::Left(prompt));
            messages.push(message_map);
            RequestMessage::Chat(messages)
        }
    };

    let dry_params = if let Some(dry_multiplier) = oairequest.dry_multiplier {
        Some(DrySamplingParams::new_with_defaults(
            dry_multiplier,
            oairequest.dry_sequence_breakers,
            oairequest.dry_base,
            oairequest.dry_allowed_length,
        )?)
    } else {
        None
    };

    let is_streaming = oairequest.stream.unwrap_or(false);
    Ok((
        Request::Normal(NormalRequest {
            id: state.next_request_id(),
            messages,
            sampling_params: SamplingParams {
                temperature: oairequest.temperature,
                top_k: oairequest.top_k,
                top_p: oairequest.top_p,
                min_p: oairequest.min_p,
                top_n_logprobs: oairequest.top_logprobs.unwrap_or(1),
                frequency_penalty: oairequest.frequency_penalty,
                presence_penalty: oairequest.presence_penalty,
                max_len: oairequest.max_tokens,
                stop_toks,
                logits_bias: oairequest.logit_bias,
                n_choices: oairequest.n_choices,
                dry_params,
            },
            response: tx,
            return_logprobs: oairequest.logprobs,
            is_streaming,
            suffix: None,
            constraint: match oairequest.grammar {
                Some(Grammar::Yacc(yacc)) => Constraint::Yacc(yacc),
                Some(Grammar::Regex(regex)) => Constraint::Regex(regex),
                None => Constraint::None,
            },
            adapters: oairequest.adapters,
            tool_choice: oairequest.tool_choice,
            tools: oairequest.tools,
            logits_processors: None,
        }),
        is_streaming,
    ))
}

pub struct MistralrsModel {
    pub spec: MistralrsSpec,
    pub mistralrs: Arc<MistralRs>,
    pub device: Device,
}

impl MistralrsModel {
    pub async fn lazy<'a>(spec: MistralrsSpec) -> Result<&'a Arc<MistralrsModel>> {
        if MISTRALRS_INSTANCE.get().is_none() {
            let model = MistralrsModel::load(spec.clone()).await?;

            let _ = MISTRALRS_INSTANCE.set(Arc::new(model)).is_ok();
        };

        Ok(MISTRALRS_INSTANCE.get().expect("QUANTIZED_INSTANCE"))
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
