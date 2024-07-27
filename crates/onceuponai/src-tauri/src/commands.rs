use crate::{
    server::{TauriAppConfig, TauriAppState},
    SpawnedActor, SPAWNED_ACTORS,
};
use futures::StreamExt;
use onceuponai_actors::abstractions::ActorMetadata;
use onceuponai_core::{
    common::{serialize_and_encode, ResultExt, SerializationType},
    notifications::{Notification, NotificationLevel},
};
use onceuponai_server::handlers::oai::ChatCompletionsRequest;
use reqwest::Client;
use serde_json::{json, Value};
use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager, State};
use tauri_plugin_shell::{process::CommandEvent, ShellExt};
use uuid::Uuid;

#[tauri::command]
pub fn actors_gallery(handle: tauri::AppHandle) -> Result<String, String> {
    let resource_path = handle
        .path()
        .resolve("resources/actors_gallery.json", BaseDirectory::Resource)
        .map_str_err()?;

    std::fs::read_to_string(resource_path).map_str_err()
}

#[tauri::command]
pub fn kill_actor(sidecar_id: Uuid) -> Result<(), String> {
    if let Some(actors_mutex) = SPAWNED_ACTORS.get() {
        let mut actors = actors_mutex.lock().map_str_err()?;
        actors
            .remove(&sidecar_id)
            .expect("SPAWNED_ACTOR")
            .child
            .kill()
            .map_str_err()?;
    }

    Ok(())
}

#[tauri::command]
pub async fn spawn_actor(
    app: tauri::AppHandle,
    name: String,
    device: String,
    spec_json_base64: String,
) -> Result<Value, String> {
    let a = app.clone();
    let state: State<TauriAppState> = a.state::<TauriAppState>();
    let mut config = state.config.lock().map_str_err()?;
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
    let metadata = serialize_and_encode(metadata, SerializationType::YAML).map_str_err()?;

    let sidecar_command = app
        .shell()
        .sidecar(format!("onceuponai-actors-candle-{}", device))
        .map_str_err()?
        .args(["spawn", "-j", &spec_json_base64, "-m", &metadata]);
    let (mut rx, child) = sidecar_command.spawn().map_str_err()?;

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
        .map_str_err()?
        .insert(sidecar_id, SpawnedActor { child });

    Ok(json!({"sidecar_id": sidecar_id}))
}

#[tauri::command]
pub fn config(handle: AppHandle) -> Result<TauriAppConfig, String> {
    let state: State<TauriAppState> = handle.state();
    let config = state.config.lock().map_str_err()?;
    Ok(TauriAppConfig {
        personal_token: config.personal_token.clone(),
        base_url: config.base_url.clone(),
        actor_seed: config.actor_seed.clone(),
        actor_base_host: "".to_string(),
        actor_next_port: 0,
    })
}

#[tauri::command]
pub async fn v1_chat_completions(
    handle: AppHandle,
    base_url: String,
    personal_token: String,
    chat_request: ChatCompletionsRequest,
) -> Result<(), String> {
    let client = Client::new();
    let mut stream = client
        .post(format!("{}/v1/chat/completions", base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", personal_token))
        .json(&chat_request)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes_stream();

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                let message = String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())?;
                handle
                    .emit("v1-chat-completions", message)
                    .map_err(|e| e.to_string())?;
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
