//! Metric source CRUD endpoints.

use crate::dto::sources::{
    CreateSourceRequest, SourceListResponse, SourceResponse, UpdateSourceRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use db::repositories::MetricSourceRepository;
use domain::models::MetricSourceType;
use tracing::debug;

/// Create a new metric source.
///
/// POST /api/v1/sources
pub async fn create_source(
    State(state): State<AppState>,
    Json(req): Json<CreateSourceRequest>,
) -> ApiResult<(StatusCode, Json<SourceResponse>)> {
    debug!(name = %req.name, source_type = %req.source_type, "Creating metric source");

    // Validate source type
    if req.source_type.parse::<MetricSourceType>().is_err() {
        return Err(ApiError::ValidationError(format!(
            "Invalid source type '{}'. Must be 'prometheus' or 'agent'",
            req.source_type
        )));
    }

    // Validate name is not empty
    if req.name.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Source name cannot be empty".to_string(),
        ));
    }

    // Check if name already exists
    let repo = MetricSourceRepository::new(state.db());
    if repo.get_by_name(&req.name).await?.is_some() {
        return Err(ApiError::Conflict(format!(
            "Source with name '{}' already exists",
            req.name
        )));
    }

    // Serialize config to JSON string
    let config_str = serde_json::to_string(&req.config)
        .map_err(|e| ApiError::ValidationError(format!("Invalid config JSON: {e}")))?;

    let source = repo
        .create(&req.name, &req.source_type, &config_str)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(SourceResponse::from_model(source)),
    ))
}

/// List all metric sources.
///
/// GET /api/v1/sources
pub async fn list_sources(State(state): State<AppState>) -> ApiResult<Json<SourceListResponse>> {
    debug!("Listing metric sources");

    let repo = MetricSourceRepository::new(state.db());
    let sources = repo.list().await?;

    let total = sources.len();
    let sources = sources
        .into_iter()
        .map(SourceResponse::from_model)
        .collect();

    Ok(Json(SourceListResponse { sources, total }))
}

/// Get a metric source by ID.
///
/// GET /api/v1/sources/:id
pub async fn get_source(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<SourceResponse>> {
    debug!(id, "Getting metric source");

    let repo = MetricSourceRepository::new(state.db());
    let source = repo
        .get_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Source with id {id} not found")))?;

    Ok(Json(SourceResponse::from_model(source)))
}

/// Update a metric source.
///
/// PUT /api/v1/sources/:id
pub async fn update_source(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSourceRequest>,
) -> ApiResult<Json<SourceResponse>> {
    debug!(id, name = %req.name, source_type = %req.source_type, "Updating metric source");

    // Validate source type
    if req.source_type.parse::<MetricSourceType>().is_err() {
        return Err(ApiError::ValidationError(format!(
            "Invalid source type '{}'. Must be 'prometheus' or 'agent'",
            req.source_type
        )));
    }

    // Validate name is not empty
    if req.name.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Source name cannot be empty".to_string(),
        ));
    }

    let repo = MetricSourceRepository::new(state.db());

    // Check if name is taken by another source
    if let Some(existing) = repo.get_by_name(&req.name).await? {
        if existing.id != id {
            return Err(ApiError::Conflict(format!(
                "Source with name '{}' already exists",
                req.name
            )));
        }
    }

    // Serialize config to JSON string
    let config_str = serde_json::to_string(&req.config)
        .map_err(|e| ApiError::ValidationError(format!("Invalid config JSON: {e}")))?;

    let source = repo
        .update(id, &req.name, &req.source_type, &config_str)
        .await
        .map_err(|e| match e {
            db::DbError::NotFound(_) => {
                ApiError::NotFound(format!("Source with id {id} not found"))
            }
            _ => ApiError::Database(e),
        })?;

    Ok(Json(SourceResponse::from_model(source)))
}

/// Delete a metric source.
///
/// DELETE /api/v1/sources/:id
pub async fn delete_source(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    debug!(id, "Deleting metric source");

    let repo = MetricSourceRepository::new(state.db());
    let deleted = repo.delete(id).await?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Source with id {id} not found")))
    }
}

