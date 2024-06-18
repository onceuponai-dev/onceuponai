use crate::actors::main_actor::MainActor;
use actix::prelude::*;
use actix_telepathy::{Cluster, RemoteActor, RemoteAddr};
use anyhow::Result;
use log::debug;
use std::net::SocketAddr;
use tokio::sync::mpsc::channel;
use uuid::Uuid;

pub async fn start_cluster(host: &SocketAddr, seed: Option<&SocketAddr>) -> Result<()> {
    env_logger::init();
    let seed_nodes = if let Some(seed) = seed {
        vec![*seed]
    } else {
        vec![]
    };
    let _ = Cluster::new(*host, seed_nodes);

    if seed.is_some() {
        let remote_addr = RemoteAddr::new_from_id(*host, MainActor::ACTOR_ID);
        let uuid = Uuid::new_v4();
        let _ = MainActor {
            uuid,
            remote_addr,
            own_addr: *host,
            models: Vec::new(),
        }
        .start();
    } else {
        todo!()
    }

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    //System::current().stop();
    Ok(())
}
