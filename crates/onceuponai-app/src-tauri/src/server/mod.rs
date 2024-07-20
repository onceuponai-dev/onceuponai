use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde_json::json;
use std::{io, sync::Mutex};
use tauri::AppHandle;

struct TauriAppState {
    app: Mutex<AppHandle>,
}

async fn index() -> impl Responder {
    HttpResponse::Ok().json(json!({"hello": "world"}))
}

pub fn init(app: AppHandle) -> io::Result<()> {
    let tauri_app = web::Data::new(TauriAppState {
        app: Mutex::new(app),
    });
    actix_rt::System::new().block_on(async {
        HttpServer::new(move || {
            App::new()
                .app_data(tauri_app.clone())
                .route("/api/hello", web::get().to(index))
        })
        .bind("0.0.0.0:8080")?
        .run()
        .await // Await the server future
    })
}
