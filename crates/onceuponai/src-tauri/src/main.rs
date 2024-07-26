// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use once_cell::sync::OnceCell;
use onceuponai_actors::abstractions::ActorMetadata;
use onceuponai_core::common::{serialize_and_encode, ResultExt, SerializationType};
use onceuponai_core::notifications::{Notification, NotificationLevel};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use server::{TauriAppConfig, TauriAppState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use uuid::Uuid;

pub mod server;

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

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust1!", name)
}

#[tauri::command]
fn actors_gallery(handle: tauri::AppHandle) -> Result<String, ()> {
    let resource_path = handle
        .path()
        .resolve("resources/actors_gallery.json", BaseDirectory::Resource)
        .unwrap();

    let file = std::fs::read_to_string(resource_path).unwrap();
    Ok(file)
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
async fn spawn_actor(
    app: tauri::AppHandle,
    name: String,
    device: String,
    spec_json_base64: String,
) -> Result<Value, ()> {
    let a = app.clone();
    let state: State<TauriAppState> = a.state::<TauriAppState>();
    let mut config = state.config.lock().unwrap();
    config.actor_next_port += 1;
    let sidecar_id = Uuid::new_v4();
    let metadata = ActorMetadata {
        name,
        features: None,
        actor_id: None,
        actor_host: format!("{}:{}", config.actor_base_host, config.actor_next_port),
        actor_seed: Some(config.actor_seed.clone()),
        sidecar_id: Some(sidecar_id),
    };
    let metadata = serialize_and_encode(metadata, SerializationType::YAML).unwrap();

    let sidecar_command = app
        .shell()
        .sidecar(format!("onceuponai-actors-candle-{}", device))
        .unwrap()
        .args(["spawn", "-j", &spec_json_base64, "-m", &metadata]);
    let (mut rx, child) = sidecar_command.spawn().unwrap();

    tauri::async_runtime::spawn(async move {
        while let Some(message) = rx.recv().await {
            match message {
                CommandEvent::Stderr(buf) => {
                    let text = std::str::from_utf8(&buf).unwrap();
                    println!("STDERR {}", text);
                    let text = Notification::read(text);
                    if let Some(message) = text {
                        app.emit("message", message).unwrap();
                    }
                }
                CommandEvent::Stdout(buf) => {
                    let text = std::str::from_utf8(&buf).unwrap();
                    println!("STDOUT {}", text);
                    let text = Notification::read(text);
                    if let Some(message) = text {
                        app.emit("message", message).unwrap();
                    }
                }
                CommandEvent::Error(error) => {
                    println!("ERROR {}", &error);
                    app.emit("message", error).unwrap();
                }
                CommandEvent::Terminated(_) => {
                    println!("TERMINATED");
                    let message =
                        Notification::build("ACTOR TERMINATED", NotificationLevel::Info).unwrap();
                    app.emit("message", message).unwrap();
                }
                _ => println!("OTHER"),
            }
        }
    });

    SPAWNED_ACTORS
        .get()
        .expect("SPAWNED_ACTORS")
        .lock()
        .unwrap()
        .insert(sidecar_id, SpawnedActor { child });

    Ok(json!({"sidecar_id": sidecar_id}))
}

#[tauri::command]
fn config(handle: AppHandle) -> TauriAppConfig {
    let state: State<TauriAppState> = handle.state();
    let config = state.config.lock().unwrap();
    TauriAppConfig {
        personal_token: config.personal_token.clone(),
        base_url: config.base_url.clone(),
        actor_seed: config.actor_seed.clone(),
        actor_base_host: "".to_string(),
        actor_next_port: 0,
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
            kill_actor,
            actors_gallery
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
