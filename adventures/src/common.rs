use anyhow::{anyhow, Result};
use std::io::{self, Result as IoResult};
use std::{fs, path::PathBuf};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub trait OptionToResult<T> {
    fn ok_or_err(self, name: &str) -> Result<T>;
}

impl<T> OptionToResult<T> for Option<T> {
    fn ok_or_err(self, name: &str) -> Result<T> {
        self.ok_or(anyhow!("{:?} - value not found", name))
    }
}

pub trait ResultExt<T, E> {
    fn map_anyhow_err(self) -> Result<T>;

    fn map_io_err(self) -> IoResult<T>;

    #[cfg(feature = "wasm")]
    fn map_jsv_err(self) -> Result<T, JsValue>;

    #[cfg(feature = "wasm")]
    fn map_jse_err(self) -> Result<T, JsError>;
}

impl<T, E: std::fmt::Debug> ResultExt<T, E> for Result<T, E> {
    fn map_anyhow_err(self) -> Result<T> {
        self.map_err(|e| anyhow!("{:?}", e))
    }

    fn map_io_err(self) -> IoResult<T> {
        self.map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e:?}")))
    }

    #[cfg(feature = "wasm")]
    fn map_jsv_err(self) -> Result<T, JsValue> {
        self.map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }

    #[cfg(feature = "wasm")]
    fn map_jse_err(self) -> Result<T, JsError> {
        self.map_err(|e| JsError::new(&format!("{:?}", e)))
    }
}

pub fn hf_hub_get_path(
    hf_repo_id: &str,
    filename: &str,
    endpoint: Option<String>,
    hf_token: Option<String>,
) -> Result<PathBuf> {
    use hf_hub::{api::sync::ApiBuilder, Repo, RepoType};

    let mut api_builder = ApiBuilder::new();

    if let Some(token) = hf_token {
        api_builder = api_builder.with_token(Some(token));
    }

    if let Some(e) = endpoint {
        api_builder = api_builder.with_endpoint(e);
    }

    let api = api_builder
        .build()?
        .repo(Repo::new(hf_repo_id.to_string(), RepoType::Model));
    let path = api.get(filename)?;

    Ok(path)
}

pub fn hf_hub_get(
    hf_repo_id: &str,
    filename: &str,
    endpoint: Option<String>,
    hf_token: Option<String>,
) -> Result<Vec<u8>> {
    let path = hf_hub_get_path(hf_repo_id, filename, endpoint, hf_token)?;
    let data = fs::read(path)?;
    Ok(data)
}

pub fn hf_hub_get_multiple(
    hf_repo_id: &str,
    json_file: &str,
    endpoint: Option<String>,
    hf_token: Option<String>,
) -> Result<Vec<PathBuf>> {
    use hf_hub::{api::sync::ApiBuilder, Repo, RepoType};

    let mut api_builder = ApiBuilder::new();

    if let Some(token) = hf_token {
        api_builder = api_builder.with_token(Some(token));
    }

    if let Some(e) = endpoint {
        api_builder = api_builder.with_endpoint(e);
    }

    let api = api_builder
        .build()?
        .repo(Repo::new(hf_repo_id.to_string(), RepoType::Model));
    let json_path = api.get(json_file)?;

    let json_file = std::fs::File::open(json_path)?;
    let json: serde_json::Value = serde_json::from_reader(&json_file)?;
    let weight_map = match json.get("weight_map") {
        None => Err(anyhow!("no weight map in {json_file:?}")),
        Some(serde_json::Value::Object(map)) => Ok(map),
        Some(_) => Err(anyhow!("weight map in {json_file:?} is not a map")),
    }?;
    let mut safetensors_files = std::collections::HashSet::new();
    for value in weight_map.values() {
        if let Some(file) = value.as_str() {
            safetensors_files.insert(file.to_string());
        }
    }
    let safetensors_files = safetensors_files
        .iter()
        .map(|v| api.get(v).map_err(anyhow::Error::msg))
        .collect::<Result<Vec<_>>>()?;

    Ok(safetensors_files)
}
