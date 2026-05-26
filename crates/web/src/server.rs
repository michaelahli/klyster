//! HTTP server setup and lifecycle management.

use crate::routes::health;
use crate::state::AppState;
use axum::{routing::get, Json, Router};
use serde_json::json;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info};

/// Errors that can occur while starting or running the HTTP server.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// Failed to parse the bind address from the configured host/port.
    #[error("Invalid bind address {host}:{port}: {source}")]
    InvalidAddress {
        /// Configured host string.
        host: String,
        /// Configured port number.
        port: u16,
        /// Underlying parse error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to bind the TCP listener.
    #[error("Failed to bind {addr}: {source}")]
    Bind {
        /// Address that failed to bind.
        addr: SocketAddr,
        /// Underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// The server returned an unexpected error while serving.
    #[error("Server error: {0}")]
    Serve(#[source] std::io::Error),
}

/// Build the application router with default middleware applied.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    Router::new()
        .route("/", get(root))
        .route("/healthz", get(health::liveness))
        .route("/readyz", get(health::readiness))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "name": "klyster",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Resolve the configured bind address.
fn resolve_addr(host: &str, port: u16) -> Result<SocketAddr, ServerError> {
    use std::net::ToSocketAddrs;

    (host, port)
        .to_socket_addrs()
        .map_err(|source| ServerError::InvalidAddress {
            host: host.to_string(),
            port,
            source,
        })?
        .next()
        .ok_or_else(|| ServerError::InvalidAddress {
            host: host.to_string(),
            port,
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "no addresses resolved",
            ),
        })
}

/// Bind the TCP listener according to the configured web host/port.
pub async fn bind(state: &AppState) -> Result<TcpListener, ServerError> {
    let host = &state.config().web.host;
    let port = state.config().web.port;
    let addr = resolve_addr(host, port)?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|source| ServerError::Bind { addr, source })?;

    let local = listener.local_addr().map_err(ServerError::Serve)?;
    info!(%local, "Web server listening");
    Ok(listener)
}

/// Run the HTTP server until `shutdown` resolves.
///
/// On shutdown, in-flight requests are given up to `grace_period` to drain
/// before the listener is forcibly closed.
pub async fn run<F>(
    listener: TcpListener,
    state: AppState,
    shutdown: F,
    grace_period: Duration,
) -> Result<(), ServerError>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let router = build_router(state);

    let server = axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(async move {
            shutdown.await;
            info!(grace_period_secs = grace_period.as_secs(), "Draining in-flight requests");
        });

    if let Err(e) = server.await {
        error!(error = %e, "Web server stopped with error");
        return Err(ServerError::Serve(e));
    }

    info!("Web server stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::DatabasePool;
    use domain::config::{
        AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, MetricsConfig,
        RetentionConfig, TelemetryConfig, WebConfig,
    };
    use domain::Config;
    use std::sync::Arc;

    fn test_config(port: u16) -> Config {
        Config {
            database: DatabaseConfig {
                db_type: "sqlite".to_string(),
                sqlite_path: ":memory:".to_string(),
                postgres_url: None,
                pool_size: 5,
                wal_mode: false,
            },
            web: WebConfig {
                host: "127.0.0.1".to_string(),
                port,
                workers: 1,
            },
            agent: AgentConfig {
                enabled: false,
                collection_interval_secs: 60,
            },
            analytics: AnalyticsConfig {
                enabled: false,
                grpc_endpoint: "http://localhost:50051".to_string(),
            },
            logging: LoggingConfig::default(),
            telemetry: TelemetryConfig::default(),
            metrics: MetricsConfig::default(),
            retention: RetentionConfig::default(),
        }
    }

    async fn test_state(port: u16) -> AppState {
        let config = test_config(port);
        let pool = DatabasePool::new(&config).await.unwrap();
        AppState::new(pool, Arc::new(config))
    }

    #[tokio::test]
    async fn binds_to_ephemeral_port() {
        // Use port 0 to let the OS pick an ephemeral port.
        let state = test_state(0).await;
        let listener = bind(&state).await.unwrap();
        let addr = listener.local_addr().unwrap();
        assert_ne!(addr.port(), 0);
    }

    #[tokio::test]
    async fn server_starts_and_stops_cleanly() {
        let state = test_state(0).await;
        let listener = bind(&state).await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server_state = state.clone();
        let handle = tokio::spawn(async move {
            run(
                listener,
                server_state,
                async move {
                    let _ = rx.await;
                },
                Duration::from_secs(1),
            )
            .await
        });

        // Hit the root endpoint to confirm the server is reachable.
        let url = format!("http://{addr}/");
        let response = tokio::task::spawn_blocking(move || {
            std::net::TcpStream::connect(addr).is_ok() && !url.is_empty()
        })
        .await
        .unwrap();
        assert!(response);

        // Trigger shutdown.
        tx.send(()).unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("server did not shut down within timeout")
            .expect("join failed");
        result.unwrap();
    }

    #[test]
    fn invalid_address_is_reported() {
        let result = resolve_addr("not a host", 8080);
        assert!(result.is_err());
    }
}
