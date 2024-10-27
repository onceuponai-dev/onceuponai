use anyhow::Result;
use candle_core::Device;
use chat::gemma::GemmaSpec;
use chat::mistral::MistralSpec;
use chat::openai_chat::OpenAIChatSpec;
use chat::quantized::QuantizedSpec;
use clap::{arg, Command};
use embeddings::e5::E5Spec;
use onceuponai_actors::abstractions::{ActorActions, ActorKindActions, ActorMetadata, ActorObject};
use onceuponai_actors::cluster::{init_actor, start_worker_cluster};
use onceuponai_actors::initialize::initialize;
use serde::Deserialize;

pub mod chat;
pub mod embeddings;

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ActorKind {
    E5(ActorObject<E5Spec>),
    Gemma(ActorObject<GemmaSpec>),
    Mistral(ActorObject<MistralSpec>),
    Quantized(ActorObject<QuantizedSpec>),
    Openaichat(ActorObject<OpenAIChatSpec>),
}

impl ActorKindActions for ActorKind {
    fn actor(&self) -> Box<dyn ActorActions> {
        match self {
            ActorKind::E5(object) => Box::new(object.spec()),
            ActorKind::Gemma(object) => Box::new(object.spec()),
            ActorKind::Mistral(object) => Box::new(object.spec()),
            ActorKind::Quantized(object) => Box::new(object.spec()),
            ActorKind::Openaichat(object) => Box::new(object.spec()),
        }
    }

    fn metadata(&self) -> ActorMetadata {
        match self {
            ActorKind::E5(object) => object.metadata(),
            ActorKind::Gemma(object) => object.metadata(),
            ActorKind::Mistral(object) => object.metadata(),
            ActorKind::Quantized(object) => object.metadata(),
            ActorKind::Openaichat(object) => object.metadata(),
        }
    }
}

fn parse_device(device_type: Option<String>) -> Result<Device> {
    let device_type = device_type.unwrap_or("cpu".to_string());

    let device = if device_type == "cpu" {
        Device::Cpu
    } else {
        Device::new_cuda(0).unwrap()
    };
    Ok(device)
}

fn cli() -> Command {
    Command::new("onceuponai")
        .about("onceuponai")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("spawn")
                .about("spawn")
                .args(vec![arg!(--file <FILE> "file")
                    .required(false)
                    .short('f')
                    .help("configuration file in yaml format")])
                .args(vec![arg!(--toml <TOML> "toml")
                    .required(false)
                    .short('t')
                    .help("configuration file in toml format")])
                .args(vec![arg!(--yaml <YAML> "yaml")
                    .required(false)
                    .short('y')
                    .help("config yaml in base64")])
                .args(vec![arg!(--json <JSON> "json")
                    .required(false)
                    .short('j')
                    .help("config json in base64")])
                .args(vec![arg!(--metadata <METADATA> "metadata")
                    .required(false)
                    .short('m')
                    .help("config metadata yaml in base64")])
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("init")
                .about("init")
                .args(vec![arg!(--json <JSON> "json")
                    .required(false)
                    .short('j')
                    .help("config json in base64")])
                .arg_required_else_help(true),
        )
}

pub(crate) async fn commands() -> Result<()> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("spawn", sub_sub_matches)) => {
            let file = sub_sub_matches.get_one::<String>("file");
            let toml = sub_sub_matches.get_one::<String>("toml");
            let json = sub_sub_matches.get_one::<String>("json");
            let yaml = sub_sub_matches.get_one::<String>("yaml");
            let metadata_yaml = sub_sub_matches.get_one::<String>("metadata");
            initialize().await?;
            start_worker_cluster::<ActorKind>(file, toml, yaml, json, metadata_yaml).await?
        }
        Some(("init", sub_sub_matches)) => {
            let json = sub_sub_matches.get_one::<String>("json");
            init_actor::<ActorKind>(json).await?
        }
        _ => unreachable!(),
    }

    Ok(())
}

#[actix_rt::main]
async fn main() -> Result<()> {
    commands().await
}
