use anyhow::Context;
use axum::Router;
use clap::Parser;
use lightweight_fgpe_server::cli::Args;
use std::net::SocketAddr;
use tracing::log::info;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    init_tracing(&args.log_level)?;

    let router = lightweight_fgpe_server::init_router(&args)?;

    info!("Starting server...");
    run(router, args.server_address)
        .await
        .context("Server failed to run")?;

    Ok(())
}

fn init_tracing(log_level: &str) -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::try_new(log_level)?).init();
    Ok(())
}

async fn run(router: Router, addr: SocketAddr) -> anyhow::Result<()> {
    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to address {}", addr))?;
    axum::serve(listener, router.into_make_service())
        .await
        .context("Axum server error")?;
    Ok(())
}
