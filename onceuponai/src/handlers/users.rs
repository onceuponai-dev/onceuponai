use actix_web::Responder;
use actix_web::{HttpRequest, HttpResponse};
use anyhow::Result;
use std::error::Error;

pub async fn user(
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
