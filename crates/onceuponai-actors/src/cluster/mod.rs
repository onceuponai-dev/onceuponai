use crate::actors::{
    main_actor::{MainActor, MainActorSpec},
    ActorBuilder, ActorInstance, WorkerActor,
};
use actix::prelude::*;
use actix_telepathy::Cluster;
use anyhow::Result;

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
    println!("{}", LOGO);
    env_logger::init();
    let _ = Cluster::new(worker_actor.own_addr, vec![worker_actor.seed_addr]);
    worker_actor.start();
    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, shutting down");
    Ok(None)
}

pub async fn start_cluster(file: &String) -> Result<Option<(MainActorSpec, Addr<MainActor>)>> {
    let actor = ActorBuilder::build(file).await?;
    match actor {
        ActorInstance::Main(main_actor) => start_main_actor(main_actor),
        ActorInstance::Worker(worker_actor) => start_worker_actor(worker_actor).await,
    }

    //System::current().stop();
}

const LOGO: &str = r#"
 ██████╗ ███╗   ██╗ ██████╗███████╗    ██╗   ██╗██████╗  ██████╗ ███╗   ██╗                  █████╗ ██╗
██╔═══██╗████╗  ██║██╔════╝██╔════╝    ██║   ██║██╔══██╗██╔═══██╗████╗  ██║                 ██╔══██╗██║
██║   ██║██╔██╗ ██║██║     █████╗      ██║   ██║██████╔╝██║   ██║██╔██╗ ██║                 ███████║██║
██║   ██║██║╚██╗██║██║     ██╔══╝      ██║   ██║██╔═══╝ ██║   ██║██║╚██╗██║                 ██╔══██║██║
╚██████╔╝██║ ╚████║╚██████╗███████╗    ╚██████╔╝██║     ╚██████╔╝██║ ╚████║    ██╗██╗██╗    ██║  ██║██║
 ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝╚══════╝     ╚═════╝ ╚═╝      ╚═════╝ ╚═╝  ╚═══╝    ╚═╝╚═╝╚═╝    ╚═╝  ╚═╝╚═╝
"#;
