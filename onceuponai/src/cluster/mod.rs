use crate::actors::{main_actor::MainActor, ActorMetadata, ActorObject, ActorWrapper, GemmaConfig};
use actix::prelude::*;
use actix_telepathy::{Cluster, RemoteActor, RemoteAddr};
use anyhow::Result;
use log::debug;
use std::net::SocketAddr;
use uuid::Uuid;

pub async fn start_cluster(host: &SocketAddr, seed: Option<&SocketAddr>) -> Result<()> {
    env_logger::init();
    let seed_nodes = if let Some(seed) = seed {
        vec![*seed]
    } else {
        vec![]
    };
    let _ = Cluster::new(*host, seed_nodes);
    let uuid = Uuid::new_v4();

    if seed.is_none() {
        let remote_addr = RemoteAddr::new_from_id(*host, MainActor::ACTOR_ID);
        debug!("START MAIN_ACTOR {:?}", remote_addr);
        let _ = MainActor {
            uuid,
            remote_addr,
            own_addr: *host,
            models: Vec::new(),
        }
        .start();
    } else {
        let remote_addr = RemoteAddr::new_from_id(*host, ActorWrapper::ACTOR_ID);
        debug!("START MODEL_ACTOR {:?}", remote_addr);
        let _ = ActorWrapper {
            remote_addr,
            actor: ActorObject::Gemma {
                metadata: ActorMetadata {
                    name: "TEST".to_string(),
                },
                spec: GemmaConfig {},
            },
            uuid,
        }
        .start();
    }

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    //System::current().stop();
    Ok(())
}
