use crate::{
    actors::{ActorBuilder, ActorInstance, ActorObject},
    serve::serve,
};
use actix::prelude::*;
use actix_telepathy::Cluster;
use anyhow::Result;

pub async fn start_cluster(file: &String) -> Result<()> {
    let actor = ActorBuilder::build(file).await?;

    match actor {
        ActorInstance::Main(main_actor) => {
            let _ = Cluster::new(main_actor.own_addr, Vec::new());
            let spec = match main_actor.actor.clone() {
                ActorObject::Main { metadata: _, spec } => spec,
                _ => todo!(),
            };
            main_actor.start();
            serve(spec).await?;
        }
        ActorInstance::Worker(worker_actor) => {
            let _ = Cluster::new(worker_actor.own_addr, vec![worker_actor.seed_addr]);
            worker_actor.start();
            tokio::signal::ctrl_c().await.unwrap();
            println!("Ctrl-C received, shutting down");
        }
    }

    //System::current().stop();
    Ok(())
}
