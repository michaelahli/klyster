//! Health check and readiness endpoints.

use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use tracing::warn;

/// Response for health check endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall status: "ok" or "degraded".
    pub status: String,
    /// Application version.
    pub version: String,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
    /// Component-level health checks.
    pub components: ComponentHealth,
}

/// Health status of individual components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Database connectivity status.
    pub database: ComponentStatus,
}

/// Status of a single component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    /// Status: "ok", "degraded", or "down".
    pub status: String,
    /// Optional message with details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Liveness probe endpoint.
///
/// Returns 200 OK if the server is running. Does not check dependencies.
/// Used by orchestrators to determine if the process should be restarted.
pub async fn liveness() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime_seconds(),
        components: ComponentHealth {
            database: ComponentStatus {
                status: "unknown".to_string(),
                message: Some("not checked in liveness probe".to_string()),
            },
        },
    })
}

/// Readiness probe endpoint.
///
/// Returns 200 OK if the server is ready to accept traffic (database reachable).
/// Returns 503 Service Unavailable if dependencies are not ready.
/// Used by load balancers to determine if traffic should be routed here.
pub async fn readiness(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db_status = check_database(&state).await;

    let overall_status = if db_status.status == "ok" {
        "ok"
    } else {
        "degraded"
    };

    let status_code = if overall_status == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = HealthResponse {
        status: overall_status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime_seconds(),
        components: ComponentHealth {
            database: db_status,
        },
    };

    (status_code, Json(response))
}

/// Check database connectivity by executing a simple query.
async fn check_database(state: &AppState) -> ComponentStatus {
    match state.db().ping().await {
        Ok(()) => ComponentStatus {
            status: "ok".to_string(),
            message: None,
        },
        Err(e) => {
            warn!(error = %e, "Database health check failed");
            ComponentStatus {
                status: "down".to_string(),
                message: Some(format!("database unreachable: {e}")),
            }
        }
    }
}

/// Calculate application uptime in seconds.
fn uptime_seconds() -> u64 {
    static START_TIME: std::sync::OnceLock<SystemTime> = std::sync::OnceLock::new();
    let start = START_TIME.get_or_init(SystemTime::now);

    start.elapsed().unwrap_or(Duration::ZERO).as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use db::DatabasePool;
    use domain::config::{
        AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, MetricsConfig,
        RetentionConfig, TelemetryConfig, WebConfig,
    };
    use domain::Config;
    use std::sync::Arc;

    fn test_config() -> Config {
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
                port: 8080,
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
            kubernetes: domain::config::KubernetesConfig::default(),
        }
    }

    async fn test_state() -> AppState {
        let config = test_config();
        let pool = DatabasePool::new(&config).await.unwrap();
        AppState::new(pool, Arc::new(config))
    }

    #[tokio::test]
    async fn liveness_always_returns_ok() {
        let response = liveness().await;
        assert_eq!(response.0.status, "ok");
        assert_eq!(response.0.version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn readiness_returns_ok_when_db_reachable() {
        let state = test_state().await;
        let (status, response) = readiness(State(state)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.0.status, "ok");
        assert_eq!(response.0.components.database.status, "ok");
    }

    #[tokio::test]
    async fn readiness_returns_503_when_db_unreachable() {
        // Create a valid pool, then close it to simulate unreachable DB
        let config = test_config();
        let pool = DatabasePool::new(&config).await.unwrap();
        pool.close().await;

        let state = AppState::new(pool, Arc::new(config));
        let (status, response) = readiness(State(state)).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.0.status, "degraded");
        assert_eq!(response.0.components.database.status, "down");
    }

    #[test]
    fn uptime_increases_over_time() {
        let uptime1 = uptime_seconds();
        std::thread::sleep(Duration::from_millis(100));
        let uptime2 = uptime_seconds();
        assert!(uptime2 >= uptime1);
    }
}
