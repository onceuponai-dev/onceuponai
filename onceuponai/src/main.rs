//#[cfg(feature = "accelerate")]
//extern crate accelerate_src;
//#[cfg(feature = "mkl")]
//extern crate intel_mkl_src;

//pub mod bot;
pub mod actors;
pub mod cli;
pub mod cluster;
pub mod config;
pub mod handlers;
pub mod llm;
pub mod models;
pub mod serve;
pub mod session;
use crate::cli::commands;
use anyhow::Result;
use config::Config;

#[actix_rt::main]
async fn main() -> Result<()> {
    Config::load()?;
    commands().await
}
