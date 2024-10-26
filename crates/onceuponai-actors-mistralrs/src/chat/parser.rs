use super::util;
use anyhow::{Context, Result};
use either::Either;
use indexmap::IndexMap;
use log::warn;
use mistralrs::{
    Constraint, DrySamplingParams, MistralRs, NormalRequest, Request, RequestMessage, Response,
    SamplingParams, StopTokens as InternalStopTokens,
};
use onceuponai_actors::abstractions::openai::{
    ChatCompletionRequest, CompletionRequest, Function, Grammar, ImageGenerationRequest,
    ImageGenerationResponseFormat, MessageInnerContent, StopTokens, Tool, ToolChoice, ToolType,
};
use std::sync::Arc;
use std::{collections::HashMap, ops::Deref};
use tokio::sync::mpsc::Sender;

pub(crate) fn parse_image_generation_request(
    oairequest: ImageGenerationRequest,
    state: Arc<MistralRs>,
    tx: Sender<Response>,
) -> Result<Request> {
    let repr = serde_json::to_string(&oairequest).expect("Serialization of request failed.");
    MistralRs::maybe_log_request(state.clone(), repr);

    Ok(Request::Normal(NormalRequest {
        id: state.next_request_id(),
        messages: RequestMessage::ImageGeneration {
            prompt: oairequest.prompt,
            format: ImageGenerationResponseFormatWrapper(oairequest.response_format).into(),
            generation_params: mistralrs::DiffusionGenerationParams {
                height: oairequest.height,
                width: oairequest.width,
            },
        },
        sampling_params: SamplingParams::deterministic(),
        response: tx,
        return_logprobs: false,
        is_streaming: false,
        suffix: None,
        constraint: Constraint::None,
        adapters: None,
        tool_choice: None,
        tools: None,
        logits_processors: None,
    }))
}

pub(crate) fn parse_completion_request(
    oairequest: CompletionRequest,
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

    if oairequest.logprobs.is_some() {
        warn!("Completion requests do not support logprobs.");
    }

    let is_streaming = oairequest.stream.unwrap_or(false);

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
    Ok((
        Request::Normal(NormalRequest {
            id: state.next_request_id(),
            messages: RequestMessage::Completion {
                text: oairequest.prompt,
                echo_prompt: oairequest.echo_prompt,
                best_of: oairequest.best_of,
            },
            sampling_params: SamplingParams {
                temperature: oairequest.temperature,
                top_k: oairequest.top_k,
                top_p: oairequest.top_p,
                min_p: oairequest.min_p,
                top_n_logprobs: 1,
                frequency_penalty: oairequest.frequency_penalty,
                presence_penalty: oairequest.presence_penalty,
                max_len: oairequest.max_tokens,
                stop_toks,
                logits_bias: oairequest.logit_bias,
                n_choices: oairequest.n_choices,
                dry_params,
            },
            response: tx,
            return_logprobs: false,
            is_streaming,
            suffix: oairequest.suffix,
            constraint: match oairequest.grammar {
                Some(Grammar::Yacc(yacc)) => Constraint::Yacc(yacc),
                Some(Grammar::Regex(regex)) => Constraint::Regex(regex),
                None => Constraint::None,
            },
            adapters: oairequest.adapters,
            tool_choice: oairequest
                .tool_choice
                .map(|tc| ToolChoiceWrapper(tc).into()),
            tools: oairequest
                .tools
                .map(|t| t.iter().map(|x| ToolWrapper(x.clone()).into()).collect()),
            logits_processors: None,
        }),
        is_streaming,
    ))
}

pub(crate) async fn parse_chat_completion_request(
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
            tool_choice: oairequest
                .tool_choice
                .map(|tc| ToolChoiceWrapper(tc).into()),
            tools: oairequest
                .tools
                .map(|t| t.iter().map(|x| ToolWrapper(x.clone()).into()).collect()),
            logits_processors: None,
        }),
        is_streaming,
    ))
}

struct ToolTypeWrapper(ToolType);

impl From<ToolTypeWrapper> for mistralrs::ToolType {
    fn from(tool_type: ToolTypeWrapper) -> Self {
        match tool_type.0 {
            ToolType::Function => mistralrs::ToolType::Function,
        }
    }
}

struct ToolWrapper(Tool);

impl From<ToolWrapper> for mistralrs::Tool {
    fn from(tool: ToolWrapper) -> Self {
        mistralrs::Tool {
            tp: ToolTypeWrapper(tool.0.tp).into(),
            function: FunctionWrapper(tool.0.function).into(),
        }
    }
}

struct FunctionWrapper(Function);
impl From<FunctionWrapper> for mistralrs::Function {
    fn from(function: FunctionWrapper) -> Self {
        mistralrs::Function {
            description: function.0.description,
            name: function.0.name,
            parameters: function.0.parameters,
        }
    }
}

struct ToolChoiceWrapper(ToolChoice);
impl From<ToolChoiceWrapper> for mistralrs::ToolChoice {
    fn from(tool_choice: ToolChoiceWrapper) -> Self {
        match tool_choice.0 {
            ToolChoice::Auto => mistralrs::ToolChoice::Auto,
            ToolChoice::None => mistralrs::ToolChoice::None,
            ToolChoice::Tool(tool) => mistralrs::ToolChoice::Tool(ToolWrapper(tool).into()),
        }
    }
}

struct ImageGenerationResponseFormatWrapper(ImageGenerationResponseFormat);
impl From<ImageGenerationResponseFormatWrapper> for mistralrs::ImageGenerationResponseFormat {
    fn from(response_format: ImageGenerationResponseFormatWrapper) -> Self {
        match response_format.0 {
            ImageGenerationResponseFormat::Url => mistralrs::ImageGenerationResponseFormat::Url,
            ImageGenerationResponseFormat::B64Json => {
                mistralrs::ImageGenerationResponseFormat::B64Json
            }
        }
    }
}
