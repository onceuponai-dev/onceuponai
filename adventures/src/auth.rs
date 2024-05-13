use anyhow::{anyhow, Result};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode_header, Algorithm, DecodingKey, TokenData, Validation};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tokio::time::{self, Duration};

static JWKS: OnceCell<Arc<Mutex<JwkSet>>> = OnceCell::new();

pub struct AuthState {
    pub auth_token: Arc<RwLock<AuthToken>>,
}

pub async fn refresh_authstate() -> Result<AuthToken> {
    let bot_client_id = env::var("BOT_CLIENT_ID")?;
    let bot_client_secret = env::var("BOT_CLIENT_SECRET")?;
    let client = reqwest::Client::new();
    let data = client
        .post("https://login.microsoftonline.com/botframework.com/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("grant_type=client_credentials&client_id={}&client_secret={}&scope=https%3A%2F%2Fapi.botframework.com%2F.default",bot_client_id, bot_client_secret))
        .send()
        .await?.json::<AuthToken>().await?;

    Ok(data)
}

pub async fn authstate_task(shared_authstate: Arc<RwLock<AuthToken>>) {
    let mut interval = time::interval(Duration::from_secs(300));
    loop {
        interval.tick().await;
        let mut write_guard = shared_authstate.write().await;
        *write_guard = refresh_authstate().await.unwrap();
    }
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct AuthToken {
    pub token_type: String,
    pub expires_in: usize,
    pub ext_expires_in: usize,
    pub access_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    serviceurl: String,
    nbf: u32,
    exp: u32,
    iss: String,
    aud: String,
}

async fn fetch_jwks(url: &str) -> Result<JwkSet> {
    if let Some(jwks_arc) = JWKS.get() {
        let keys = jwks_arc.lock().unwrap().keys.clone();
        let jwks = JwkSet { keys };
        Ok(jwks)
    } else {
        let response = reqwest::get(url).await?;
        let jwks = response.json::<JwkSet>().await?;
        let _ = JWKS.set(Arc::new(Mutex::new(jwks.clone()))).is_ok();
        Ok(jwks)
    }
}

fn find_jwk(jwks: &JwkSet, kid: String) -> Option<DecodingKey> {
    jwks.find(&kid)
        .map(|jwk| DecodingKey::from_jwk(jwk).expect("Wrong jwk"))
}

fn decode_jwt(
    token: &str,
    public_key: &DecodingKey,
    alg: Algorithm,
    aud: &str,
) -> Result<TokenData<Claims>> {
    let mut validation = Validation::new(alg);
    validation.set_audience(&[aud]);
    let token_data = jsonwebtoken::decode::<Claims>(token, public_key, &validation)?;
    Ok(token_data)
}

pub async fn validate_jwt(token: &str) -> Result<bool> {
    let jwks_url = env::var("BOT_JWKS_URL")?;
    let aud = env::var("BOT_CLIENT_ID")?;
    let headers = decode_header(token)?;
    let kid = headers.kid.expect("Wrong kid");
    let alg = headers.alg;

    if let Ok(jwks) = fetch_jwks(&jwks_url).await {
        if let Some(public_key) = find_jwk(&jwks, kid) {
            let _ = decode_jwt(token, &public_key, alg, &aud)?;
            Ok(true)
        } else {
            Err(anyhow!("No suitable key found for validation"))
        }
    } else {
        Err(anyhow!("Failed to fetch JWKS"))
    }
}

#[tokio::test]
async fn test_reqwest() -> Result<()> {
    let bot_client_id = env::var("BOT_CLIENT_ID")?;
    let bot_client_secret = env::var("BOT_CLIENT_SECRET")?;
    let client = reqwest::Client::new();
    let data = client
        .post("https://login.microsoftonline.com/botframework.com/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("grant_type=client_credentials&client_id={}&client_secret={}&scope=https%3A%2F%2Fapi.botframework.com%2F.default",bot_client_id, bot_client_secret))
        .send()
        .await?.json::<AuthToken>().await?;

    //let v: = serde_json::from_str(&data)?;
    println!("Q!!! {data:?}");
    Ok(())
}
