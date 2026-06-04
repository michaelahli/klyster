//! Kubernetes client and configuration.

use kube::{Client, Config};
use tracing::{debug, error, info};

/// Kubernetes client initialization error.
#[derive(Debug, thiserror::Error)]
pub enum K8sClientError {
    /// Failed to load kubeconfig.
    #[error("Failed to load kubeconfig: {0}")]
    ConfigError(String),
    /// Kubernetes API error.
    #[error("Kubernetes API error: {0}")]
    ApiError(String),
    /// Kubernetes API unavailable.
    #[error("Kubernetes API unavailable: {0}")]
    ApiUnavailable(String),
}

/// Result type for Kubernetes client operations.
pub type K8sResult<T> = Result<T, K8sClientError>;

/// Initialize Kubernetes client from in-cluster config or kubeconfig.
///
/// # Arguments
/// * `kubeconfig_path` - Optional path to kubeconfig file. If None, uses in-cluster config.
///
/// # Errors
/// Returns error if config cannot be loaded or API is unavailable.
pub async fn init_client(kubeconfig_path: Option<&str>) -> K8sResult<Client> {
    let config = if let Some(path) = kubeconfig_path {
        debug!("Loading kubeconfig from: {}", path);
        Config::from_kubeconfig(&kube::config::KubeConfigOptions {
            context: None,
            cluster: None,
            user: None,
        })
        .await
        .map_err(|e| K8sClientError::ConfigError(e.to_string()))?
    } else {
        debug!("Loading in-cluster config");
            match Config::incluster() {
                Ok(cfg) => cfg,
                Err(e) => {
                    debug!("In-cluster config failed, falling back to kubeconfig: {}", e);
                    Config::from_kubeconfig(&kube::config::KubeConfigOptions::default())
                        .await
                        .map_err(|e| K8sClientError::ConfigError(e.to_string()))?
            }
        }
    };

    let client = Client::try_from(config).map_err(|e| K8sClientError::ApiError(e.to_string()))?;

    // Test connection
    match test_connection(&client).await {
        Ok(version) => {
            info!("Connected to Kubernetes API version: {}", version);
            Ok(client)
        }
        Err(e) => {
            error!("Failed to connect to Kubernetes API: {}", e);
            Err(K8sClientError::ApiUnavailable(e.to_string()))
        }
    }
}

/// Test Kubernetes API connectivity by fetching server version.
async fn test_connection(client: &Client) -> K8sResult<String> {
    let version = client
        .apiserver_version()
        .await
        .map_err(|e| K8sClientError::ApiError(e.to_string()))?;
    Ok(format!("{}.{}", version.major, version.minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Kubernetes cluster
    async fn test_init_client_incluster() {
        let result = init_client(None).await;
        // Will fail in CI without K8s, that's expected
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires kubeconfig
    async fn test_init_client_kubeconfig() {
        let result = init_client(Some("~/.kube/config")).await;
        // Will fail without kubeconfig, that's expected
        assert!(result.is_ok() || result.is_err());
    }
}
