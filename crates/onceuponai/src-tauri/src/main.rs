// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod commands;
pub mod server;
use anyhow::Result;
use clap::Parser;
use commands::{actors_gallery, config, init_actor, kill_actor, spawn_actor, v1_chat_completions};
use once_cell::sync::OnceCell;
use onceuponai_core::common::ResultExt;
use serde::{Deserialize, Serialize};
use server::{TauriAppConfig, TauriAppState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{Manager, RunEvent};
use tauri_plugin_shell::process::CommandChild;
use uuid::Uuid;

#[derive(Debug)]
struct SpawnedActor {
    child: CommandChild,
}

static SPAWNED_ACTORS: OnceCell<Arc<Mutex<HashMap<Uuid, SpawnedActor>>>> = OnceCell::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct SpawnActorRequest {
    pub name: String,
    pub spec_json_base64: String,
}

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about)]
pub struct MainArgs {
    #[clap(long, default_value = "127.0.0.1:1992")]
    actor_host: String,
    #[clap(long, default_value = "0.0.0.0")]
    host: String,
    #[clap(long, default_value = "8080")]
    port: u16,
    #[clap(long, default_value = "info")]
    log_level: String,
    #[clap(long, default_value = "0")]
    workers: usize,
    #[clap(long, default_value = "60")]
    invoke_timeout: u64,
    #[clap(long)]
    session_key: Option<String>,
    #[clap(long)]
    personal_access_token_secret: Option<String>,
    #[clap(long, default_value_t = false)]
    headless: bool,
    #[clap(long, default_value_t = false)]
    oidc: bool,
    #[clap(long)]
    oidc_issuer_url: Option<String>,
    #[clap(long)]
    oidc_client_id: Option<String>,
    #[clap(long)]
    oidc_client_secret: Option<String>,
    #[clap(long)]
    oidc_redirect_url: Option<String>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    SPAWNED_ACTORS
        .set(Arc::new(Mutex::new(HashMap::new())))
        .unwrap();
    let args = MainArgs::parse();

    if args.headless {
        thread::spawn(move || {
            server::init(None, args).unwrap();
        });
        tokio::signal::ctrl_c().await?;
        println!("Ctrl-C received, shutting down");
    } else {
        let app = tauri::Builder::default()
            .plugin(tauri_plugin_process::init())
            .setup(|app| {
                let config = Arc::new(Mutex::new(TauriAppConfig::default()));
                let shared_config = Arc::clone(&config);
                app.manage(TauriAppState { config });
                thread::spawn(move || {
                    server::init(Some(shared_config), args).unwrap();
                });

                Ok(())
            })
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_http::init())
            .invoke_handler(tauri::generate_handler![
                config,
                init_actor,
                spawn_actor,
                kill_actor,
                actors_gallery,
                v1_chat_completions
            ])
            .build(tauri::generate_context!())
            .expect("error while running tauri application");

        app.run(|_app, event| {
            if let RunEvent::ExitRequested { api: _, .. } = event {
                // api.prevent_exit();
                clean_up_resources().unwrap();
            }
        });
    }

    Ok(())
}

fn clean_up_resources() -> Result<()> {
    if let Some(actors_mutex) = SPAWNED_ACTORS.get() {
        let mut actors = actors_mutex.lock().map_anyhow_err()?;

        for (_uuid, actor) in actors.drain() {
            actor.child.kill()?;
        }
    }
    println!("Cleaning up resources...");
    Ok(())
}
