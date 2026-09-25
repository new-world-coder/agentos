//! Axum HTTP API for AgentOS.

mod routes;
mod state;

pub use routes::router;
pub use state::{AppState, build_runtime};

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use tracing_subscriber::EnvFilter;

/// Run the HTTP server.
pub async fn serve(addr: SocketAddr, db_path: Option<PathBuf>) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("agentos=info".parse()?))
        .init();

    let runtime = build_runtime(db_path).context("build runtime")?;
    let state = AppState {
        runtime: Arc::new(runtime),
    };
    let app = router(state);

    tracing::info!(%addr, "AgentOS API listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
