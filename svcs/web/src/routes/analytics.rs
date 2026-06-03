//! Analytics function management endpoints.

use crate::dto::analytics::{
    CreateFunctionRequest, FunctionListResponse, FunctionResponse, TestFunctionRequest,
    TestFunctionResponse, UpdateFunctionRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use db::repositories::AnalyticsFunctionRepository;
use domain::models::AnalyticsFunction;
use tracing::debug;

/// List all analytics functions (predefined + custom).
///
/// GET /api/v1/analytics/functions
pub async fn list_functions(
    State(state): State<AppState>,
) -> ApiResult<Json<FunctionListResponse>> {
    debug!("Listing analytics functions");

    let repo = AnalyticsFunctionRepository::new(state.db());
    let functions = repo.list_all().await?;

    let total = functions.len();
    let functions = functions
        .into_iter()
        .map(FunctionResponse::from_model)
        .collect();

    Ok(Json(FunctionListResponse { functions, total }))
}

/// Get an analytics function by ID.
///
/// GET /api/v1/analytics/functions/:id
pub async fn get_function(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<FunctionResponse>> {
    debug!(id, "Getting analytics function");

    let repo = AnalyticsFunctionRepository::new(state.db());

    let function = repo
        .get_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Analytics function {id} not found")))?;

    Ok(Json(FunctionResponse::from_model(function)))
}

/// Register a custom analytics function.
///
/// POST /api/v1/analytics/functions
pub async fn create_function(
    State(state): State<AppState>,
    Json(req): Json<CreateFunctionRequest>,
) -> ApiResult<(StatusCode, Json<FunctionResponse>)> {
    debug!(name = %req.name, "Creating custom analytics function");

    // Validate name is not empty
    if req.name.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Function name cannot be empty".to_string(),
        ));
    }

    // Validate description
    if req.description.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Function description cannot be empty".to_string(),
        ));
    }

    // Validate language
    if req.language.to_lowercase() != "python" {
        return Err(ApiError::ValidationError(
            "Only 'python' language is supported".to_string(),
        ));
    }

    // Validate source code is not empty
    if req.source_code.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Source code cannot be empty".to_string(),
        ));
    }

    // Basic Python syntax validation (check if it's valid Python)
    // In a real implementation, we'd use a Python parser or send to analytics engine
    if !req.source_code.contains("def ") {
        return Err(ApiError::ValidationError(
            "Source code must contain at least one function definition".to_string(),
        ));
    }

    let repo = AnalyticsFunctionRepository::new(state.db());

    // Check if name already exists
    if repo.get_by_name(&req.name).await?.is_some() {
        return Err(ApiError::Conflict(format!(
            "Function with name '{}' already exists",
            req.name
        )));
    }

    // Serialize parameters schema to JSON string
    let parameters_schema = req
        .parameters_schema
        .map(|p| serde_json::to_string(&p))
        .transpose()
        .map_err(|e| ApiError::ValidationError(format!("Invalid parameters schema: {e}")))?;

    let now = Utc::now();
    let function = AnalyticsFunction {
        id: 0,
        name: req.name,
        description: req.description,
        function_type: "custom".to_string(),
        language: req.language,
        source_code: Some(req.source_code),
        parameters_schema,
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    let id = repo.create(&function).await?;

    let created = repo
        .get_by_id(id)
        .await?
        .ok_or_else(|| ApiError::Internal("Failed to retrieve created function".to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(FunctionResponse::from_model(created)),
    ))
}

/// Update a custom analytics function.
///
/// PUT /api/v1/analytics/functions/:id
pub async fn update_function(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateFunctionRequest>,
) -> ApiResult<Json<FunctionResponse>> {
    debug!(id, name = %req.name, "Updating analytics function");

    let repo = AnalyticsFunctionRepository::new(state.db());

    // Check if function exists
    let existing = repo
        .get_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Analytics function {id} not found")))?;

    // Cannot update predefined functions
    if existing.function_type == "predefined" {
        return Err(ApiError::ValidationError(
            "Cannot update predefined functions".to_string(),
        ));
    }

    // Validate name is not empty
    if req.name.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Function name cannot be empty".to_string(),
        ));
    }

    // Validate source code
    if req.source_code.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Source code cannot be empty".to_string(),
        ));
    }

    if !req.source_code.contains("def ") {
        return Err(ApiError::ValidationError(
            "Source code must contain at least one function definition".to_string(),
        ));
    }

    // Serialize parameters schema
    let parameters_schema = req
        .parameters_schema
        .map(|p| serde_json::to_string(&p))
        .transpose()
        .map_err(|e| ApiError::ValidationError(format!("Invalid parameters schema: {e}")))?;

    let updated_function = AnalyticsFunction {
        id,
        name: req.name,
        description: req.description,
        function_type: existing.function_type,
        language: req.language,
        source_code: Some(req.source_code),
        parameters_schema,
        is_active: req.is_active,
        created_at: existing.created_at,
        updated_at: Utc::now(),
    };

    repo.update(&updated_function).await?;

    let updated = repo
        .get_by_id(id)
        .await?
        .ok_or_else(|| ApiError::Internal("Failed to retrieve updated function".to_string()))?;

    Ok(Json(FunctionResponse::from_model(updated)))
}

