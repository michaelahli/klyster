//! Resource group CRUD endpoints.

use crate::dto::resource_groups::{
    CreateResourceGroupRequest, ResourceGroupDetailResponse, ResourceGroupListResponse,
    ResourceGroupResponse, ResourceResponse, ScalingTargetResponse, SetScalingTargetRequest,
    UpdateResourceGroupRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use db::repositories::ResourceRepository;
use domain::models::{ResourceGroup, ScalingTarget};
use tracing::debug;

/// Create a new resource group.
///
/// POST /api/v1/resource-groups
pub async fn create_group(
    State(state): State<AppState>,
    Json(req): Json<CreateResourceGroupRequest>,
) -> ApiResult<(StatusCode, Json<ResourceGroupResponse>)> {
    debug!(name = %req.name, provider_type = %req.provider_type, "Creating resource group");

    // Validate name is not empty
    if req.name.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Group name cannot be empty".to_string(),
        ));
    }

    // Validate provider type
    let valid_providers = ["kubernetes", "vm", "cloud"];
    if !valid_providers.contains(&req.provider_type.as_str()) {
        return Err(ApiError::ValidationError(format!(
            "Invalid provider type '{}'. Must be one of: {}",
            req.provider_type,
            valid_providers.join(", ")
        )));
    }

    // Serialize config to JSON string
    let config_str = serde_json::to_string(&req.provider_config)
        .map_err(|e| ApiError::ValidationError(format!("Invalid config JSON: {e}")))?;

    let group = ResourceGroup {
        id: 0,
        name: req.name,
        description: req.description,
        provider_type: req.provider_type,
        provider_config: config_str,
        created_at: Utc::now(),
    };

    let repo = ResourceRepository::new(state.db());
    let id = repo.create_group(&group).await?;

    let created = repo
        .get_group(id)
        .await?
        .ok_or_else(|| ApiError::Internal("Failed to retrieve created group".to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(ResourceGroupResponse::from_model(created)),
    ))
}

/// List all resource groups.
///
/// GET /api/v1/resource-groups
pub async fn list_groups(
    State(state): State<AppState>,
) -> ApiResult<Json<ResourceGroupListResponse>> {
    debug!("Listing resource groups");

    let repo = ResourceRepository::new(state.db());
    let groups = repo.list_groups().await?;

    let total = groups.len();
    let groups = groups
        .into_iter()
        .map(ResourceGroupResponse::from_model)
        .collect();

    Ok(Json(ResourceGroupListResponse { groups, total }))
}

/// Get a resource group by ID with details.
///
/// GET /api/v1/resource-groups/:id
pub async fn get_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<ResourceGroupDetailResponse>> {
    debug!(id, "Getting resource group");

    let repo = ResourceRepository::new(state.db());

    let group = repo
        .get_group(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Resource group {id} not found")))?;

    let resources = repo.list_by_group(id).await?;
    let scaling_targets = repo.get_scaling_targets_by_group(id).await?;

    Ok(Json(ResourceGroupDetailResponse {
        group: ResourceGroupResponse::from_model(group),
        resources: resources
            .into_iter()
            .map(ResourceResponse::from_model)
            .collect(),
        scaling_targets: scaling_targets
            .into_iter()
            .map(ScalingTargetResponse::from_model)
            .collect(),
    }))
}

/// Update a resource group.
///
/// PUT /api/v1/resource-groups/:id
pub async fn update_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateResourceGroupRequest>,
) -> ApiResult<Json<ResourceGroupResponse>> {
    debug!(id, name = %req.name, "Updating resource group");

    // Validate name is not empty
    if req.name.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Group name cannot be empty".to_string(),
        ));
    }

    // Validate provider type
    let valid_providers = ["kubernetes", "vm", "cloud"];
    if !valid_providers.contains(&req.provider_type.as_str()) {
        return Err(ApiError::ValidationError(format!(
            "Invalid provider type '{}'. Must be one of: {}",
            req.provider_type,
            valid_providers.join(", ")
        )));
    }

    let repo = ResourceRepository::new(state.db());

    // Check if group exists
    let existing = repo
        .get_group(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Resource group {id} not found")))?;

    // Serialize config to JSON string
    let config_str = serde_json::to_string(&req.provider_config)
        .map_err(|e| ApiError::ValidationError(format!("Invalid config JSON: {e}")))?;

    let updated_group = ResourceGroup {
        id,
        name: req.name,
        description: req.description,
        provider_type: req.provider_type,
        provider_config: config_str,
        created_at: existing.created_at,
    };

    repo.update_group(&updated_group).await?;

    let updated = repo
        .get_group(id)
        .await?
        .ok_or_else(|| ApiError::Internal("Failed to retrieve updated group".to_string()))?;

    Ok(Json(ResourceGroupResponse::from_model(updated)))
}

