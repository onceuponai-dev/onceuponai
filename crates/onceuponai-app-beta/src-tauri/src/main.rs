// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use server::{TauriAppConfig, TauriAppState};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Manager, State};

pub mod server;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust1!", name)
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
    tauri::Builder::default()
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
        .invoke_handler(tauri::generate_handler![greet, config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
