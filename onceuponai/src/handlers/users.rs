use actix_web::Responder;
use actix_web::{HttpRequest, HttpResponse};
use anyhow::Result;
use serde::Serialize;
use std::error::Error;

#[derive(Serialize, Debug)]
pub struct UserInfo {
    pub email: String,
}

pub async fn user(
    _req: HttpRequest,
    session: actix_session::Session,
) -> Result<impl Responder, Box<dyn Error>> {
    let user_id: Option<String> = session.get("EMAIL")?;
    if let Some(user_id) = user_id {
        Ok(HttpResponse::Ok().json(UserInfo { email: user_id }))
    } else {
        Ok(HttpResponse::Ok().body("No session found"))
    }
}

pub async fn anonymous(_req: HttpRequest) -> Result<impl Responder, Box<dyn Error>> {
    Ok(HttpResponse::Ok().json(UserInfo {
        email: "anonymous".to_string(),
    }))
}
