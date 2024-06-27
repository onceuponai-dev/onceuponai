use crate::actors::main_actor::{MainActor, MainActorConfig};
use crate::guards::AuthGuard;
use crate::handlers::actors::{connected_actors, invoke};
use crate::handlers::auth::generate_token;
use crate::handlers::users::user;
use crate::handlers::{
    self, assets_css, assets_js, health, index_html, ASSETS_CSS_HASH, ASSETS_JS_HASH,
};
use actix::Addr;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::middleware::Logger;
use actix_web::HttpResponse;
use actix_web::{cookie::Key, web, App, HttpServer};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use log::{debug, warn};
use num_traits::Zero;
use onceuponai_core::common::ResultExt;

fn get_secret_key(spec: &MainActorConfig) -> Result<Key> {
    let key = spec.session_key.clone().expect("SESSION_KEY");
    let k = general_purpose::STANDARD.decode(key)?;
    Ok(Key::from(&k))
}

pub struct AppState {
    pub addr: Addr<MainActor>,
    pub spec: MainActorConfig,
}

// Handler for getting a session value

#[allow(clippy::too_many_arguments)]
pub(crate) async fn serve(spec: MainActorConfig, addr: Addr<MainActor>) -> std::io::Result<()> {
    let secret_key = get_secret_key(&spec).map_io_err()?;
    let mut sp = spec.clone();
    if let Some(v) = spec.log_level {
        env_logger::init_from_env(env_logger::Env::new().default_filter_or(v));
    }

    if sp.oidc.is_none() {
        let root_token = generate_token(50);
        sp._auth_token = Some(root_token.clone());
        // warn!("Auth token ");
        println!(
            "Server running on http://{}:{}/login?token={}",
            spec.server_host, spec.server_port, root_token
        );
    } else {
        println!(
            "Server running on http://{}:{}",
            spec.server_host, spec.server_port
        );
    }

    let mut server = HttpServer::new(move || {
        let mut app = App::new()
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .wrap(Logger::default())
            .app_data(web::Data::new(AppState {
                addr: addr.clone(),
                spec: sp.clone(),
            }))
            .route("/", web::get().to(index_html))
            .route(
                &format!("/assets/index-{}.js", ASSETS_JS_HASH),
                web::get().to(assets_js),
            )
            .route(
                &format!("/assets/index-{}.css", ASSETS_CSS_HASH),
                web::get().to(assets_css),
            )
            .route("/health", web::get().to(health));

        if sp.oidc.is_none() && sp._auth_token.is_some() {
            app = app.route("/login", web::get().to(handlers::auth::token_login));
        }

        if sp.oidc.is_some() {
            app = app.service(
                web::scope("/auth")
                    .route("", web::get().to(handlers::auth::auth))
                    .route("/callback", web::get().to(handlers::auth::auth_callback)),
            );
        }

        let mut api_scope = web::scope("/api")
            .route("/actors", web::get().to(connected_actors))
            .route("/invoke/{kind}/{name}", web::post().to(invoke));

        api_scope = api_scope
            .guard(AuthGuard {
                secret: sp
                    .clone()
                    .personal_access_token_secret
                    .expect("PERSONAL_ACCESS_TOKEN_SECRET")
                    .to_string(),
            })
            .route("/user", web::get().to(handlers::users::user))
            .route(
                "/user/personal-token",
                web::post().to(handlers::auth::personal_token),
            );

        app = app.default_service(web::route().to(HttpResponse::Unauthorized));

        app = app.service(api_scope);

        //app.service(fs::Files::new("/", "../onceuponai-ui/dist/").show_files_listing())
        app
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
