//! HTTP server setup and lifecycle management.

use crate::error::{ErrorDetail, ErrorResponse};
use crate::middleware::apm_logging_middleware;
use crate::routes::{
    analytics, config, forecasts, health, metrics, recommendations, resource_groups, sources, ws,
};
use crate::state::AppState;
use axum::http::StatusCode;
use axum::middleware;
use axum::routing::{delete, post, put};
use axum::{routing::get, Json, Router};
use serde_json::json;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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

    // API v1 routes
    let api_v1 = Router::new()
        .route("/sources", post(sources::create_source))
        .route("/sources", get(sources::list_sources))
        .route("/sources/:id", get(sources::get_source))
        .route("/sources/:id", put(sources::update_source))
        .route("/sources/:id", delete(sources::delete_source))
        .route("/metrics", get(metrics::list_metric_names))
        .route("/metrics/latest", get(metrics::get_latest_metrics))
        .route("/metrics/:name", get(metrics::query_metrics))
        .route("/resource-groups", post(resource_groups::create_group))
        .route("/resource-groups", get(resource_groups::list_groups))
        .route("/resource-groups/:id", get(resource_groups::get_group))
        .route("/resource-groups/:id", put(resource_groups::update_group))
        .route(
            "/resource-groups/:id",
            delete(resource_groups::delete_group),
        )
        .route(
            "/resource-groups/:id/scaling-targets",
            post(resource_groups::set_scaling_target),
        )
        .route(
            "/resource-groups/:id/resources",
            get(resource_groups::list_resources),
        )
        .route("/forecasts", get(forecasts::list_forecasts))
        .route("/forecasts/trigger", post(forecasts::trigger_forecast))
        .route("/forecasts/:id", get(forecasts::get_forecast))
        .route(
            "/recommendations",
            get(recommendations::list_recommendations),
        )
        .route(
            "/recommendations/pending",
            get(recommendations::list_pending_recommendations),
        )
        .route(
            "/recommendations/:id/approve",
            post(recommendations::approve_recommendation),
        )
        .route(
            "/recommendations/:id/dismiss",
            post(recommendations::dismiss_recommendation),
        )
        .route("/analytics/functions", get(analytics::list_functions))
        .route("/analytics/functions", post(analytics::create_function))
        .route("/analytics/functions/:id", get(analytics::get_function))
        .route("/analytics/functions/:id", put(analytics::update_function))
        .route(
            "/analytics/functions/:id",
            delete(analytics::delete_function),
        )
        .route(
            "/analytics/functions/:id/test",
            post(analytics::test_function),
        )
        .route("/config", get(config::get_config))
        .route("/config", axum::routing::patch(config::update_config))
        .route("/ws/metrics", get(ws::ws_metrics_handler));

    Router::new()
        .route("/", get(root))
        .route("/healthz", get(health::liveness))
        .route("/readyz", get(health::readiness))
        .route("/metrics", get(metrics_endpoint))
        .nest("/api/v1", api_v1)
        .merge(
            SwaggerUi::new("/api/docs")
                .url("/api/docs/openapi.json", crate::docs::ApiDoc::openapi()),
        )
        .fallback(handler_404)
        .layer(middleware::from_fn(apm_logging_middleware))
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

/// Prometheus metrics endpoint.
async fn metrics_endpoint() -> Result<String, (StatusCode, String)> {
    crate::metrics::gather_metrics().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

/// 404 handler for unknown routes.
async fn handler_404() -> (StatusCode, Json<ErrorResponse>) {
    let response = ErrorResponse {
        error: ErrorDetail {
            code: "not_found".to_string(),
            message: "The requested resource was not found".to_string(),
        },
    };
    (StatusCode::NOT_FOUND, Json(response))
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
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "no addresses resolved"),
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

    let server =
        axum::serve(listener, router.into_make_service()).with_graceful_shutdown(async move {
            shutdown.await;
            info!(
                grace_period_secs = grace_period.as_secs(),
                "Draining in-flight requests"
            );
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
    use tower::ServiceExt;

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
                prometheus: domain::config::PrometheusAgentConfig::default(),
            },
            analytics: AnalyticsConfig {
                enabled: false,
                grpc_endpoint: "http://localhost:50051".to_string(),
                python_path: None,
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

    #[tokio::test]
    async fn handler_404_returns_json_error() {
        let state = test_state(0).await;
        let router = build_router(state);

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/nonexistent")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "not_found");
    }
}
