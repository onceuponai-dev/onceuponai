use crate::actors::main_actor::{InvokeTask, CONNECTED_ACTORS, INVOKE_TASKS};
use crate::actors::ActorStartInvokeRequest;
use crate::models::InvokeRequest;
use crate::serve::AppState;
use actix_web::Responder;
use actix_web::{web, HttpRequest, HttpResponse};
use anyhow::Result;
use async_stream::stream;
use onceuponai_core::common::ResultExt;
use serde_json::json;
use std::error::Error;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub async fn connected_actors(_req: HttpRequest) -> Result<impl Responder, Box<dyn Error>> {
    let connected_actors = crate::actors::main_actor::CONNECTED_ACTORS
        .get()
        .expect("CONNECTED_ACTORS")
        .lock()
        .map_box_err()?;

    let keys = connected_actors.clone();
    Ok(HttpResponse::Ok().json(keys.clone()))
}

pub async fn invoke(
    req: HttpRequest,
    invoke_request: web::Json<InvokeRequest>,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let kind = req
        .match_info()
        .get("kind")
        .expect("KIND")
        .to_string()
        .to_lowercase();

    let name = req
        .match_info()
        .get("name")
        .expect("NAME")
        .to_string()
        .to_lowercase();

    base_invoke(kind, name, app_state, invoke_request.clone()).await
}

pub async fn base_invoke(
    kind: String,
    name: String,
    app_state: web::Data<AppState>,
    invoke_request: InvokeRequest,
) -> Result<impl Responder, Box<dyn Error>> {
    let kind_connected = CONNECTED_ACTORS
        .get()
        .expect("CONNECTED_MODELS")
        .lock()
        .unwrap()
        .iter()
        .any(|a| a.1.kind == kind && a.1.metadata.name == name);

    if !kind_connected {
        return Ok(HttpResponse::NotFound().body(format!(
            "ACTOR WITH KIND: {kind:?} NAME: {name:?} NOT CONNECTED"
        )));
    }

    let task_id = Uuid::new_v4();
    let (tx, rx) = mpsc::channel();

    {
        let mut response_map = INVOKE_TASKS.get().expect("INVOKE_TASKS").lock()?;
        response_map.insert(
            task_id,
            InvokeTask {
                time: Instant::now(),
                sender: tx,
            },
        );
    }

    let stream = invoke_request.stream.unwrap_or_default();

    app_state.addr.do_send(ActorStartInvokeRequest {
        task_id,
        kind,
        name,
        stream,
        data: invoke_request.data,
    });

    if !stream {
        let invoke_timeout = app_state.spec.invoke_timeout.unwrap_or(5u64);
        match rx.recv_timeout(Duration::from_secs(invoke_timeout)) {
            Ok(response) => {
                let mut response_map = INVOKE_TASKS.get().expect("INVOKE_TASKS").lock()?;
                response_map.remove(&task_id);
                match response {
                    crate::actors::ActorInvokeResponse::Success(result) => {
                        Ok(HttpResponse::Ok().json(result.data))
                    }
                    crate::actors::ActorInvokeResponse::Failure(result) => {
                        Ok(HttpResponse::BadRequest().json(result.error))
                    }
                    crate::actors::ActorInvokeResponse::Finish(_) => {
                        Ok(HttpResponse::Ok().body(""))
                    }
                }
            }
            Err(_) => {
                let mut response_map = INVOKE_TASKS.get().expect("INVOKE_TASKS").lock()?;
                response_map.remove(&task_id);
                Ok(HttpResponse::InternalServerError()
                    .body(format!("Request timeout ( > {invoke_timeout:?} s)")))
            }
        }
    } else {
        let rx = Arc::new(Mutex::new(rx));
        let stream_tasks = stream! {
            let mut finished = false;

            let invoke_timeout = app_state.spec.invoke_timeout.unwrap_or(5u64);
            while !finished {
                tokio::task::yield_now().await;
                match rx.lock().unwrap().recv_timeout(Duration::from_secs(invoke_timeout)) {
                    Ok(response) => {
                        match response {
                            crate::actors::ActorInvokeResponse::Success(result) => {
                                let text = json!(result.data).to_string();
                                let byte = bytes::Bytes::from(text);
                                yield Ok::<bytes::Bytes, Box<dyn std::error::Error>>(byte);
                            }
                            crate::actors::ActorInvokeResponse::Failure(result) => {
                                let text = json!(result.error).to_string();
                                let byte = bytes::Bytes::from(text);
                                finished = true;
                                yield Ok::<bytes::Bytes, Box<dyn std::error::Error>>(byte);
                            }
                            crate::actors::ActorInvokeResponse::Finish(_) => {
                                finished = true;
                            }
                        }
                    }
                    Err(e) => {
                        let text = format!("{}", e);
                        //let text = format!("Request timeout ( > {invoke_timeout:?} s)");
                        let byte = bytes::Bytes::from(text);
                        finished = true;
                        yield Ok::<bytes::Bytes, Box<dyn std::error::Error>>(byte);
                    }
                }
            }
        };

        let mut response_map = INVOKE_TASKS.get().expect("INVOKE_TASKS").lock()?;
        response_map.remove(&task_id);

        Ok(HttpResponse::Ok()
            .content_type("text/event-stream")
            .streaming(Box::pin(stream_tasks)))
    }
}
