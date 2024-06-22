use crate::actors::{ActorBuilder, ActorInstance};
use actix::prelude::*;
use actix_telepathy::Cluster;
use anyhow::Result;

pub async fn start_cluster(file: &String) -> Result<()> {
    env_logger::init();
    let actor = ActorBuilder::build(file).await?;

    match actor {
        ActorInstance::Main(main_actor) => {
            let _ = Cluster::new(main_actor.own_addr, Vec::new());
            main_actor.start();
        }
        ActorInstance::Worker(worker_actor) => {
            let _ = Cluster::new(worker_actor.own_addr, vec![worker_actor.seed_addr]);
            worker_actor.start();
        }
    }

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    //System::current().stop();
    Ok(())
}