/// Delete a resource group.
///
/// DELETE /api/v1/resource-groups/:id
pub async fn delete_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    debug!(id, "Deleting resource group");

    let repo = ResourceRepository::new(state.db());

    // Check if group exists
    if repo.get_group(id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Resource group {id} not found")));
    }

    let rows = repo.delete_group(id).await?;

    if rows == 0 {
        return Err(ApiError::NotFound(format!("Resource group {id} not found")));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Set a scaling target for a resource group.
///
/// POST /api/v1/resource-groups/:id/scaling-targets
pub async fn set_scaling_target(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<SetScalingTargetRequest>,
) -> ApiResult<(StatusCode, Json<ScalingTargetResponse>)> {
    debug!(
        group_id = id,
        metric_name = %req.metric_name,
        "Setting scaling target"
    );

    // Validate metric name
    if req.metric_name.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Metric name cannot be empty".to_string(),
        ));
    }

    // Validate replicas
    if req.min_replicas < 0 {
        return Err(ApiError::ValidationError(
            "min_replicas must be >= 0".to_string(),
        ));
    }

    if req.max_replicas < req.min_replicas {
        return Err(ApiError::ValidationError(
            "max_replicas must be >= min_replicas".to_string(),
        ));
    }

    // Validate target value
    if req.target_value <= 0.0 {
        return Err(ApiError::ValidationError(
            "target_value must be > 0".to_string(),
        ));
    }

    let repo = ResourceRepository::new(state.db());

    // Check if group exists
    if repo.get_group(id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Resource group {id} not found")));
    }

    let target = ScalingTarget {
        id: 0,
        resource_group_id: id,
        metric_name: req.metric_name,
        min_replicas: req.min_replicas,
        max_replicas: req.max_replicas,
        target_value: req.target_value,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let target_id = repo.set_scaling_target(&target).await?;

    // Retrieve the created/updated target
    let targets = repo.get_scaling_targets_by_group(id).await?;
    let created_target = targets
        .into_iter()
        .find(|t| t.id == target_id)
        .ok_or_else(|| ApiError::Internal("Failed to retrieve scaling target".to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(ScalingTargetResponse::from_model(created_target)),
    ))
}

