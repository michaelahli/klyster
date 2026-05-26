//! Prometheus service discovery integration.

use crate::prometheus::{PrometheusClient, PrometheusError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Service discovery configuration.
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Whether to enable service discovery
    pub enabled: bool,
    /// Refresh interval in seconds
    pub refresh_interval_secs: u64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            refresh_interval_secs: 300,
        }
    }
}

/// A discovered target from Prometheus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    /// Target address (e.g., "10.0.0.1:9100")
    pub address: String,
    /// Target labels
    pub labels: HashMap<String, String>,
    /// Health status
    pub health: TargetHealth,
}

/// Target health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetHealth {
    /// Target is healthy
    Up,
    /// Target is down
    Down,
    /// Health status unknown
    Unknown,
}

impl Target {
    /// Gets the job name from labels.
    pub fn job(&self) -> Option<&str> {
        self.labels.get("job").map(|s| s.as_str())
    }

    /// Gets the instance name from labels.
    pub fn instance(&self) -> Option<&str> {
        self.labels.get("instance").map(|s| s.as_str())
    }

    /// Checks if the target is healthy.
    pub fn is_healthy(&self) -> bool {
        self.health == TargetHealth::Up
    }
}

/// Service discovery client.
pub struct ServiceDiscovery {
    client: PrometheusClient,
    config: DiscoveryConfig,
}

impl ServiceDiscovery {
    /// Creates a new service discovery client.
    pub fn new(client: PrometheusClient, config: DiscoveryConfig) -> Self {
        Self { client, config }
    }

    /// Discovers active targets from Prometheus.
    ///
    /// # Errors
    ///
    /// Returns error if the API request fails.
    pub async fn discover_targets(&self) -> Result<Vec<Target>, PrometheusError> {
        debug!("Discovering targets from Prometheus");

        let url = self
            .client
            .config()
            .url
            .parse::<url::Url>()
            .map_err(|e| PrometheusError::InvalidUrl(e.to_string()))?
            .join("/api/v1/targets")
            .map_err(|e| PrometheusError::InvalidUrl(e.to_string()))?;

        let response = reqwest::get(url).await?;

        if !response.status().is_success() {
            let status = response.status().to_string();
            let error = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to get targets".to_string());
            return Err(PrometheusError::ApiError { status, error });
        }

        let api_response: TargetsResponse = response.json().await.map_err(|e| {
            PrometheusError::ParseError(format!("Failed to parse targets response: {e}"))
        })?;

        if api_response.status != "success" {
            return Err(PrometheusError::ApiError {
                status: api_response.status,
                error: api_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            });
        }

        let targets: Vec<Target> = api_response
            .data
            .ok_or(PrometheusError::NoData)?
            .active_targets
            .into_iter()
            .map(|t| Target {
                address: format!(
                    "{}:{}",
                    t.discovered_labels
                        .get("__address__")
                        .unwrap_or(&"unknown".to_string()),
                    ""
                ),
                labels: t.labels,
                health: match t.health.as_str() {
                    "up" => TargetHealth::Up,
                    "down" => TargetHealth::Down,
                    _ => TargetHealth::Unknown,
                },
            })
            .collect();

        info!(count = targets.len(), "Discovered targets");
        Ok(targets)
    }

    /// Discovers healthy targets only.
    ///
    /// # Errors
    ///
    /// Returns error if the API request fails.
    pub async fn discover_healthy_targets(&self) -> Result<Vec<Target>, PrometheusError> {
        let targets = self.discover_targets().await?;
        let healthy: Vec<Target> = targets.into_iter().filter(|t| t.is_healthy()).collect();

        info!(count = healthy.len(), "Discovered healthy targets");
        Ok(healthy)
    }

    /// Discovers targets for a specific job.
    ///
    /// # Errors
    ///
    /// Returns error if the API request fails.
    pub async fn discover_targets_by_job(
        &self,
        job_name: &str,
    ) -> Result<Vec<Target>, PrometheusError> {
        let targets = self.discover_targets().await?;
        let filtered: Vec<Target> = targets
            .into_iter()
            .filter(|t| t.job() == Some(job_name))
            .collect();

        info!(job = %job_name, count = filtered.len(), "Discovered targets for job");
        Ok(filtered)
    }

    /// Gets the discovery configuration.
    pub fn config(&self) -> &DiscoveryConfig {
        &self.config
    }
}

/// Prometheus targets API response.
#[derive(Debug, Deserialize)]
struct TargetsResponse {
    status: String,
    data: Option<TargetsData>,
    error: Option<String>,
}

/// Targets data section.
#[derive(Debug, Deserialize)]
struct TargetsData {
    #[serde(rename = "activeTargets")]
    active_targets: Vec<ActiveTarget>,
}

/// Active target information.
#[derive(Debug, Deserialize)]
struct ActiveTarget {
    #[serde(rename = "discoveredLabels")]
    discovered_labels: HashMap<String, String>,
    labels: HashMap<String, String>,
    health: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prometheus::PrometheusConfig;

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.refresh_interval_secs, 300);
    }

    #[test]
    fn test_target_health() {
        assert_eq!(TargetHealth::Up, TargetHealth::Up);
        assert_ne!(TargetHealth::Up, TargetHealth::Down);
    }

    #[test]
    fn test_target_is_healthy() {
        let target = Target {
            address: "localhost:9090".to_string(),
            labels: HashMap::new(),
            health: TargetHealth::Up,
        };

        assert!(target.is_healthy());

        let target_down = Target {
            address: "localhost:9090".to_string(),
            labels: HashMap::new(),
            health: TargetHealth::Down,
        };

        assert!(!target_down.is_healthy());
    }

    #[test]
    fn test_target_job() {
        let mut labels = HashMap::new();
        labels.insert("job".to_string(), "prometheus".to_string());

        let target = Target {
            address: "localhost:9090".to_string(),
            labels,
            health: TargetHealth::Up,
        };

        assert_eq!(target.job(), Some("prometheus"));
    }

    #[test]
    fn test_target_instance() {
        let mut labels = HashMap::new();
        labels.insert("instance".to_string(), "localhost:9090".to_string());

        let target = Target {
            address: "localhost:9090".to_string(),
            labels,
            health: TargetHealth::Up,
        };

        assert_eq!(target.instance(), Some("localhost:9090"));
    }

    #[tokio::test]
    async fn test_service_discovery_creation() {
        let prom_config = PrometheusConfig::default();
        let client = PrometheusClient::new(prom_config).unwrap();
        let discovery_config = DiscoveryConfig::default();

        let discovery = ServiceDiscovery::new(client, discovery_config);
        assert!(!discovery.config().enabled);
    }

    #[tokio::test]
    async fn test_discover_targets_no_prometheus() {
        let prom_config = PrometheusConfig {
            url: "http://localhost:19090".to_string(),
            ..Default::default()
        };
        let client = PrometheusClient::new(prom_config).unwrap();
        let discovery_config = DiscoveryConfig::default();

        let discovery = ServiceDiscovery::new(client, discovery_config);
        let result = discovery.discover_targets().await;

        assert!(result.is_err());
    }
}
