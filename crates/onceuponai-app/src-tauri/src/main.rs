// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use once_cell::sync::OnceCell;
use onceuponai_core::common::ResultExt;
use server::{TauriAppConfig, TauriAppState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Manager, State, WindowEvent};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::mpsc::Receiver;
use uuid::timestamp::UUID_TICKS_BETWEEN_EPOCHS;
use uuid::Uuid;

pub mod server;

#[derive(Debug)]
struct SpawnedActor {
    rx: Receiver<CommandEvent>,
    child: CommandChild,
}

pub static SPAWNED_ACTORS: OnceCell<Arc<Mutex<HashMap<Uuid, SpawnedActor>>>> = OnceCell::new();

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust1!", name)
}

#[tauri::command]
async fn spawn_actor(app: tauri::AppHandle) {
    let sidecar_command = app
        .shell()
        .sidecar("binaries/sidecar/onceuponai-actors-candle")
        .unwrap()
        .args([
            "apply",
            "-f",
            "/home/jovyan/rust-src/onceuponai/examples/bielik.yaml",
        ]);
    let (rx, child) = sidecar_command.spawn().unwrap();
    SPAWNED_ACTORS
        .get()
        .expect("SPAWNED_ACTORS")
        .lock()
        .unwrap()
        .insert(Uuid::new_v4(), SpawnedActor { rx, child });
}

#[tauri::command]
fn config(handle: AppHandle) -> TauriAppConfig {
    let state: State<TauriAppState> = handle.state();
    let config = state.config.lock().unwrap();
    TauriAppConfig {
        personal_token: config.personal_token.clone(),
        base_url: config.base_url.clone(),
    }
}

fn main() {
    SPAWNED_ACTORS
        .set(Arc::new(Mutex::new(HashMap::new())))
        .unwrap();

    tauri::Builder::default()
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
        .invoke_handler(tauri::generate_handler![greet, config, spawn_actor])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Prevent the window from closing immediately
                api.prevent_close();
                // Call the cleanup function
                clean_up_resources().unwrap();
                // Then close the window
                window.close().unwrap();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
