use actix_web::{HttpResponse, Responder};
pub mod auth;
pub mod chat;

const INDEX_HTML: &str = include_str!("../../public/index.html");

pub async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(INDEX_HTML)
}

pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}
