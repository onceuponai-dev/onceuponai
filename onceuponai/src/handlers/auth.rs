use crate::config::Config;
use crate::models::AuthCallback;
use crate::session::SessionExt;
use actix_web::{web, Responder};
use actix_web::{HttpRequest, HttpResponse};
use anyhow::anyhow;
use anyhow::Result;
use onceuponai_core::common::{Errors, OptionToResult};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::reqwest::http_client;
use openidconnect::PkceCodeVerifier;
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, RedirectUrl,
};
use openidconnect::{OAuth2TokenResponse, TokenResponse};
use std::error::Error;

pub async fn auth(
    _req: HttpRequest,
    session: actix_session::Session,
) -> Result<impl Responder, Box<dyn Error>> {
    let provider_metadata = CoreProviderMetadata::discover(
        &IssuerUrl::new(Config::get().oidc_issuer_url.clone())?,
        http_client,
    )?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(Config::get().oidc_client_id.clone()),
        Some(ClientSecret::new(Config::get().oidc_client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(Config::get().oidc_redirect_url.clone())?);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .set_pkce_challenge(pkce_challenge)
        .url();

    println!("Browse to: {}", auth_url);
    println!("PKCE: {}", pkce_verifier.secret());
    println!("NONCE: {}", nonce.secret());
    println!("CSRF: {}", csrf_token.secret());

    session.set_pkce(pkce_verifier.secret())?;
    session.set_nonce(nonce.secret())?;

    Ok(HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .finish())
}

pub async fn auth_callback(
    request: web::Query<AuthCallback>,
    session: actix_session::Session,
) -> Result<impl Responder, Box<dyn Error>> {
    let pkce = session.pkce()?;
    let nonce = session.nonce()?;

    let pkce_verifier = PkceCodeVerifier::new(pkce);
    let nonce = Nonce::new(nonce);

    let provider_metadata = CoreProviderMetadata::discover(
        &IssuerUrl::new(Config::get().oidc_issuer_url.clone())?,
        http_client,
    )?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(Config::get().oidc_client_id.clone()),
        Some(ClientSecret::new(Config::get().oidc_client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(Config::get().oidc_redirect_url.clone())?);

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

    println!(
        "User {} with e-mail address {} has authenticated successfully",
        claims.subject().as_str(),
        claims
            .email()
            .map(|email| email.as_str())
            .unwrap_or("<not provided>"),
    );

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