/// Get resources in a resource group.
///
/// GET /api/v1/resource-groups/:id/resources
pub async fn list_resources(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Vec<ResourceResponse>>> {
    debug!(group_id = id, "Listing resources in group");

    let repo = ResourceRepository::new(state.db());

    // Check if group exists
    if repo.get_group(id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Resource group {id} not found")));
    }

    let resources = repo.list_by_group(id).await?;

    Ok(Json(
        resources
            .into_iter()
            .map(ResourceResponse::from_model)
            .collect(),
    ))
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
    async fn test_create_group() {
        let state = setup_test_state().await;

        let req = CreateResourceGroupRequest {
            name: "test-cluster".to_string(),
            description: Some("Test cluster".to_string()),
            provider_type: "kubernetes".to_string(),
            provider_config: serde_json::json!({"endpoint": "https://k8s.example.com"}),
        };

        let result = create_group(State(state.clone()), Json(req)).await;
        assert!(result.is_ok());

        let (status, response) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.name, "test-cluster");
        assert_eq!(response.provider_type, "kubernetes");
    }

    #[tokio::test]
    async fn test_create_group_validation() {
        let state = setup_test_state().await;

        // Empty name
        let req = CreateResourceGroupRequest {
            name: String::new(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: serde_json::json!({}),
        };

        let result = create_group(State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));

        // Invalid provider type
        let req = CreateResourceGroupRequest {
            name: "test".to_string(),
            description: None,
            provider_type: "invalid".to_string(),
            provider_config: serde_json::json!({}),
        };

        let result = create_group(State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_list_groups() {
        let state = setup_test_state().await;

        // Create a group first
        let req = CreateResourceGroupRequest {
            name: "test-cluster".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: serde_json::json!({}),
        };
        let _result = create_group(State(state.clone()), Json(req)).await.unwrap();

        let result = list_groups(State(state.clone())).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.total, 1);
        assert_eq!(response.groups.len(), 1);
    }

    #[tokio::test]
    async fn test_get_group() {
        let state = setup_test_state().await;

        // Create a group first
        let req = CreateResourceGroupRequest {
            name: "test-cluster".to_string(),
            description: Some("Test".to_string()),
            provider_type: "kubernetes".to_string(),
            provider_config: serde_json::json!({}),
        };
        let (_, created) = create_group(State(state.clone()), Json(req)).await.unwrap();

        let result = get_group(State(state.clone()), Path(created.id)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.group.id, created.id);
        assert_eq!(response.group.name, "test-cluster");
        assert_eq!(response.resources.len(), 0);
        assert_eq!(response.scaling_targets.len(), 0);
    }

    #[tokio::test]
    async fn test_update_group() {
        let state = setup_test_state().await;

        // Create a group first
        let req = CreateResourceGroupRequest {
            name: "test-cluster".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: serde_json::json!({}),
        };
        let (_, created) = create_group(State(state.clone()), Json(req)).await.unwrap();

        // Update it
        let update_req = UpdateResourceGroupRequest {
            name: "updated-cluster".to_string(),
            description: Some("Updated".to_string()),
            provider_type: "kubernetes".to_string(),
            provider_config: serde_json::json!({"new": "config"}),
        };

        let result = update_group(State(state.clone()), Path(created.id), Json(update_req)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.name, "updated-cluster");
        assert_eq!(response.description, Some("Updated".to_string()));
    }

    #[tokio::test]
    async fn test_delete_group() {
        let state = setup_test_state().await;

        // Create a group first
        let req = CreateResourceGroupRequest {
            name: "test-cluster".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: serde_json::json!({}),
        };
        let (_, created) = create_group(State(state.clone()), Json(req)).await.unwrap();

        // Delete it
        let result = delete_group(State(state.clone()), Path(created.id)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);

        // Verify it's gone
        let get_result = get_group(State(state.clone()), Path(created.id)).await;
        assert!(matches!(get_result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_set_scaling_target() {
        let state = setup_test_state().await;

        // Create a group first
        let req = CreateResourceGroupRequest {
            name: "test-cluster".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: serde_json::json!({}),
        };
        let (_, created) = create_group(State(state.clone()), Json(req)).await.unwrap();

        // Set scaling target
        let target_req = SetScalingTargetRequest {
            metric_name: "cpu_usage".to_string(),
            min_replicas: 2,
            max_replicas: 10,
            target_value: 0.7,
        };

        let result =
            set_scaling_target(State(state.clone()), Path(created.id), Json(target_req)).await;
        assert!(result.is_ok());

        let (status, response) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.metric_name, "cpu_usage");
        assert_eq!(response.min_replicas, 2);
        assert_eq!(response.max_replicas, 10);
    }

    #[tokio::test]
    async fn test_set_scaling_target_validation() {
        let state = setup_test_state().await;

        // Create a group first
        let req = CreateResourceGroupRequest {
            name: "test-cluster".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: serde_json::json!({}),
        };
        let (_, created) = create_group(State(state.clone()), Json(req)).await.unwrap();

        // Invalid: max < min
        let target_req = SetScalingTargetRequest {
            metric_name: "cpu_usage".to_string(),
            min_replicas: 10,
            max_replicas: 2,
            target_value: 0.7,
        };

        let result =
            set_scaling_target(State(state.clone()), Path(created.id), Json(target_req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));

        // Invalid: negative target value
        let target_req = SetScalingTargetRequest {
            metric_name: "cpu_usage".to_string(),
            min_replicas: 2,
            max_replicas: 10,
            target_value: -0.5,
        };

        let result =
            set_scaling_target(State(state.clone()), Path(created.id), Json(target_req)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));
    }
}
