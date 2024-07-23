use crate::{
    abstractions::{ActorKindActions, ActorMetadata, ActorObject},
    actors::{
        main_actor::{MainActor, MainActorSpec},
        ActorBuilder, WorkerActor,
    },
};
use actix::prelude::*;
use actix_telepathy::Cluster;
use anyhow::{anyhow, Result};
use onceuponai_core::{
    common::{decode_and_deserialize, ResultExt, SerializationType},
    config::read_config_str,
};
use serde::de::DeserializeOwned;

pub fn start_main_actor(main_actor: MainActor) -> Result<Option<(MainActorSpec, Addr<MainActor>)>> {
    println!("{}", LOGO);
    let _ = Cluster::new(main_actor.own_addr, Vec::new());
    let spec = main_actor.actor.spec();
    let addr = main_actor.start();
    Ok(Some((spec, addr)))
}

pub async fn start_worker_actor(
    worker_actor: WorkerActor,
) -> Result<Option<(MainActorSpec, Addr<MainActor>)>> {
    // println!("{}", LOGO);
    env_logger::init();
    let _ = Cluster::new(worker_actor.own_addr, vec![worker_actor.seed_addr]);
    worker_actor.start();
    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, shutting down");
    Ok(None)
}

pub async fn start_main_cluster(
    file: &String,
) -> Result<Option<(MainActorSpec, Addr<MainActor>, ActorMetadata)>> {
    let configuration_str = read_config_str(file, Some(true)).await.map_anyhow_err()?;
    let actor: ActorObject<MainActorSpec> = serde_yaml::from_str(&configuration_str)?;
    let metadata = actor.metadata();
    let main_actor = ActorBuilder::build_main(actor)?;
    let res = start_main_actor(main_actor)?.expect("MAIN_ACTOR_SPEC");
    //System::current().stop();
    Ok(Some((res.0, res.1, metadata)))
}

pub async fn start_worker_cluster<T: ActorKindActions + DeserializeOwned>(
    file: Option<&String>,
    yaml: Option<&String>,
    json: Option<&String>,
    metadata_yaml: Option<&String>,
) -> Result<()> {
    let actor_kind: T = if let Some(f) = file {
        let configuration_str = read_config_str(f, Some(true)).await.map_anyhow_err()?;
        serde_yaml::from_str(&configuration_str)?
    } else if let Some(y) = yaml {
        decode_and_deserialize(y, SerializationType::YAML)?
    } else if let Some(j) = json {
        decode_and_deserialize(j, SerializationType::JSON)?
    } else {
        return Err(anyhow!("Wrong worker actor configuration"));
    };

    let metadata = if let Some(m) = metadata_yaml {
        decode_and_deserialize(m, SerializationType::YAML)?
    } else {
        actor_kind.metadata()
    };

    let worker_actor = ActorBuilder::build_worker(metadata, || actor_kind.actor())?;
    start_worker_actor(worker_actor).await?;
    Ok(())
}

const LOGO: &str = r#"
 ██████╗ ███╗   ██╗ ██████╗███████╗    ██╗   ██╗██████╗  ██████╗ ███╗   ██╗                  █████╗ ██╗
██╔═══██╗████╗  ██║██╔════╝██╔════╝    ██║   ██║██╔══██╗██╔═══██╗████╗  ██║                 ██╔══██╗██║
██║   ██║██╔██╗ ██║██║     █████╗      ██║   ██║██████╔╝██║   ██║██╔██╗ ██║                 ███████║██║
██║   ██║██║╚██╗██║██║     ██╔══╝      ██║   ██║██╔═══╝ ██║   ██║██║╚██╗██║                 ██╔══██║██║
╚██████╔╝██║ ╚████║╚██████╗███████╗    ╚██████╔╝██║     ╚██████╔╝██║ ╚████║    ██╗██╗██╗    ██║  ██║██║
 ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝╚══════╝     ╚═════╝ ╚═╝      ╚═════╝ ╚═╝  ╚═══╝    ╚═╝╚═╝╚═╝    ╚═╝  ╚═╝╚═╝
"#;
