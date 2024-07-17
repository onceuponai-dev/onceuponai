use crate::actors::{
    main_actor::{MainActor, MainActorSpec},
    ActorBuilder, ActorInstance,
};
use actix::prelude::*;
use actix_telepathy::Cluster;
use anyhow::Result;

pub async fn start_cluster(file: &String) -> Result<Option<(MainActorSpec, Addr<MainActor>)>> {
    let actor = ActorBuilder::build(file).await?;

    println!("{}", LOGO);
    match actor {
        ActorInstance::Main(main_actor) => {
            let _ = Cluster::new(main_actor.own_addr, Vec::new());
            let spec = main_actor.actor.spec();
            let addr = main_actor.start();

            // serve(spec, addr).await?;

            Ok(Some((spec, addr)))
        }
        ActorInstance::Worker(worker_actor) => {
            env_logger::init();
            let _ = Cluster::new(worker_actor.own_addr, vec![worker_actor.seed_addr]);
            worker_actor.start();
            tokio::signal::ctrl_c().await?;
            println!("Ctrl-C received, shutting down");
            Ok(None)
        }
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
