//#[cfg(feature = "accelerate")]
//extern crate accelerate_src;
//#[cfg(feature = "mkl")]
//extern crate intel_mkl_src;

//pub mod bot;
pub mod cli;
pub mod llm;
pub mod models;
pub mod serve;
use crate::cli::commands;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    commands().await
}
