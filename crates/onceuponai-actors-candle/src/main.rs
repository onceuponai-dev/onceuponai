pub mod llm;
use crate::llm::ActorKind;
use anyhow::Result;
use clap::{arg, Command};
use onceuponai_actors::cluster::start_worker_cluster;

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
                    .help("file")])
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
}

pub(crate) async fn commands() -> Result<()> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("spawn", sub_sub_matches)) => {
            let file = sub_sub_matches.get_one::<String>("file");
            let json = sub_sub_matches.get_one::<String>("json");
            let yaml = sub_sub_matches.get_one::<String>("yaml");
            let metadata_yaml = sub_sub_matches.get_one::<String>("metadata");
            start_worker_cluster::<ActorKind>(file, yaml, json, metadata_yaml).await?
        }
        _ => unreachable!(),
    }

    Ok(())
}

#[actix_rt::main]
async fn main() -> Result<()> {
    commands().await
}
