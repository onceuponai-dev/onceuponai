// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use once_cell::sync::OnceCell;
use onceuponai_actors::abstractions::ActorMetadata;
use onceuponai_core::common::ResultExt;
use serde_json::{json, Value};
use server::{TauriAppConfig, TauriAppState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Manager, RunEvent, State};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

pub mod server;

#[derive(Debug)]
struct SpawnedActor {
    rx: Receiver<CommandEvent>,
    child: CommandChild,
}

static SPAWNED_ACTORS: OnceCell<Arc<Mutex<HashMap<Uuid, SpawnedActor>>>> = OnceCell::new();

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust1!", name)
}

#[tauri::command]
fn kill_actor(sidecar_id: Uuid) {
    if let Some(actors_mutex) = SPAWNED_ACTORS.get() {
        let mut actors = actors_mutex.lock().map_anyhow_err().unwrap();
        actors
            .remove(&sidecar_id)
            .expect("SPAWNED_ACTOR")
            .child
            .kill()
            .unwrap();
    }
}

#[tauri::command]
async fn spawn_actor(app: tauri::AppHandle) -> Result<Value, ()> {
    // let metadata = ActorMetadata {
    //     name: todo!(),
    //     features: todo!(),
    //     actor_id: todo!(),
    //     actor_host: todo!(),
    //     actor_seed: todo!(),
    //     sidecar_id: todo!(),
    // };

    let sidecar_command = app
        .shell()
        .sidecar("onceuponai-actors-candle")
        .unwrap()
        .args([
            "apply",
            "-f",
            "/home/jovyan/rust-src/onceuponai/examples/bielik.yaml",
        ]);
    let (rx, child) = sidecar_command.spawn().unwrap();
    let sidecar_id = Uuid::new_v4();
    SPAWNED_ACTORS
        .get()
        .expect("SPAWNED_ACTORS")
        .lock()
        .unwrap()
        .insert(sidecar_id, SpawnedActor { rx, child });

    Ok(json!({"sidecar_id": sidecar_id}))
}

#[tauri::command]
fn config(handle: AppHandle) -> TauriAppConfig {
    let state: State<TauriAppState> = handle.state();
    let config = state.config.lock().unwrap();
    TauriAppConfig {
        personal_token: config.personal_token.clone(),
        base_url: config.base_url.clone(),
        actor_seed: String::from(""),
    }
}

fn main() {
    SPAWNED_ACTORS
        .set(Arc::new(Mutex::new(HashMap::new())))
        .unwrap();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let config = Arc::new(Mutex::new(TauriAppConfig::default()));
            let shared_config = Arc::clone(&config);
            app.manage(TauriAppState { config });
            thread::spawn(move || {
                server::init(shared_config).unwrap();
            });

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            config,
            spawn_actor,
            kill_actor
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