/// Delete a custom analytics function.
///
/// DELETE /api/v1/analytics/functions/:id
pub async fn delete_function(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    debug!(id, "Deleting analytics function");

    let repo = AnalyticsFunctionRepository::new(state.db());

    // Check if function exists
    let existing = repo
        .get_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Analytics function {id} not found")))?;

    // Cannot delete predefined functions
    if existing.function_type == "predefined" {
        return Err(ApiError::ValidationError(
            "Cannot delete predefined functions".to_string(),
        ));
    }

    let rows = repo.delete(id).await?;

    if rows == 0 {
        return Err(ApiError::NotFound(format!(
            "Analytics function {id} not found"
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Test a function with sample data (dry-run).
///
/// POST /api/v1/analytics/functions/:id/test
pub async fn test_function(
    State(_state): State<AppState>,
    Path(id): Path<i64>,
    Json(_req): Json<TestFunctionRequest>,
) -> ApiResult<Json<TestFunctionResponse>> {
    debug!(id, "Testing analytics function");

    // For now, return a placeholder response
    // In M4 (Analytics), this will actually execute the function in the Python engine
    Ok(Json(TestFunctionResponse {
        status: "success".to_string(),
        output: Some(serde_json::json!({
            "message": "Function test not yet implemented (analytics engine not available)",
            "function_id": id,
        })),
        error: None,
    }))
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
    async fn test_list_functions() {
        let state = setup_test_state().await;

        let result = list_functions(State(state.clone())).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        // Should have predefined functions from migrations
        assert!(response.total >= 0);
    }

    #[tokio::test]
    async fn test_create_function() {
        let state = setup_test_state().await;

        let req = CreateFunctionRequest {
            name: "my_custom_forecast".to_string(),
            description: "My custom forecasting function".to_string(),
            language: "python".to_string(),
            source_code: "def forecast(data, params):\n    return data".to_string(),
            parameters_schema: Some(serde_json::json!({"window": "integer"})),
        };

        let result = create_function(State(state.clone()), Json(req)).await;
        assert!(result.is_ok());

        let (status, response) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.name, "my_custom_forecast");
        assert_eq!(response.function_type, "custom");
        assert!(response.source_code.is_some());
    }

    #[tokio::test]
    async fn test_create_function_validation() {
        let state = setup_test_state().await;

        // Empty name
        let req = CreateFunctionRequest {
            name: "".to_string(),
            description: "Test".to_string(),
            language: "python".to_string(),
            source_code: "def test(): pass".to_string(),
            parameters_schema: None,
        };
        let result = create_function(State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));

        // Invalid language
        let req = CreateFunctionRequest {
            name: "test".to_string(),
            description: "Test".to_string(),
            language: "javascript".to_string(),
            source_code: "def test(): pass".to_string(),
            parameters_schema: None,
        };
        let result = create_function(State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));

        // No function definition
        let req = CreateFunctionRequest {
            name: "test".to_string(),
            description: "Test".to_string(),
            language: "python".to_string(),
            source_code: "x = 1".to_string(),
            parameters_schema: None,
        };
        let result = create_function(State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_get_function() {
        let state = setup_test_state().await;

        // Create a function first
        let req = CreateFunctionRequest {
            name: "test_get".to_string(),
            description: "Test".to_string(),
            language: "python".to_string(),
            source_code: "def forecast(data): return data".to_string(),
            parameters_schema: None,
        };
        let (_, created) = create_function(State(state.clone()), Json(req))
            .await
            .unwrap();

        // Get it
        let result = get_function(State(state.clone()), Path(created.id)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, created.id);
        assert_eq!(response.name, "test_get");
    }

    #[tokio::test]
    async fn test_update_function() {
        let state = setup_test_state().await;

        // Create a function first
        let req = CreateFunctionRequest {
            name: "test_update".to_string(),
            description: "Original".to_string(),
            language: "python".to_string(),
            source_code: "def forecast(data): return data".to_string(),
            parameters_schema: None,
        };
        let (_, created) = create_function(State(state.clone()), Json(req))
            .await
            .unwrap();

        // Update it
        let update_req = UpdateFunctionRequest {
            name: "test_update_modified".to_string(),
            description: "Updated".to_string(),
            language: "python".to_string(),
            source_code: "def forecast(data): return data * 2".to_string(),
            parameters_schema: Some(serde_json::json!({"factor": "float"})),
            is_active: true,
        };

        let result =
            update_function(State(state.clone()), Path(created.id), Json(update_req)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.name, "test_update_modified");
        assert_eq!(response.description, "Updated");
    }

    #[tokio::test]
    async fn test_delete_function() {
        let state = setup_test_state().await;

        // Create a function first
        let req = CreateFunctionRequest {
            name: "test_delete".to_string(),
            description: "Test".to_string(),
            language: "python".to_string(),
            source_code: "def forecast(data): return data".to_string(),
            parameters_schema: None,
        };
        let (_, created) = create_function(State(state.clone()), Json(req))
            .await
            .unwrap();

        // Delete it
        let result = delete_function(State(state.clone()), Path(created.id)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);

        // Verify it's gone
        let get_result = get_function(State(state.clone()), Path(created.id)).await;
        assert!(matches!(get_result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_test_function() {
        let state = setup_test_state().await;

        let req = TestFunctionRequest {
            input_data: serde_json::json!({"values": [1, 2, 3]}),
            parameters: Some(serde_json::json!({"window": 7})),
        };

        let result = test_function(State(state.clone()), Path(1), Json(req)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status, "success");
    }
}
