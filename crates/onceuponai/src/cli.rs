use anyhow::Result;
use clap::{arg, Command};
use onceuponai_actors::cluster::start_cluster;

use crate::serve::serve;

fn cli() -> Command {
    Command::new("onceuponai")
        .about("onceuponai")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("apply")
                .about("apply")
                .args(vec![arg!(--file <FILE> "file")
                    .required(true)
                    .short('f')
                    .help("file")])
                .arg_required_else_help(true),
        )
}

pub(crate) async fn commands() -> Result<()> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("apply", sub_sub_matches)) => {
            let file = sub_sub_matches.get_one::<String>("file").expect("required");
            let res = start_cluster(file).await?;
            match res {
                Some((spec, addr)) => serve(spec, addr).await?,
                None => (),
            };
        }
        _ => unreachable!(),
    }

    Ok(())
}
