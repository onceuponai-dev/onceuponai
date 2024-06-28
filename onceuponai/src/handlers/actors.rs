use crate::actors::main_actor::{InvokeTask, CONNECTED_ACTORS, INVOKE_TASKS};
use crate::actors::ActorStartInvokeRequest;
use crate::models::InvokeRequest;
use crate::serve::AppState;
use actix_web::Responder;
use actix_web::{web, HttpRequest, HttpResponse};
use anyhow::Result;
use async_stream::stream;
use futures::stream::Stream;
use futures::task::{Context, Poll};
use log::debug;
use onceuponai_core::common::ResultExt;
use serde_json::json;
use std::error::Error;
use std::pin::Pin;
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

        let stream = MpscStream {
            receiver: rx,
            task_id,
        };
        Ok(HttpResponse::Ok().streaming(stream))
    }
}

struct MpscStream {
    receiver: Arc<Mutex<mpsc::Receiver<crate::actors::ActorInvokeResponse>>>,
    task_id: Uuid,
}

impl Stream for MpscStream {
    type Item = Result<bytes::Bytes, actix_web::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let receiver = self.receiver.lock().unwrap();
        match receiver.try_recv() {
            Ok(response) => match response {
                crate::actors::ActorInvokeResponse::Success(result) => {
                    let text = json!(result.data).to_string();

                    let byte = bytes::Bytes::from(text);
                    Poll::Ready(Some(Ok(byte)))
                }
                crate::actors::ActorInvokeResponse::Failure(result) => {
                    let text = json!(result.error).to_string();
                    debug!("ERROR {text:?}");
                    let mut response_map =
                        INVOKE_TASKS.get().expect("INVOKE_TASKS").lock().unwrap();
                    response_map.remove(&self.task_id);

                    Poll::Ready(None)
                }
                crate::actors::ActorInvokeResponse::Finish(_) => {
                    let mut response_map =
                        INVOKE_TASKS.get().expect("INVOKE_TASKS").lock().unwrap();
                    response_map.remove(&self.task_id);

                    Poll::Ready(None)
                }
            },

            Err(mpsc::TryRecvError::Empty) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                let mut response_map = INVOKE_TASKS.get().expect("INVOKE_TASKS").lock().unwrap();
                response_map.remove(&self.task_id);

                Poll::Ready(None)
            }
        }
    }
}