/// Test Prometheus connection.
///
/// GET /api/v1/sources/test?url=...
pub async fn test_connection(
    Query(params): Query<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let url = params.get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::ValidationError("Missing 'url' parameter".to_string()))?;

    // Simple connection test
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| ApiError::Internal(format!("Failed to create client: {e}")))?
;

    let response = client
        .get(format!("{url}/api/v1/query?query=up"))
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("Connection failed: {e}")))?
;

    if response.status().is_success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Successfully connected to Prometheus"
        })))
    } else {
        Err(ApiError::Internal(format!(
            "Prometheus returned status: {}",
            response.status()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use db::{run_migrations, DatabasePool};
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
        }
    }

    async fn test_state() -> AppState {
        let config = test_config();
        let pool = DatabasePool::new(&config).await.unwrap();
        run_migrations(&pool).await.unwrap();
        AppState::new(pool, Arc::new(config))
    }

    #[tokio::test]
    async fn test_create_source_success() {
        let state = test_state().await;

        let req = CreateSourceRequest {
            name: "test_prom".to_string(),
            source_type: "prometheus".to_string(),
            config: serde_json::json!({"url": "http://localhost:9090"}),
        };

        let result = create_source(State(state), Json(req)).await;
        assert!(result.is_ok());

        let (status, response) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.0.name, "test_prom");
        assert_eq!(response.0.source_type, "prometheus");
    }

    #[tokio::test]
    async fn test_create_source_invalid_type() {
        let state = test_state().await;

        let req = CreateSourceRequest {
            name: "test".to_string(),
            source_type: "invalid".to_string(),
            config: serde_json::json!({}),
        };

        let result = create_source(State(state), Json(req)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_create_source_empty_name() {
        let state = test_state().await;

        let req = CreateSourceRequest {
            name: String::new(),
            source_type: "prometheus".to_string(),
            config: serde_json::json!({}),
        };

        let result = create_source(State(state), Json(req)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_create_source_duplicate_name() {
        let state = test_state().await;

        let req = CreateSourceRequest {
            name: "duplicate".to_string(),
            source_type: "prometheus".to_string(),
            config: serde_json::json!({}),
        };

        let _result = create_source(State(state.clone()), Json(req.clone()))
            .await
            .unwrap();

        let result = create_source(State(state), Json(req)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_list_sources() {
        let state = test_state().await;

        let req1 = CreateSourceRequest {
            name: "source1".to_string(),
            source_type: "prometheus".to_string(),
            config: serde_json::json!({}),
        };
        let req2 = CreateSourceRequest {
            name: "source2".to_string(),
            source_type: "agent".to_string(),
            config: serde_json::json!({}),
        };

        let _result1 = create_source(State(state.clone()), Json(req1))
            .await
            .unwrap();
        let _result2 = create_source(State(state.clone()), Json(req2))
            .await
            .unwrap();

        let result = list_sources(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.total, 2);
        assert_eq!(response.sources.len(), 2);
    }

    #[tokio::test]
    async fn test_get_source() {
        let state = test_state().await;

        let req = CreateSourceRequest {
            name: "get_test".to_string(),
            source_type: "prometheus".to_string(),
            config: serde_json::json!({"key": "value"}),
        };

        let (_, created) = create_source(State(state.clone()), Json(req))
            .await
            .unwrap();

        let result = get_source(State(state), Path(created.0.id)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.name, "get_test");
        assert_eq!(response.config["key"], "value");
    }

    #[tokio::test]
    async fn test_get_source_not_found() {
        let state = test_state().await;

        let result = get_source(State(state), Path(99999)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_update_source() {
        let state = test_state().await;

        let create_req = CreateSourceRequest {
            name: "old_name".to_string(),
            source_type: "prometheus".to_string(),
            config: serde_json::json!({}),
        };

        let (_, created) = create_source(State(state.clone()), Json(create_req))
            .await
            .unwrap();

        let update_req = UpdateSourceRequest {
            name: "new_name".to_string(),
            source_type: "agent".to_string(),
            config: serde_json::json!({"updated": true}),
        };

        let result = update_source(State(state), Path(created.0.id), Json(update_req)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.name, "new_name");
        assert_eq!(response.source_type, "agent");
        assert_eq!(response.config["updated"], true);
    }

    #[tokio::test]
    async fn test_update_source_not_found() {
        let state = test_state().await;

        let req = UpdateSourceRequest {
            name: "test".to_string(),
            source_type: "prometheus".to_string(),
            config: serde_json::json!({}),
        };

        let result = update_source(State(state), Path(99999), Json(req)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_delete_source() {
        let state = test_state().await;

        let req = CreateSourceRequest {
            name: "to_delete".to_string(),
            source_type: "prometheus".to_string(),
            config: serde_json::json!({}),
        };

        let (_, created) = create_source(State(state.clone()), Json(req))
            .await
            .unwrap();

        let result = delete_source(State(state.clone()), Path(created.0.id)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);

        // Verify it's deleted
        let get_result = get_source(State(state), Path(created.0.id)).await;
        assert!(get_result.is_err());
    }

    #[tokio::test]
    async fn test_delete_source_not_found() {
        let state = test_state().await;

        let result = delete_source(State(state), Path(99999)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::NotFound(_)));
    }
}
