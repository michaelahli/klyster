//! Kubernetes resource discovery.

use crate::models::{Resource, ResourceKind};
use crate::provider::kubernetes::K8sProviderError;
use chrono::Utc;
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};
use kube::{
    api::{Api, ListParams},
    Client, ResourceExt,
};
use serde_json::Value;
use tracing::{debug, info};

/// Discover Kubernetes workload resources.
#[allow(dead_code)]
pub struct ResourceDiscovery {
    client: Client,
    namespaces: Vec<String>,
}

impl ResourceDiscovery {
    /// Create a new resource discovery instance.
    pub fn new(client: Client, namespaces: Vec<String>) -> Self {
        Self { client, namespaces }
    }

    /// Discover all workload resources (Deployments, `StatefulSets`, `DaemonSets`).
    pub async fn discover_all(&self) -> Result<Vec<Resource>, K8sProviderError> {
        debug!(namespaces = ?self.namespaces, "Starting resource discovery");
        let mut resources = Vec::new();

        let deployments = self.discover_deployments().await?;
        info!(
            count = deployments.len(),
            "Discovered Kubernetes Deployments"
        );
        resources.extend(deployments);

        let statefulsets = self.discover_statefulsets().await?;
        info!(
            count = statefulsets.len(),
            "Discovered Kubernetes StatefulSets"
        );
        resources.extend(statefulsets);

        let daemonsets = self.discover_daemonsets().await?;
        info!(count = daemonsets.len(), "Discovered Kubernetes DaemonSets");
        resources.extend(daemonsets);

        info!(
            count = resources.len(),
            "Kubernetes resource discovery complete"
        );
        Ok(resources)
    }

    /// Get target namespaces for discovery.
    #[allow(dead_code)]
    fn target_namespaces(&self) -> Vec<String> {
        self.namespaces.clone()
    }

    async fn discover_deployments(&self) -> Result<Vec<Resource>, K8sProviderError> {
        if self.namespaces.is_empty() {
            let api: Api<Deployment> = Api::all(self.client.clone());
            let list = api.list(&ListParams::default()).await?;
            return Ok(list.iter().map(deployment_to_resource).collect());
        }

        let mut resources = Vec::new();
        for namespace in &self.namespaces {
            let api: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
            let list = api.list(&ListParams::default()).await?;
            resources.extend(list.iter().map(deployment_to_resource));
        }
        Ok(resources)
    }

    async fn discover_statefulsets(&self) -> Result<Vec<Resource>, K8sProviderError> {
        if self.namespaces.is_empty() {
            let api: Api<StatefulSet> = Api::all(self.client.clone());
            let list = api.list(&ListParams::default()).await?;
            return Ok(list.iter().map(statefulset_to_resource).collect());
        }

        let mut resources = Vec::new();
        for namespace in &self.namespaces {
            let api: Api<StatefulSet> = Api::namespaced(self.client.clone(), namespace);
            let list = api.list(&ListParams::default()).await?;
            resources.extend(list.iter().map(statefulset_to_resource));
        }
        Ok(resources)
    }

    async fn discover_daemonsets(&self) -> Result<Vec<Resource>, K8sProviderError> {
        if self.namespaces.is_empty() {
            let api: Api<DaemonSet> = Api::all(self.client.clone());
            let list = api.list(&ListParams::default()).await?;
            return Ok(list.iter().map(daemonset_to_resource).collect());
        }

        let mut resources = Vec::new();
        for namespace in &self.namespaces {
            let api: Api<DaemonSet> = Api::namespaced(self.client.clone(), namespace);
            let list = api.list(&ListParams::default()).await?;
            resources.extend(list.iter().map(daemonset_to_resource));
        }
        Ok(resources)
    }
}

fn deployment_to_resource(deployment: &Deployment) -> Resource {
    workload_to_resource(
        deployment.name_any(),
        deployment.namespace(),
        ResourceKind::Deployment,
        labels_json(deployment.labels()),
    )
}

fn statefulset_to_resource(statefulset: &StatefulSet) -> Resource {
    workload_to_resource(
        statefulset.name_any(),
        statefulset.namespace(),
        ResourceKind::Statefulset,
        labels_json(statefulset.labels()),
    )
}

fn daemonset_to_resource(daemonset: &DaemonSet) -> Resource {
    workload_to_resource(
        daemonset.name_any(),
        daemonset.namespace(),
        ResourceKind::Daemonset,
        labels_json(daemonset.labels()),
    )
}

fn workload_to_resource(
    name: String,
    namespace: Option<String>,
    kind: ResourceKind,
    labels: Option<String>,
) -> Resource {
    let now = Utc::now();
    Resource {
        id: 0,
        group_id: 0,
        name,
        namespace,
        kind: kind.as_str().to_string(),
        labels,
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
    }
}

fn labels_json(labels: &std::collections::BTreeMap<String, String>) -> Option<String> {
    if labels.is_empty() {
        return None;
    }

    serde_json::to_string(&Value::Object(
        labels
            .iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect(),
    ))
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

    #[tokio::test]
    async fn target_namespaces_returns_configured_namespaces() {
        let discovery = ResourceDiscovery::new(dummy_client(), vec!["default".to_string()]);
        assert_eq!(discovery.target_namespaces(), vec!["default"]);
    }

    #[test]
    fn deployment_conversion_preserves_identity_and_labels() {
        let deployment = Deployment {
            metadata: metadata("web", "default", [("app", "frontend")]),
            ..Default::default()
        };

        let resource = deployment_to_resource(&deployment);

        assert_eq!(resource.name, "web");
        assert_eq!(resource.namespace, Some("default".to_string()));
        assert_eq!(resource.kind, "deployment");
        assert_eq!(resource.status, "active");
        assert_eq!(resource.labels, Some(r#"{"app":"frontend"}"#.to_string()));
    }

    #[test]
    fn statefulset_conversion_uses_statefulset_kind() {
        let statefulset = StatefulSet {
            metadata: metadata("postgres", "data", []),
            ..Default::default()
        };

        let resource = statefulset_to_resource(&statefulset);

        assert_eq!(resource.name, "postgres");
        assert_eq!(resource.namespace, Some("data".to_string()));
        assert_eq!(resource.kind, "statefulset");
        assert_eq!(resource.labels, None);
    }

    #[test]
    fn daemonset_conversion_uses_daemonset_kind() {
        let daemonset = DaemonSet {
            metadata: metadata("fluentd", "kube-system", []),
            ..Default::default()
        };

        let resource = daemonset_to_resource(&daemonset);

        assert_eq!(resource.name, "fluentd");
        assert_eq!(resource.namespace, Some("kube-system".to_string()));
        assert_eq!(resource.kind, "daemonset");
    }

    fn metadata<const N: usize>(
        name: &str,
        namespace: &str,
        labels: [(&str, &str); N],
    ) -> ObjectMeta {
        ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            labels: (!labels.is_empty()).then(|| {
                labels
                    .into_iter()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect()
            }),
            ..Default::default()
        }
    }

    fn dummy_client() -> Client {
        let config = kube::Config::new("http://127.0.0.1:65535".parse().unwrap());
        Client::try_from(config).unwrap()
    }
}
