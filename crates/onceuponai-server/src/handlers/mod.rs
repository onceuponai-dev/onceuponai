use actix_web::{http::header::ContentType, HttpResponse, Responder};
pub mod actors;
pub mod auth;
pub mod chat;
pub mod oai;
pub mod users;

pub const ASSETS_CSS_HASH: &str = "1e95b263";
pub const ASSETS_JS_HASH: &str = "4599dad9";

const INDEX_HTML: &str = include_str!("../../ui/index.html");
const ASSETS_JS: &str = include_str!("../../ui/assets/index-4599dad9.js");
const ASSETS_CSS: &str = include_str!("../../ui/assets/index-1e95b263.css");

const LOGO: &[u8] = include_bytes!("../../ui/images/logo100.png");
const FAVICON: &[u8] = include_bytes!("../../ui/favicon.ico");

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

pub async fn logo() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::png())
        .body(LOGO)
}

pub async fn favicon() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::png())
        .body(FAVICON)
}
