use anyhow::Result;
use once_cell::sync::OnceCell;
use onceuponai_core::common::ResultExt;
use serde::Deserialize;

static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Deserialize, Debug)]
pub(crate) struct Config {
    pub oidc_issuer_url: String,
    pub oidc_client_id: String,
    pub oidc_client_secret: String,
    pub oidc_redirect_url: String,
}

impl Config {
    pub fn get() -> &'static Config {
        CONFIG.get().expect("logger is not initialized")
    }

    pub fn load() -> Result<()> {
        dotenv::dotenv()?;
        let config = envy::from_env::<Config>()?;
        CONFIG.set(config).map_anyhow_err()?;
        Ok(())
    }
}
