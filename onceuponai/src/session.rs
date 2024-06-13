use actix_session::Session;
use anyhow::Result;
use onceuponai_core::common::{OptionToResult, ResultExt};
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug)]
pub enum SessionItems {
    PKCE,
    NONCE,
    EMAIL,
}

impl SessionItems {
    fn as_str(&self) -> &str {
        match self {
            SessionItems::PKCE => "PKCE",
            SessionItems::NONCE => "NONCE",
            SessionItems::EMAIL => "EMAIL",
        }
    }
}

pub trait SessionExt {
    fn try_get<T: DeserializeOwned>(&self, key: &str) -> Result<T>;
    fn try_set<T: Serialize>(&self, key: &str, value: T) -> Result<()>;
    fn try_rm(&self, key: &str) -> Result<String>;

    fn pkce(&self) -> Result<String>;
    fn set_pkce(&self, pkce: &str) -> Result<()>;
    fn rm_pkce(&self) -> Result<String>;
    fn nonce(&self) -> Result<String>;
    fn set_nonce(&self, pkce: &str) -> Result<()>;
    fn rm_nonce(&self) -> Result<String>;
    fn email(&self) -> Result<String>;
    fn set_email(&self, email: &str) -> Result<()>;
    fn rm_email(&self) -> Result<String>;
}

macro_rules! session_method {
    ($get_fn:ident, $set_fn:ident, $rm_fn:ident, $key:expr) => {
        fn $get_fn(&self) -> Result<String> {
            self.get($key.as_str())?.ok_or_err($key.as_str())
        }

        fn $set_fn(&self, value: &str) -> Result<()> {
            self.insert($key.as_str(), value).map_anyhow_err()
        }

        fn $rm_fn(&self) -> Result<String> {
            self.remove($key.as_str()).ok_or_err($key.as_str())
        }
    };
}

impl SessionExt for Session {
    session_method!(pkce, set_pkce, rm_pkce, SessionItems::PKCE);
    session_method!(nonce, set_nonce, rm_nonce, SessionItems::NONCE);
    session_method!(email, set_email, rm_email, SessionItems::EMAIL);

    fn try_get<T: DeserializeOwned>(&self, key: &str) -> Result<T> {
        self.get(key)?.ok_or_err(key)
    }

    fn try_set<T: Serialize>(&self, key: &str, value: T) -> Result<()> {
        self.insert(key, value).map_anyhow_err()
    }

    fn try_rm(&self, key: &str) -> Result<String> {
        self.remove(key).ok_or_err(key)
    }
}

#[tokio::test]
async fn test_key() -> Result<()> {
    use actix_web::cookie::Key;
    use base64::{engine::general_purpose, Engine as _};
    let key = Key::generate();
    let master = key.master();
    let enc = general_purpose::STANDARD.encode(master);

    // Convert &str to String
    println!("KEY: {}", enc);

    let d = general_purpose::STANDARD.decode(enc)?;

    let _kk = Key::from(&d);
    Ok(())
}
