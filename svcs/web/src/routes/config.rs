//! Configuration management endpoints.

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Response for configuration view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    /// Web server configuration.
    pub web: WebConfigResponse,
    /// Agent configuration.
    pub agent: AgentConfigResponse,
    /// Analytics configuration.
    pub analytics: AnalyticsConfigResponse,
    /// Logging configuration.
    pub logging: LoggingConfigResponse,
    /// Retention configuration.
    pub retention: RetentionConfigResponse,
}

/// Web server configuration response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfigResponse {
    /// Server host address.
    pub host: String,
    /// Server port.
    pub port: u16,
    /// Number of worker threads.
    pub workers: usize,
}

/// Agent configuration response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigResponse {
    /// Whether agent is enabled.
    pub enabled: bool,
    /// Collection interval in seconds.
    pub collection_interval_secs: u64,
}

/// Analytics configuration response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfigResponse {
    /// Whether analytics is enabled.
    pub enabled: bool,
    /// gRPC endpoint for analytics service.
    pub grpc_endpoint: String,
}

/// Logging configuration response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfigResponse {
    /// Log level.
    pub level: String,
    /// Log format.
    pub format: String,
}

/// Retention configuration response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfigResponse {
    /// Metrics retention in days.
    pub metrics_days: u32,
    /// Forecasts retention in days.
    pub forecasts_days: u32,
    /// Recommendations retention in days.
    pub recommendations_days: u32,
}

/// Request to update mutable configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfigRequest {
    /// Agent collection interval (optional).
    pub collection_interval_secs: Option<u64>,
    /// Metrics retention days (optional).
    pub metrics_retention_days: Option<u32>,
    /// Forecasts retention days (optional).
    pub forecasts_retention_days: Option<u32>,
    /// Recommendations retention days (optional).
    pub recommendations_retention_days: Option<u32>,
}

/// Get current configuration.
///
/// GET /api/v1/config
pub async fn get_config(State(state): State<AppState>) -> ApiResult<Json<ConfigResponse>> {
    debug!("Getting configuration");

    let config = state.config();

    Ok(Json(ConfigResponse {
        web: WebConfigResponse {
            host: config.web.host.clone(),
            port: config.web.port,
            workers: config.web.workers,
        },
        agent: AgentConfigResponse {
            enabled: config.agent.enabled,
            collection_interval_secs: config.agent.collection_interval_secs,
        },
        analytics: AnalyticsConfigResponse {
            enabled: config.analytics.enabled,
            grpc_endpoint: config.analytics.grpc_endpoint.clone(),
        },
        logging: LoggingConfigResponse {
            level: config.logging.level.clone(),
            format: config.logging.format.clone(),
        },
        retention: RetentionConfigResponse {
            metrics_days: config.retention.metrics_days,
            forecasts_days: config.retention.forecasts_days,
            recommendations_days: config.retention.recommendations_days,
        },
    }))
}

/// Update mutable configuration values.
///
/// PATCH /api/v1/config
pub async fn update_config(
    State(_state): State<AppState>,
    Json(req): Json<UpdateConfigRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    debug!(?req, "Updating configuration");

    // Validate values
    if let Some(interval) = req.collection_interval_secs {
        if interval == 0 {
            return Err(ApiError::ValidationError(
                "collection_interval_secs must be > 0".to_string(),
            ));
        }
    }

    if let Some(days) = req.metrics_retention_days {
        if days == 0 {
            return Err(ApiError::ValidationError(
                "metrics_retention_days must be > 0".to_string(),
            ));
        }
    }

    // In a real implementation, we would:
    // 1. Update the configuration in memory
    // 2. Persist changes to the config file
    // 3. Notify components of configuration changes
    //
    // For now, return a placeholder response
    Ok(Json(serde_json::json!({
        "message": "Configuration update not yet implemented (requires config persistence)",
        "requested_changes": req,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use db::migrate::run_migrations;
    use db::pool::DatabasePool;
    use domain::config::{
        AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, MetricsConfig,
        RetentionConfig, TelemetryConfig, WebConfig,
    };
    use domain::Config;

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
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                outputs: vec![],
            },
            telemetry: TelemetryConfig::default(),
            metrics: MetricsConfig::default(),
            retention: RetentionConfig::default(),
        }
    }

    async fn setup_test_state() -> AppState {
        let config = test_config();
        let pool = DatabasePool::new(&config).await.unwrap();
        run_migrations(&pool).await.unwrap();
        AppState::new(pool, std::sync::Arc::new(config))
    }

    #[tokio::test]
    async fn test_get_config() {
        let state = setup_test_state().await;

        let result = get_config(State(state.clone())).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.web.host, "127.0.0.1");
        assert_eq!(response.web.port, 8080);
        assert_eq!(response.agent.collection_interval_secs, 60);
    }

    #[tokio::test]
    async fn test_update_config() {
        let state = setup_test_state().await;

        let req = UpdateConfigRequest {
            collection_interval_secs: Some(120),
            metrics_retention_days: Some(60),
            forecasts_retention_days: None,
            recommendations_retention_days: None,
        };

        let result = update_config(State(state.clone()), Json(req)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_config_validation() {
        let state = setup_test_state().await;

        // Invalid: zero interval
        let req = UpdateConfigRequest {
            collection_interval_secs: Some(0),
            metrics_retention_days: None,
            forecasts_retention_days: None,
            recommendations_retention_days: None,
        };

        let result = update_config(State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));

        // Invalid: zero retention
        let req = UpdateConfigRequest {
            collection_interval_secs: None,
            metrics_retention_days: Some(0),
            forecasts_retention_days: None,
            recommendations_retention_days: None,
        };

        let result = update_config(State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));
    }
}
