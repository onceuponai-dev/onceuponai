use crate::cluster::start_cluster;
use crate::serve::{serve, vectorize};
use anyhow::Result;
use clap::{arg, Command};
use std::net::SocketAddr;

fn cli() -> Command {
    Command::new("onceuponai")
        .about("onceuponai")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("serve")
                .about("serve rag")
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
                        .default_value("true")
                        .value_parser(clap::value_parser!(bool)),
                    arg!(--modelrepo <MODELREPO> "modelrepo")
                        .required(false)
                        .help("hf model repo")
                        .default_value("TheBloke/Mistral-7B-Instruct-v0.2-GGUF"),
                    arg!(--modelfile <MODELFILE> "modelfile")
                        .required(false)
                        .help("model file")
                        .default_value("mistral-7b-instruct-v0.2.Q4_K_S.gguf"),
                    arg!(--tokenizerrepo <TOKENIZERREPO> "tokenizerrepo")
                        .required(false)
                        .help("tokenizer repo")
                        .default_value("mistralai/Mistral-7B-Instruct-v0.2"),
                    arg!(--lancedburi <LANCEDBURI> "lancedburi")
                        .required(false)
                        .help("lancedb uri")
                        .default_value("/tmp/fantasy-lancedb"),
                    arg!(--lancedbtable <LANCEDBTABLE> "lancedbtable")
                        .required(false)
                        .help("lancedb table")
                        .default_value("fantasy_vectors"),
                    arg!(--e5modelrepo <E5MODELREPO> "e5_modelrepo")
                        .required(false)
                        .help("hf e5 model repo")
                        .default_value("intfloat/e5-small-v2"),
                    arg!(--prompttemplate <PROMPTTEMPLACE> "prompttemplate")
                        .required(false)
                        .help("prompt template")
                        .default_value("You are a seller in a fantasy store. Use the following pieces of context to answer the question at the end. If you don't know the answer, just say that you don't know, don't try to make up an answer. Context: {context}.
Question: {question}"),
                    arg!(--device <DEVICE> "device")
                        .required(false)
                        .help("device: cpu or gpu")
                        .default_value("cpu"),

                ])
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("vectorize")
                .about("vectorize")
                .args(vec![
                    arg!(--loglevel <LOGLEVEL>)
                        .required(false)
                        .help("log level")
                        .default_value("error"),
                   arg!(--lancedburi <LANCEDBURI> "lancedburi")
                        .required(false)
                        .help("lancedb uri")
                        .default_value("/tmp/fantasylancedb"),
                    arg!(--lancedbtable <LANCEDBTABLE> "lancedbtable")
                        .required(false)
                        .help("lancedb table")
                        .default_value("fantasy_vectors"),
                    arg!(--e5modelrepo <E5MODELREPO> "e5modelrepo")
                        .required(false)
                        .help("hf e5 model repo")
                        .default_value("intfloat/e5-small-v2"),
                    arg!(--device <DEVICE> "device")
                        .required(false)
                        .help("device: cpu or gpu")
                        .default_value("cpu"),

                ])
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("apply")
                .about("apply")
                .args(vec![
                    arg!(--loglevel <LOGLEVEL>)
                        .required(false)
                        .help("log level")
                        .default_value("error"),
                    arg!(--file <FILE> "file")
                        .required(true)
                        .short('f')
                        .help("file")
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

            let model_repo = sub_sub_matches
                .get_one::<String>("modelrepo")
                .expect("required");

            let model_file = sub_sub_matches
                .get_one::<String>("modelfile")
                .expect("required");

            let tokenizer_repo = sub_sub_matches
                .get_one::<String>("tokenizerrepo")
                .expect("required");

            let lancedb_uri = sub_sub_matches
                .get_one::<String>("lancedburi")
                .expect("required");

            let lancedb_table = sub_sub_matches
                .get_one::<String>("lancedbtable")
                .expect("required");

            let e5_model_repo = sub_sub_matches
                .get_one::<String>("e5modelrepo")
                .expect("required");

            let prompt_template = sub_sub_matches
                .get_one::<String>("prompttemplate")
                .expect("required");

            let device = sub_sub_matches
                .get_one::<String>("device")
                .expect("required");

            // let hftoken = sub_sub_matches
            //     .get_one::<String>("hftoken")
            //     .expect("required")
            //     .clone();

            serve(
                host,
                *port,
                log_level,
                workers,
                use_quantized,
                model_repo,
                model_file,
                Some(tokenizer_repo.to_string()),
                lancedb_uri,
                lancedb_table,
                e5_model_repo,
                prompt_template,
                Some(device.to_string()),
            )
            .await?
        }
        Some(("vectorize", sub_sub_matches)) => {
            let log_level = sub_sub_matches.get_one::<String>("loglevel");

            let lancedb_uri = sub_sub_matches
                .get_one::<String>("lancedb_uri")
                .expect("required");

            let lancedb_table = sub_sub_matches
                .get_one::<String>("lancedb_table")
                .expect("required");

            let e5_model_repo = sub_sub_matches
                .get_one::<String>("e5_model_repo")
                .expect("required");

            vectorize(log_level, lancedb_uri, lancedb_table, e5_model_repo).await?
        }
        Some(("apply", sub_sub_matches)) => {
            let log_level = sub_sub_matches.get_one::<String>("loglevel");
            let file = sub_sub_matches.get_one::<String>("file").expect("required");
            start_cluster(file).await?
        }

        _ => unreachable!(),
    }

    Ok(())
}
