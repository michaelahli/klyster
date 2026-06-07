//! Kubernetes infrastructure provider implementation.

use crate::k8s::{discovery::ResourceDiscovery, init_client, K8sClientError};
use crate::models::Resource;
use crate::provider::{Capacity, InfraProvider};
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};
use kube::{api::Api, Client};
use std::sync::Arc;
use tracing::{debug, warn};

/// Kubernetes provider error.
#[derive(Debug, thiserror::Error)]
pub enum K8sProviderError {
    /// Kubernetes client error.
    #[error("Kubernetes client error: {0}")]
    ClientError(String),
    /// Resource not found.
    #[error("Resource not found: {0}")]
    NotFound(String),
    /// Invalid scale target.
    #[error("Invalid scale target: {0}")]
    InvalidTarget(String),
    /// Invalid group identifier.
    #[error("Invalid group identifier: {0}")]
    InvalidGroupId(String),
}

fn map_kube_get_error(err: kube::Error, resource: impl Into<String>) -> K8sProviderError {
    match err {
        kube::Error::Api(ref api_err) if api_err.code == 404 => {
            K8sProviderError::NotFound(resource.into())
        }
        _ => K8sProviderError::ClientError(err.to_string()),
    }
}

impl From<K8sClientError> for K8sProviderError {
    fn from(err: K8sClientError) -> Self {
        K8sProviderError::ClientError(err.to_string())
    }
}

impl From<kube::Error> for K8sProviderError {
    fn from(err: kube::Error) -> Self {
        K8sProviderError::ClientError(err.to_string())
    }
}

/// Kubernetes infrastructure provider.
pub struct KubernetesProvider {
    client: Arc<Client>,
    namespaces: Vec<String>,
}

impl KubernetesProvider {
    /// Create a new Kubernetes provider.
    ///
    /// # Arguments
    /// * `kubeconfig_path` - Optional path to kubeconfig file
    /// * `namespaces` - List of namespaces to watch (empty = all namespaces)
    ///
    /// # Errors
    /// Returns error if Kubernetes client initialization fails.
    pub async fn new(
        kubeconfig_path: Option<&str>,
        namespaces: Vec<String>,
    ) -> Result<Self, K8sProviderError> {
        let client = init_client(kubeconfig_path).await?;
        Ok(Self {
            client: Arc::new(client),
            namespaces,
        })
    }

    /// Get namespaces to query (empty = all namespaces).
    #[allow(dead_code)]
    fn target_namespaces(&self) -> Vec<&str> {
        if self.namespaces.is_empty() {
            vec![]
        } else {
            self.namespaces
                .iter()
                .map(std::string::String::as_str)
                .collect()
        }
    }
}

#[async_trait::async_trait]
impl InfraProvider for KubernetesProvider {
    type Error = K8sProviderError;

    async fn get_resources(&self) -> Result<Vec<Resource>, Self::Error> {
        debug!("Discovering Kubernetes resources");

        let discovery = ResourceDiscovery::new((*self.client).clone(), self.namespaces.clone());

        discovery.discover_all().await
    }

    async fn get_current_capacity(&self, group_id: &str) -> Result<Capacity, Self::Error> {
        debug!("Getting capacity for group: {}", group_id);

        let target = parse_group_id(group_id)?;
        capacity_for(&self.client, &target).await
    }

    async fn validate_scale_target(&self, group_id: &str, target: u32) -> Result<(), Self::Error> {
        debug!("Validating scale target {} for group: {}", target, group_id);

        // Basic validation: target must be > 0
        if target == 0 {
            return Err(K8sProviderError::InvalidTarget(
                "Target replicas must be greater than 0".to_string(),
            ));
        }

        // TODO: Check against resource quotas, HPA settings, etc.
        warn!("validate_scale_target basic implementation only");

        Ok(())
    }

    fn name(&self) -> &'static str {
        "kubernetes"
    }
}

/// Parsed identifier for a Kubernetes workload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkloadTarget {
    /// Workload kind (deployment, statefulset, daemonset).
    pub kind: WorkloadKind,
    /// Namespace.
    pub namespace: String,
    /// Workload name.
    pub name: String,
}

/// Kubernetes workload kind supported for capacity reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkloadKind {
    /// `Deployment`
    Deployment,
    /// `StatefulSet`
    StatefulSet,
    /// `DaemonSet`
    DaemonSet,
}

