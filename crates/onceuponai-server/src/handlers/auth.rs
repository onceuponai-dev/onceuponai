use crate::models::{AuthCallback, PATClaims, PATRequest, PATResponse, TokenLogin};
use crate::serve::AppState;
use crate::session::SessionExt;
use actix_web::HttpResponse;
use actix_web::{web, Responder};
use anyhow::anyhow;
use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use onceuponai_core::common::{Errors, OptionToResult};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::reqwest::http_client;
use openidconnect::PkceCodeVerifier;
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, RedirectUrl,
};
use openidconnect::{OAuth2TokenResponse, TokenResponse};
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde_json::json;
use std::error::Error;

pub async fn auth(
    session: actix_session::Session,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let oidc = app_state.spec.oidc();
    let provider_metadata =
        CoreProviderMetadata::discover(&IssuerUrl::new(oidc.issuer_url.clone())?, http_client)?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(oidc.client_id.clone()),
        Some(ClientSecret::new(oidc.client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(oidc.redirect_url.clone())?);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, _csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .set_pkce_challenge(pkce_challenge)
        .url();

    // println!("Browse to: {}", auth_url);
    // println!("PKCE: {}", pkce_verifier.secret());
    // println!("NONCE: {}", nonce.secret());
    // println!("CSRF: {}", csrf_token.secret());

    session.set_pkce(pkce_verifier.secret())?;
    session.set_nonce(nonce.secret())?;

    Ok(HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .finish())
}

pub async fn auth_callback(
    request: web::Query<AuthCallback>,
    session: actix_session::Session,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let oidc = app_state.spec.oidc();

    let pkce = session.pkce()?;
    let nonce = session.nonce()?;

    let pkce_verifier = PkceCodeVerifier::new(pkce);
    let nonce = Nonce::new(nonce);

    let provider_metadata =
        CoreProviderMetadata::discover(&IssuerUrl::new(oidc.issuer_url.clone())?, http_client)?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(oidc.client_id.clone()),
        Some(ClientSecret::new(oidc.client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(oidc.redirect_url.clone())?);

    let token_response = client
        .exchange_code(AuthorizationCode::new(request.code.clone()))
        .set_pkce_verifier(pkce_verifier)
        .request(http_client)?;

    let id_token = token_response
        .id_token()
        .ok_or_else(|| anyhow!("Server did not return an ID token"))?;
    let claims = id_token.claims(&client.id_token_verifier(), &nonce)?;

    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash =
            AccessTokenHash::from_token(token_response.access_token(), &id_token.signing_alg()?)?;
        if actual_access_token_hash != *expected_access_token_hash {
            return Err(Errors::str("Invalid access token"));
        }
    }

    // println!(
    //     "User {} with e-mail address {} has authenticated successfully",
    //     claims.subject().as_str(),
    //     claims
    //         .email()
    //         .map(|email| email.as_str())
    //         .unwrap_or("<not provided>"),
    // );

    let email = claims.email().ok_or_err("EMAIL")?;
    session.rm_pkce()?;
    session.rm_nonce()?;
    session.set_email(email)?;

    /*
        let userinfo: CoreUserInfoClaims = client
            .user_info(token_response.access_token().to_owned(), None)
            .map_err(|err| anyhow!("No user info endpoint: {:?}", err))?
            .request(http_client)
            .map_err(|err| anyhow!("Failed requesting user info: {:?}", err))?;
    */

    Ok(HttpResponse::Found()
        .append_header(("Location", "/".to_string()))
        .finish())
}

pub async fn unauthorized() -> HttpResponse {
    HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}))
}

pub async fn token_login(
    session: actix_session::Session,
    token: web::Query<TokenLogin>,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let token = token.token.clone();

    if app_state.spec._auth_token() == token {
        let _ = session.set_email("user@");
        return Ok(HttpResponse::Found()
            .append_header(("Location", "/".to_string()))
            .finish());
    }

    Ok(HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"})))
}

pub fn generate_token(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}

pub async fn personal_token(
    pat_request: web::Json<PATRequest>,
    session: actix_session::Session,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, Box<dyn Error>> {
    let email = session.get("EMAIL")?.unwrap_or("root".to_string());
    let secret = &app_state
        .spec
        .clone()
        .personal_access_token_secret
        .expect("PERSONAL_ACCESS_TOKEN_SECRET")
        .to_string();
    let personal_access_token = generate_pat_token(secret, &email, pat_request.expiration_days);
    Ok(HttpResponse::Ok().json(PATResponse {
        personal_access_token,
    }))
}

pub fn generate_pat_token(secret: &str, email: &str, expiration_days: i64) -> String {
    let my_claims = PATClaims {
        sub: email.to_string(),
        exp: (Utc::now() + Duration::days(expiration_days)).timestamp() as usize,
    };

    encode(
        &Header::default(),
        &my_claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .unwrap()
}

pub fn verify_pat_token(
    token: &str,
    secret: &str,
) -> Result<TokenData<PATClaims>, jsonwebtoken::errors::Error> {
    decode::<PATClaims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
}
