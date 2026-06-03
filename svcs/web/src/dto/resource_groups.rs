//! DTOs for resource group endpoints.

use serde::{Deserialize, Serialize};

/// Request to create a new resource group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResourceGroupRequest {
    /// Group name (must be unique).
    pub name: String,
    /// Description (optional).
    pub description: Option<String>,
    /// Provider type: "kubernetes", "vm", "cloud".
    pub provider_type: String,
    /// JSON configuration for the provider.
    pub provider_config: serde_json::Value,
}

/// Request to update an existing resource group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResourceGroupRequest {
    /// Group name (must be unique).
    pub name: String,
    /// Description (optional).
    pub description: Option<String>,
    /// Provider type: "kubernetes", "vm", "cloud".
    pub provider_type: String,
    /// JSON configuration for the provider.
    pub provider_config: serde_json::Value,
}

/// Response for a resource group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGroupResponse {
    /// Group ID.
    pub id: i64,
    /// Group name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Provider type.
    pub provider_type: String,
    /// JSON configuration.
    pub provider_config: serde_json::Value,
    /// Creation timestamp (ISO8601).
    pub created_at: String,
}

impl ResourceGroupResponse {
    /// Convert from domain model.
    #[must_use]
    pub fn from_model(group: domain::models::ResourceGroup) -> Self {
        let config: serde_json::Value = serde_json::from_str(&group.provider_config)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        Self {
            id: group.id,
            name: group.name,
            description: group.description,
            provider_type: group.provider_type,
            provider_config: config,
            created_at: group.created_at.to_rfc3339(),
        }
    }
}

/// Response for resource group list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGroupListResponse {
    /// List of resource groups.
    pub groups: Vec<ResourceGroupResponse>,
    /// Total count.
    pub total: usize,
}

/// Response for a resource group with details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGroupDetailResponse {
    /// Group information.
    #[serde(flatten)]
    pub group: ResourceGroupResponse,
    /// Resources in this group.
    pub resources: Vec<ResourceResponse>,
    /// Scaling targets for this group.
    pub scaling_targets: Vec<ScalingTargetResponse>,
}

/// Response for a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceResponse {
    /// Resource ID.
    pub id: i64,
    /// Group ID.
    pub group_id: i64,
    /// Resource name.
    pub name: String,
    /// Namespace (optional, for Kubernetes).
    pub namespace: Option<String>,
    /// Resource kind.
    pub kind: String,
    /// JSON labels.
    pub labels: Option<serde_json::Value>,
    /// Resource status.
    pub status: String,
    /// Creation timestamp (ISO8601).
    pub created_at: String,
    /// Last update timestamp (ISO8601).
    pub updated_at: String,
}

impl ResourceResponse {
    /// Convert from domain model.
    #[must_use]
    pub fn from_model(resource: domain::models::Resource) -> Self {
        let labels: Option<serde_json::Value> =
            resource.labels.and_then(|l| serde_json::from_str(&l).ok());

        Self {
            id: resource.id,
            group_id: resource.group_id,
            name: resource.name,
            namespace: resource.namespace,
            kind: resource.kind,
            labels,
            status: resource.status,
            created_at: resource.created_at.to_rfc3339(),
            updated_at: resource.updated_at.to_rfc3339(),
        }
    }
}

/// Request to set a scaling target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetScalingTargetRequest {
    /// Metric name to track.
    pub metric_name: String,
    /// Minimum replicas.
    pub min_replicas: i32,
    /// Maximum replicas.
    pub max_replicas: i32,
    /// Target value for the metric.
    pub target_value: f64,
}

/// Response for a scaling target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingTargetResponse {
    /// Target ID.
    pub id: i64,
    /// Resource group ID.
    pub resource_group_id: i64,
    /// Metric name.
    pub metric_name: String,
    /// Minimum replicas.
    pub min_replicas: i32,
    /// Maximum replicas.
    pub max_replicas: i32,
    /// Target value.
    pub target_value: f64,
    /// Creation timestamp (ISO8601).
    pub created_at: String,
    /// Last update timestamp (ISO8601).
    pub updated_at: String,
}

impl ScalingTargetResponse {
    /// Convert from domain model.
    #[must_use]
    pub fn from_model(target: domain::models::ScalingTarget) -> Self {
        Self {
            id: target.id,
            resource_group_id: target.resource_group_id,
            metric_name: target.metric_name,
            min_replicas: target.min_replicas,
            max_replicas: target.max_replicas,
            target_value: target.target_value,
            created_at: target.created_at.to_rfc3339(),
            updated_at: target.updated_at.to_rfc3339(),
        }
    }
}