/// Parse a group identifier of the form `kind/namespace/name`.
///
/// Examples:
/// - `deployment/default/web`
/// - `statefulset/data/postgres`
/// - `daemonset/kube-system/fluentd`
pub(crate) fn parse_group_id(group_id: &str) -> Result<WorkloadTarget, K8sProviderError> {
    let parts: Vec<&str> = group_id.split('/').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) {
        return Err(K8sProviderError::InvalidGroupId(format!(
            "expected `kind/namespace/name`, got: {group_id}"
        )));
    }

    let kind = match parts[0].to_ascii_lowercase().as_str() {
        "deployment" | "deployments" => WorkloadKind::Deployment,
        "statefulset" | "statefulsets" => WorkloadKind::StatefulSet,
        "daemonset" | "daemonsets" => WorkloadKind::DaemonSet,
        other => {
            return Err(K8sProviderError::InvalidGroupId(format!(
                "unsupported kind `{other}`, expected one of: deployment, statefulset, daemonset"
            )));
        }
    };

    Ok(WorkloadTarget {
        kind,
        namespace: parts[1].to_string(),
        name: parts[2].to_string(),
    })
}

/// Read capacity for a single Kubernetes workload.
pub(crate) async fn capacity_for(
    client: &Client,
    target: &WorkloadTarget,
) -> Result<Capacity, K8sProviderError> {
    match target.kind {
        WorkloadKind::Deployment => {
            deployment_capacity(client, &target.namespace, &target.name).await
        }
        WorkloadKind::StatefulSet => {
            statefulset_capacity(client, &target.namespace, &target.name).await
        }
        WorkloadKind::DaemonSet => {
            daemonset_capacity(client, &target.namespace, &target.name).await
        }
    }
}

async fn deployment_capacity(
    client: &Client,
    namespace: &str,
    name: &str,
) -> Result<Capacity, K8sProviderError> {
    let api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    let dep = api
        .get(name)
        .await
        .map_err(|e| map_kube_get_error(e, format!("deployment/{namespace}/{name}")))?;

    let desired = dep
        .spec
        .as_ref()
        .and_then(|s| s.replicas)
        .and_then(|r| u32::try_from(r).ok())
        .unwrap_or(0);
    let current = dep
        .status
        .as_ref()
        .and_then(|s| s.replicas)
        .and_then(|r| u32::try_from(r).ok())
        .unwrap_or(0);

    Ok(Capacity {
        current,
        desired,
        min: 0,
        max: desired.max(current).max(1),
    })
}

async fn statefulset_capacity(
    client: &Client,
    namespace: &str,
    name: &str,
) -> Result<Capacity, K8sProviderError> {
    let api: Api<StatefulSet> = Api::namespaced(client.clone(), namespace);
    let ss = api
        .get(name)
        .await
        .map_err(|e| map_kube_get_error(e, format!("statefulset/{namespace}/{name}")))?;

    let desired = ss
        .spec
        .as_ref()
        .and_then(|s| s.replicas)
        .and_then(|r| u32::try_from(r).ok())
        .unwrap_or(0);
    let current = ss
        .status
        .as_ref()
        .map(|s| s.replicas)
        .and_then(|r| u32::try_from(r).ok())
        .unwrap_or(0);

    Ok(Capacity {
        current,
        desired,
        min: 0,
        max: desired.max(current).max(1),
    })
}

async fn daemonset_capacity(
    client: &Client,
    namespace: &str,
    name: &str,
) -> Result<Capacity, K8sProviderError> {
    let api: Api<DaemonSet> = Api::namespaced(client.clone(), namespace);
    let ds = api
        .get(name)
        .await
        .map_err(|e| map_kube_get_error(e, format!("daemonset/{namespace}/{name}")))?;

    // DaemonSet capacity is the number of nodes it should run on.
    let desired = ds
        .status
        .as_ref()
        .map(|s| s.desired_number_scheduled)
        .and_then(|r| u32::try_from(r).ok())
        .unwrap_or(0);
    let current = ds
        .status
        .as_ref()
        .map(|s| s.number_ready)
        .and_then(|r| u32::try_from(r).ok())
        .unwrap_or(0);

    Ok(Capacity {
        current,
        desired,
        // DaemonSet is not user-scalable; min/max equal desired.
        min: desired,
        max: desired,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_group_id_accepts_supported_workloads() {
        let deployment = parse_group_id("deployment/default/web").unwrap();
        assert_eq!(deployment.kind, WorkloadKind::Deployment);
        assert_eq!(deployment.namespace, "default");
        assert_eq!(deployment.name, "web");

        let statefulset = parse_group_id("statefulsets/data/postgres").unwrap();
        assert_eq!(statefulset.kind, WorkloadKind::StatefulSet);

        let daemonset = parse_group_id("DaemonSet/kube-system/fluentd").unwrap();
        assert_eq!(daemonset.kind, WorkloadKind::DaemonSet);
    }

    #[test]
    fn parse_group_id_rejects_invalid_shapes() {
        assert!(matches!(
            parse_group_id("default/web"),
            Err(K8sProviderError::InvalidGroupId(_))
        ));
        assert!(matches!(
            parse_group_id("deployment//web"),
            Err(K8sProviderError::InvalidGroupId(_))
        ));
        assert!(matches!(
            parse_group_id("job/default/batch"),
            Err(K8sProviderError::InvalidGroupId(_))
        ));
    }
}
