//! Agent HTTP server for receiving pushed metrics.

use axum::{routing::post, Router};
use std::net::SocketAddr;
use tracing::info;

pub mod routes;

/// Start the agent HTTP server.
pub async fn start(addr: SocketAddr) -> anyhow::Result<()> {
    let app = Router::new().route("/metrics", post(routes::push_metrics));

    info!("Agent HTTP server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
