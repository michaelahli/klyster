//! Forecast endpoints.

use crate::dto::forecasts::{
    ForecastDetailResponse, ForecastListResponse, ForecastPointResponse, ForecastResponse,
    TriggerForecastRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use db::repositories::ForecastRepository;
use serde::Deserialize;
use tracing::debug;

/// Query parameters for listing forecasts.
#[derive(Debug, Deserialize)]
pub struct ListForecastsQuery {
    /// Filter by resource group ID.
    pub resource_group_id: Option<i64>,
}

/// List forecasts.
///
/// GET /api/v1/forecasts
pub async fn list_forecasts(
    State(state): State<AppState>,
    Query(query): Query<ListForecastsQuery>,
) -> ApiResult<Json<ForecastListResponse>> {
    debug!(?query, "Listing forecasts");

    let repo = ForecastRepository::new(state.db());

    let forecasts = if let Some(group_id) = query.resource_group_id {
        repo.list_by_resource_group(group_id).await?
    } else {
        // If no filter, return empty list for now
        // In a real implementation, we'd have a list_all method
        vec![]
    };

    let total = forecasts.len();
    let forecasts = forecasts
        .into_iter()
        .map(ForecastResponse::from_model)
        .collect();

    Ok(Json(ForecastListResponse { forecasts, total }))
}

/// Get a forecast by ID with all data points.
///
/// GET /api/v1/forecasts/:id
pub async fn get_forecast(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<ForecastDetailResponse>> {
    debug!(id, "Getting forecast");

    let repo = ForecastRepository::new(state.db());

    let result = repo
        .get_forecast(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Forecast {id} not found")))?;

    let (forecast, points) = result;

    Ok(Json(ForecastDetailResponse {
        forecast: ForecastResponse::from_model(forecast),
        points: points
            .into_iter()
            .map(ForecastPointResponse::from_model)
            .collect(),
    }))
}

/// Trigger a forecast for a resource group.
///
/// POST /api/v1/forecasts/trigger
pub async fn trigger_forecast(
    State(_state): State<AppState>,
    Json(req): Json<TriggerForecastRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    debug!(
        metric_name = %req.metric_name,
        model_name = ?req.model_name,
        "Triggering forecast"
    );

    // Validate metric name
    if req.metric_name.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Metric name cannot be empty".to_string(),
        ));
    }

    // Validate horizon
    if let Some(hours) = req.horizon_hours {
        if hours <= 0 {
            return Err(ApiError::ValidationError(
                "horizon_hours must be > 0".to_string(),
            ));
        }
    }

    // For now, return a placeholder response
    // In M4 (Analytics), this will actually trigger the Python analytics engine
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "message": "Forecast triggered (analytics engine not yet implemented)",
            "metric_name": req.metric_name,
            "model_name": req.model_name.unwrap_or_else(|| "linear_regression".to_string()),
            "horizon_hours": req.horizon_hours.unwrap_or(24),
        })),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use db::migrate::run_migrations;
    use db::pool::DatabasePool;
    use db::repositories::ResourceRepository;
    use domain::config::{
        AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, MetricsConfig,
        RetentionConfig, TelemetryConfig, WebConfig,
    };
    use domain::models::{Forecast, ForecastPoint, ResourceGroup};
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

    async fn setup_test_state() -> (AppState, i64) {
        let config = test_config();
        let pool = DatabasePool::new(&config).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Create a test resource group
        let resource_repo = ResourceRepository::new(&pool);
        let now = chrono::Utc::now();
        let group = ResourceGroup {
            id: 0,
            name: "test-group".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: "{}".to_string(),
            created_at: now,
        };
        let group_id = resource_repo.create_group(&group).await.unwrap();

        (AppState::new(pool, std::sync::Arc::new(config)), group_id)
    }

    #[tokio::test]
    async fn test_list_forecasts() {
        let (state, group_id) = setup_test_state().await;

        // Create a forecast
        let repo = ForecastRepository::new(state.db());
        let now = chrono::Utc::now();
        let forecast = Forecast {
            id: 0,
            resource_group_id: group_id,
            metric_name: "cpu_usage".to_string(),
            model_name: "linear_regression".to_string(),
            parameters: None,
            horizon_start: now,
            horizon_end: now + chrono::Duration::hours(24),
            confidence_score: Some(0.85),
            created_at: now,
        };
        repo.create_forecast(&forecast, &[]).await.unwrap();

        // List forecasts
        let query = ListForecastsQuery {
            resource_group_id: Some(group_id),
        };
        let result = list_forecasts(State(state.clone()), Query(query)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.total, 1);
        assert_eq!(response.forecasts.len(), 1);
        assert_eq!(response.forecasts[0].metric_name, "cpu_usage");
    }

    #[tokio::test]
    async fn test_get_forecast() {
        let (state, group_id) = setup_test_state().await;

        // Create a forecast with points
        let repo = ForecastRepository::new(state.db());
        let now = chrono::Utc::now();
        let forecast = Forecast {
            id: 0,
            resource_group_id: group_id,
            metric_name: "memory_usage".to_string(),
            model_name: "arima".to_string(),
            parameters: Some(r#"{"order": [1,1,1]}"#.to_string()),
            horizon_start: now,
            horizon_end: now + chrono::Duration::hours(12),
            confidence_score: Some(0.9),
            created_at: now,
        };

        let points = vec![
            ForecastPoint {
                id: 0,
                forecast_id: 0,
                timestamp: now + chrono::Duration::hours(1),
                predicted_value: 0.6,
                lower_bound: Some(0.5),
                upper_bound: Some(0.7),
            },
            ForecastPoint {
                id: 0,
                forecast_id: 0,
                timestamp: now + chrono::Duration::hours(2),
                predicted_value: 0.65,
                lower_bound: Some(0.55),
                upper_bound: Some(0.75),
            },
        ];

        let forecast_id = repo.create_forecast(&forecast, &points).await.unwrap();

        // Get forecast
        let result = get_forecast(State(state.clone()), Path(forecast_id)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.forecast.metric_name, "memory_usage");
        assert_eq!(response.points.len(), 2);
        assert_eq!(response.points[0].predicted_value, 0.6);
    }

    #[tokio::test]
    async fn test_get_forecast_not_found() {
        let (state, _) = setup_test_state().await;

        let result = get_forecast(State(state.clone()), Path(999)).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_trigger_forecast() {
        let (state, _) = setup_test_state().await;

        let req = TriggerForecastRequest {
            metric_name: "cpu_usage".to_string(),
            model_name: Some("arima".to_string()),
            horizon_hours: Some(48),
        };

        let result = trigger_forecast(State(state.clone()), Json(req)).await;
        assert!(result.is_ok());

        let (status, response) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(response["metric_name"], "cpu_usage");
        assert_eq!(response["model_name"], "arima");
        assert_eq!(response["horizon_hours"], 48);
    }

    #[tokio::test]
    async fn test_trigger_forecast_validation() {
        let (state, _) = setup_test_state().await;

        // Empty metric name
        let req = TriggerForecastRequest {
            metric_name: "".to_string(),
            model_name: None,
            horizon_hours: None,
        };

        let result = trigger_forecast(State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));

        // Invalid horizon
        let req = TriggerForecastRequest {
            metric_name: "cpu_usage".to_string(),
            model_name: None,
            horizon_hours: Some(-10),
        };

        let result = trigger_forecast(State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));
    }
}
