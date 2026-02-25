//! HTTP REST API server for Jarvis.
//!
//! This crate provides a web interface to interact with Jarvis remotely,
//! following the hexagonal architecture pattern by reusing the core library.

use anyhow::Result;
use clap::Parser;
use jarvis_common::CliConfigOverrides;
use tracing::info;

mod handlers;
mod middleware;
mod models;
mod server;
mod state;

#[cfg(test)]
pub mod test_utils;

use server::run_server;

#[derive(Parser, Debug)]
#[clap(name = "jarvis-web-api")]
struct Args {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    info!("Starting Jarvis Web API server...");

    run_server(args.config_overrides).await?;

    Ok(())
}
