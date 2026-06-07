//! Resource group CRUD endpoints.

use crate::dto::resource_groups::{
    CreateResourceGroupRequest, ResourceGroupCapacityResponse, ResourceGroupDetailResponse,
    ResourceGroupListResponse, ResourceGroupResponse, ResourceResponse, ScalingTargetResponse,
    SetScalingTargetRequest, UpdateResourceGroupRequest, WorkloadCapacityResponse,
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
use domain::provider::kubernetes::{K8sProviderError, KubernetesProvider};
use domain::provider::{Capacity, InfraProvider};
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

/// Get current capacity for a resource group.
///
/// GET /api/v1/resource-groups/:id/capacity
pub async fn get_capacity(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<ResourceGroupCapacityResponse>> {
    debug!(group_id = id, "Getting resource group capacity");

    let repo = ResourceRepository::new(state.db());
    let group = repo
        .get_group(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Resource group {id} not found")))?;

    if group.provider_type != "kubernetes" {
        return Err(ApiError::ValidationError(format!(
            "Capacity is not supported for provider type '{}'",
            group.provider_type
        )));
    }

    if !state.config().kubernetes.enabled {
        return Err(ApiError::ValidationError(
            "Kubernetes integration is disabled".to_string(),
        ));
    }

    let targets = capacity_targets(&group.provider_config)?;
    let provider = KubernetesProvider::new(
        state.config().kubernetes.kubeconfig_path.as_deref(),
        state.config().kubernetes.namespaces.clone(),
    )
    .await
    .map_err(provider_error_to_api)?;

    let mut entries = Vec::with_capacity(targets.len());
    let mut capacities = Vec::with_capacity(targets.len());
    for target in targets {
        let capacity = provider
            .get_current_capacity(&target)
            .await
            .map_err(provider_error_to_api)?;
        entries.push(WorkloadCapacityResponse::from_capacity(target, &capacity));
        capacities.push(capacity);
    }

    let aggregate = aggregate_capacity(&capacities);
    Ok(Json(ResourceGroupCapacityResponse::from_capacity(
        id, &aggregate, entries,
    )))
}

fn capacity_targets(provider_config: &str) -> ApiResult<Vec<String>> {
    let config: serde_json::Value = serde_json::from_str(provider_config)
        .map_err(|e| ApiError::ValidationError(format!("Invalid provider config JSON: {e}")))?;

    let targets = if let Some(targets) = config.get("capacity_targets").and_then(|v| v.as_array()) {
        targets
            .iter()
            .map(|target| {
                target.as_str().map(str::to_string).ok_or_else(|| {
                    ApiError::ValidationError(
                        "provider_config.capacity_targets must contain only strings".to_string(),
                    )
                })
            })
            .collect::<ApiResult<Vec<_>>>()?
    } else if let Some(target) = config.get("capacity_target").and_then(|v| v.as_str()) {
        vec![target.to_string()]
    } else if let (Some(kind), Some(namespace), Some(name)) = (
        config.get("kind").and_then(|v| v.as_str()),
        config.get("namespace").and_then(|v| v.as_str()),
        config.get("name").and_then(|v| v.as_str()),
    ) {
        vec![format!("{kind}/{namespace}/{name}")]
    } else {
        return Err(ApiError::ValidationError(
            "provider_config must include capacity_target, capacity_targets, or kind/namespace/name"
                .to_string(),
        ));
    };

    if targets.is_empty() || targets.iter().any(|target| target.trim().is_empty()) {
        return Err(ApiError::ValidationError(
            "At least one non-empty capacity target is required".to_string(),
        ));
    }

    Ok(targets)
}

fn aggregate_capacity(capacities: &[Capacity]) -> Capacity {
    capacities.iter().fold(
        Capacity {
            current: 0,
            desired: 0,
            min: 0,
            max: 0,
        },
        |mut aggregate, capacity| {
            aggregate.current += capacity.current;
            aggregate.desired += capacity.desired;
            aggregate.min += capacity.min;
            aggregate.max += capacity.max;
            aggregate
        },
    )
}

fn provider_error_to_api(err: K8sProviderError) -> ApiError {
    match err {
        K8sProviderError::NotFound(msg) => ApiError::NotFound(msg),
        K8sProviderError::InvalidGroupId(msg) | K8sProviderError::InvalidTarget(msg) => {
            ApiError::ValidationError(msg)
        }
        K8sProviderError::ClientError(msg) => ApiError::Internal(msg),
    }
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
            kubernetes: domain::config::KubernetesConfig::default(),
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

    #[tokio::test]
    async fn test_get_capacity_requires_existing_group() {
        let state = setup_test_state().await;

        let result = get_capacity(State(state), Path(999)).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_capacity_requires_kubernetes_enabled() {
        let state = setup_test_state().await;
        let req = CreateResourceGroupRequest {
            name: "test-cluster".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: serde_json::json!({
                "capacity_target": "deployment/default/web"
            }),
        };
        let (_, created) = create_group(State(state.clone()), Json(req)).await.unwrap();

        let result = get_capacity(State(state), Path(created.id)).await;
        assert!(matches!(result, Err(ApiError::ValidationError(_))));
    }

    #[test]
    fn test_capacity_targets_from_single_target() {
        let config = serde_json::json!({
            "capacity_target": "deployment/default/web"
        });

        let targets = capacity_targets(&config.to_string()).unwrap();
        assert_eq!(targets, vec!["deployment/default/web"]);
    }

    #[test]
    fn test_capacity_targets_from_multiple_targets() {
        let config = serde_json::json!({
            "capacity_targets": [
                "deployment/default/web",
                "statefulset/data/postgres"
            ]
        });

        let targets = capacity_targets(&config.to_string()).unwrap();
        assert_eq!(
            targets,
            vec![
                "deployment/default/web".to_string(),
                "statefulset/data/postgres".to_string()
            ]
        );
    }

    #[test]
    fn test_capacity_targets_from_parts() {
        let config = serde_json::json!({
            "kind": "daemonset",
            "namespace": "kube-system",
            "name": "fluentd"
        });

        let targets = capacity_targets(&config.to_string()).unwrap();
        assert_eq!(targets, vec!["daemonset/kube-system/fluentd"]);
    }

    #[test]
    fn test_capacity_targets_rejects_missing_target() {
        let config = serde_json::json!({});
        let result = capacity_targets(&config.to_string());
        assert!(matches!(result, Err(ApiError::ValidationError(_))));
    }

    #[test]
    fn test_aggregate_capacity_sums_values_and_drift() {
        let aggregate = aggregate_capacity(&[
            Capacity {
                current: 2,
                desired: 3,
                min: 1,
                max: 5,
            },
            Capacity {
                current: 4,
                desired: 4,
                min: 2,
                max: 8,
            },
        ]);

        assert_eq!(aggregate.current, 6);
        assert_eq!(aggregate.desired, 7);
        assert_eq!(aggregate.min, 3);
        assert_eq!(aggregate.max, 13);
        assert!(aggregate.has_drift());
    }
}
