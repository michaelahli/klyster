//! Resource domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;

/// Resource kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceKind {
    /// Kubernetes pod
    Pod,
    /// Virtual machine
    Vm,
    /// Node
    Node,
    /// Kubernetes deployment
    Deployment,
    /// Kubernetes statefulset
    Statefulset,
}

impl ResourceKind {
    /// Convert to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceKind::Pod => "pod",
            ResourceKind::Vm => "vm",
            ResourceKind::Node => "node",
            ResourceKind::Deployment => "deployment",
            ResourceKind::Statefulset => "statefulset",
        }
    }
}

impl FromStr for ResourceKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pod" => Ok(ResourceKind::Pod),
            "vm" => Ok(ResourceKind::Vm),
            "node" => Ok(ResourceKind::Node),
            "deployment" => Ok(ResourceKind::Deployment),
            "statefulset" => Ok(ResourceKind::Statefulset),
            _ => Err(format!("Invalid resource kind: {s}")),
        }
    }
}

/// Resource group (cluster, namespace, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResourceGroup {
    /// Unique identifier
    pub id: i64,
    /// Group name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Provider type (kubernetes, vm, cloud)
    pub provider_type: String,
    /// JSON provider configuration
    pub provider_config: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Resource (pod, VM, node, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Resource {
    /// Unique identifier
    pub id: i64,
    /// Resource group ID reference
    pub group_id: i64,
    /// Resource name
    pub name: String,
    /// Namespace (optional, for Kubernetes)
    pub namespace: Option<String>,
    /// Resource kind
    pub kind: String,
    /// JSON labels
    pub labels: Option<String>,
    /// Resource status (active, inactive, deleted)
    pub status: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Resource {
    /// Get the resource kind as enum.
    pub fn get_kind(&self) -> Option<ResourceKind> {
        self.kind.parse().ok()
    }
}

/// Scaling target configuration.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScalingTarget {
    /// Unique identifier
    pub id: i64,
    /// Resource group ID reference
    pub resource_group_id: i64,
    /// Metric name to track
    pub metric_name: String,
    /// Minimum replicas
    pub min_replicas: i32,
    /// Maximum replicas
    pub max_replicas: i32,
    /// Target value for the metric
    pub target_value: f64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_kind_serialization() {
        let pod = ResourceKind::Pod;
        let json = serde_json::to_string(&pod).unwrap();
        assert_eq!(json, "\"pod\"");

        let vm = ResourceKind::Vm;
        let json = serde_json::to_string(&vm).unwrap();
        assert_eq!(json, "\"vm\"");
    }

    #[test]
    fn test_resource_kind_deserialization() {
        let pod: ResourceKind = serde_json::from_str("\"pod\"").unwrap();
        assert_eq!(pod, ResourceKind::Pod);

        let deployment: ResourceKind = serde_json::from_str("\"deployment\"").unwrap();
        assert_eq!(deployment, ResourceKind::Deployment);
    }

    #[test]
    fn test_resource_kind_as_str() {
        assert_eq!(ResourceKind::Pod.as_str(), "pod");
        assert_eq!(ResourceKind::Vm.as_str(), "vm");
        assert_eq!(ResourceKind::Node.as_str(), "node");
        assert_eq!(ResourceKind::Deployment.as_str(), "deployment");
        assert_eq!(ResourceKind::Statefulset.as_str(), "statefulset");
    }

    #[test]
    fn test_resource_kind_from_str() {
        assert_eq!("pod".parse::<ResourceKind>().unwrap(), ResourceKind::Pod);
        assert_eq!("vm".parse::<ResourceKind>().unwrap(), ResourceKind::Vm);
        assert_eq!("node".parse::<ResourceKind>().unwrap(), ResourceKind::Node);
        assert_eq!(
            "deployment".parse::<ResourceKind>().unwrap(),
            ResourceKind::Deployment
        );
        assert_eq!(
            "statefulset".parse::<ResourceKind>().unwrap(),
            ResourceKind::Statefulset
        );
        assert!("invalid".parse::<ResourceKind>().is_err());
    }
}
