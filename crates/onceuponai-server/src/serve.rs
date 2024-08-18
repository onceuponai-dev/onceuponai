use crate::guards::AuthGuard;
use crate::handlers::actors::{connected_actors, invoke};
use crate::handlers::oai::{v1_chat_completions, v1_embeddings};
use crate::handlers::{
    self, assets_css, assets_js, favicon, health, index_html, logo, ASSETS_CSS_HASH, ASSETS_JS_HASH,
};
use actix::Addr;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::middleware::Logger;
use actix_web::HttpResponse;
use actix_web::{cookie::Key, web, App, HttpServer};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use num_traits::Zero;
use onceuponai_actors::actors::main_actor::{MainActor, MainActorSpec};
use onceuponai_core::common::generate_token;
use onceuponai_core::common::ResultExt;

fn get_secret_key(spec: &MainActorSpec) -> Result<Key> {
    let key = spec.session_key.clone().expect("SESSION_KEY");
    let k = general_purpose::STANDARD.decode(key)?;
    Ok(Key::from(&k))
}

pub struct AppState {
    pub addr: Addr<MainActor>,
    pub spec: MainActorSpec,
}

pub async fn serve(spec: MainActorSpec, addr: Addr<MainActor>) -> std::io::Result<()> {
    let secret_key = get_secret_key(&spec).map_io_err()?;
    let mut sp = spec.clone();
    if let Some(v) = spec.log_level {
        env_logger::init_from_env(env_logger::Env::new().default_filter_or(v));
    }

    if !sp.is_oidc() {
        let _auth_token = generate_token(50);
        sp._auth_token_set(_auth_token.clone());
        // warn!("Auth token ");
        println!(
            "Server running on http://{}:{}/login?token={}",
            spec.server_host, spec.server_port, _auth_token
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
            .route("/ui/images/logo100.png", web::get().to(logo))
            .route("/favicon.ico", web::get().to(favicon))
            .route("/health", web::get().to(health));

        if sp.is_oidc() {
            app = app.service(
                web::scope("/auth")
                    .route("", web::get().to(handlers::auth::auth))
                    .route("/callback", web::get().to(handlers::auth::auth_callback)),
            );
        } else {
            app = app.route("/login", web::get().to(handlers::auth::token_login));
        }

        let auth_guard = AuthGuard {
            secret: sp
                .clone()
                .personal_access_token_secret
                .expect("PERSONAL_ACCESS_TOKEN_SECRET")
                .to_string(),
        };

        app = app.default_service(web::route().to(HttpResponse::Unauthorized));

        app = app.service(
            web::scope("/api")
                .guard(auth_guard.clone())
                .route("/actors", web::get().to(connected_actors))
                .route("/invoke/{kind}/{name}", web::post().to(invoke))
                .route("/user", web::get().to(handlers::users::user))
                .route(
                    "/user/personal-token",
                    web::post().to(handlers::auth::personal_token),
                ),
        );

        app = app.service(
            web::scope("v1")
                .guard(auth_guard)
                .route("/chat/completions", web::post().to(v1_chat_completions))
                .route("/embeddings", web::post().to(v1_embeddings)),
        );

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
