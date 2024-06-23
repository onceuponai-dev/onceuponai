use actix_web::{HttpResponse, Responder};
pub mod actors;
pub mod auth;
pub mod chat;
pub mod users;

pub const ASSETS_CSS_HASH: &str = "4a47f803";
pub const ASSETS_JS_HASH: &str = "e1b29641";

const INDEX_HTML: &str = include_str!("../../ui/index.html");
const ASSETS_JS: &str = include_str!("../../ui/assets/index-e1b29641.js");
const ASSETS_CSS: &str = include_str!("../../ui/assets/index-4a47f803.css");

pub async fn index_html() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(INDEX_HTML)
}

pub async fn assets_css() -> impl Responder {
    HttpResponse::Ok().content_type("text/css").body(ASSETS_CSS)
}

pub async fn assets_js() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/javascript")
        .body(ASSETS_JS)
}

pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}
