use crate::actors::main_actor::{
    InvokeTask, MainActor, MainActorConfig, CONNECTED_ACTORS, INVOKE_TASKS,
};
use crate::actors::ActorStartInvokeRequest;
use crate::config::Config;
use crate::handlers::chat::chat;
use crate::handlers::{self, health};
use actix::Addr;
use actix_files as fs;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::middleware::Logger;
use actix_web::Responder;
use actix_web::{cookie::Key, web, App, HttpRequest, HttpResponse, HttpServer};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use num_traits::Zero;
use onceuponai_core::common::ResultExt;
use onceuponai_core::common_models::EntityValue;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use uuid::Uuid;

fn get_secret_key() -> Result<Key> {
    let key = Config::get().session_key.to_string();
    let k = general_purpose::STANDARD.decode(key)?;
    Ok(Key::from(&k))
}

pub struct AppState {
    pub addr: Addr<MainActor>,
    pub spec: MainActorConfig,
}

#[derive(Deserialize)]
pub struct InvokeRequest {
    data: HashMap<String, Vec<EntityValue>>,
}

// Handler for getting a session value
async fn get_session(
    _req: HttpRequest,
    session: actix_session::Session,
) -> Result<impl Responder, Box<dyn Error>> {
    let user_id: Option<String> = session.get("EMAIL")?;
    if let Some(user_id) = user_id {
        Ok(HttpResponse::Ok().body(format!("User ID: {}", user_id)))
    } else {
        Ok(HttpResponse::Ok().body("No session found"))
    }
}

async fn connected_actors(_req: HttpRequest) -> Result<impl Responder, Box<dyn Error>> {
    let connected_actors = crate::actors::main_actor::CONNECTED_ACTORS
        .get()
        .expect("CONNECTED_ACTORS")
        .lock()
        .map_box_err()?;

    let keys = connected_actors.clone();
    Ok(HttpResponse::Ok().json(keys.clone()))
}

async fn invoke(
    req: HttpRequest,
    invoke_request: web::Json<HashMap<String, Vec<EntityValue>>>,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let kind = req
        .match_info()
        .get("kind")
        .expect("KIND")
        .to_string()
        .to_lowercase();

    base_invoke(kind, app_state, invoke_request.clone()).await
}

async fn base_invoke(
    kind: String,
    app_state: web::Data<AppState>,
    data: HashMap<String, Vec<EntityValue>>,
) -> Result<impl Responder, Box<dyn Error>> {
    let task_id = Uuid::new_v4();
    let (tx, rx) = mpsc::channel();

    let kind_connected = CONNECTED_ACTORS
        .get()
        .expect("CONNECTED_MODELS")
        .lock()
        .unwrap()
        .iter()
        .any(|a| a.1.kind == kind);

    if !kind_connected {
        return Ok(
            HttpResponse::NotFound().body(format!("ACTOR WITH KIND: {kind:?} NOT CONNECTED"))
        );
    }

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

    app_state.addr.do_send(ActorStartInvokeRequest {
        task_id,
        kind,
        data,
    });

    match rx.recv_timeout(Duration::from_secs(
        app_state.spec.invoke_timeout.unwrap_or(5u64),
    )) {
        Ok(response) => {
            let mut response_map = INVOKE_TASKS.get().expect("INVOKE_TASKS").lock()?;
            response_map.remove(&task_id);
            match response {
                crate::actors::ActorInvokeResponse::Success(result) => {
                    Ok(HttpResponse::Ok().json(result.data))
                }
                crate::actors::ActorInvokeResponse::Failure(error) => {
                    Ok(HttpResponse::BadRequest().json(error))
                }
            }
        }
        Err(_) => {
            let mut response_map = INVOKE_TASKS.get().expect("INVOKE_TASKS").lock()?;
            response_map.remove(&task_id);
            Ok(HttpResponse::InternalServerError().body("Error processing request"))
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn serve(spec: MainActorConfig, addr: Addr<MainActor>) -> std::io::Result<()> {
    let sp = spec.clone();
    if let Some(v) = spec.log_level {
        env_logger::init_from_env(env_logger::Env::new().default_filter_or(v));
    }

    let secret_key = get_secret_key().map_io_err()?;
    println!(
        "Server running on http://{}:{}",
        spec.server_host, spec.server_port
    );
    let mut server = HttpServer::new(move || {
        let mut app = App::new()
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .wrap(Logger::default())
            .route("/health", web::get().to(health))
            .route("/get", web::get().to(get_session))
            .route("/actors", web::get().to(connected_actors))
            .route("/invoke/{kind}", web::post().to(invoke))
            .route("/auth", web::get().to(handlers::auth::auth))
            .route(
                "/auth-callback",
                web::get().to(handlers::auth::auth_callback),
            )
            .app_data(web::Data::new(AppState {
                addr: addr.clone(),
                spec: sp.clone(),
            }));

        let mut llm_scope = web::scope("/llm");
        llm_scope = llm_scope.route("/chat", web::get().to(chat));

        app = app.service(llm_scope);

        app.service(fs::Files::new("/", "../onceuponai-ui/dist/").show_files_listing())
    });

    if let Some(num_workers) = spec.workers {
        if !num_workers.is_zero() {
            server = server.workers(num_workers);
        }
    }

    server
        .bind((spec.server_host, spec.server_port))?
        .run()
        .await
}
