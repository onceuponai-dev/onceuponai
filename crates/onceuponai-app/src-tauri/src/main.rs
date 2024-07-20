// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use server::TauriAppState;
use std::thread;
use tauri::{AppHandle, Manager, State};

pub mod server;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust1!", name)
}

#[tauri::command]
fn config(handle: AppHandle) -> TauriAppState {
    let state: State<TauriAppState> = handle.state();
    TauriAppState {
        personal_token: state.personal_token.clone(),
        base_url: state.base_url.clone(),
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            let box_handle = Box::new(handle);
            thread::spawn(move || {
                server::init(*box_handle).unwrap();
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
