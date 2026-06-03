//! Prometheus connection health monitoring.

use crate::prometheus::PrometheusClient;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Health status of Prometheus connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Connection is healthy
    Healthy,
    /// Connection is degraded (some errors)
    Degraded,
    /// Connection is down
    Down,
    /// Health status unknown (not checked yet)
    Unknown,
}

impl HealthStatus {
    /// Checks if the status is healthy.
    #[must_use] 
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    /// Checks if the status is degraded or down.
    #[must_use] 
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, HealthStatus::Degraded | HealthStatus::Down)
    }
}

/// Health check result.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Current health status
    pub status: HealthStatus,
    /// Last successful check timestamp
    pub last_success: Option<DateTime<Utc>>,
    /// Last failed check timestamp
    pub last_failure: Option<DateTime<Utc>>,
    /// Last error message
    pub last_error: Option<String>,
    /// Consecutive failure count
    pub consecutive_failures: u32,
}

impl Default for HealthCheckResult {
    fn default() -> Self {
        Self {
            status: HealthStatus::Unknown,
            last_success: None,
            last_failure: None,
            last_error: None,
            consecutive_failures: 0,
        }
    }
}

/// Prometheus connection health monitor.
pub struct HealthMonitor {
    client: Arc<PrometheusClient>,
    check_interval: Duration,
    failure_threshold: u32,
    health_result: Arc<RwLock<HealthCheckResult>>,
}

impl HealthMonitor {
    /// Creates a new health monitor.
    ///
    /// # Arguments
    ///
    /// * `client` - Prometheus client to monitor
    /// * `check_interval` - Interval between health checks
    /// * `failure_threshold` - Number of consecutive failures before marking as down
    #[must_use] 
    pub fn new(client: PrometheusClient, check_interval: Duration, failure_threshold: u32) -> Self {
        Self {
            client: Arc::new(client),
            check_interval,
            failure_threshold,
            health_result: Arc::new(RwLock::new(HealthCheckResult::default())),
        }
    }

    /// Starts the health monitoring loop.
    ///
    /// Runs until the provided shutdown signal is received.
    pub async fn run(self, mut shutdown: tokio::sync::broadcast::Receiver<()>) {
        info!(
            interval_secs = self.check_interval.as_secs(),
            failure_threshold = self.failure_threshold,
            "Starting Prometheus health monitoring"
        );

        let mut ticker = interval(self.check_interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.check_health().await;
                }
                _ = shutdown.recv() => {
                    info!("Health monitor received shutdown signal");
                    break;
                }
            }
        }

        info!("Health monitor stopped");
    }

    /// Performs a single health check.
    async fn check_health(&self) {
        debug!("Performing health check");

        match self.client.health_check().await {
            Ok(()) => {
                let mut result = self.health_result.write().await;
                result.status = HealthStatus::Healthy;
                result.last_success = Some(Utc::now());
                result.consecutive_failures = 0;
                result.last_error = None;
                debug!("Health check passed");
            }
            Err(e) => {
                let mut result = self.health_result.write().await;
                result.consecutive_failures += 1;
                result.last_failure = Some(Utc::now());
                result.last_error = Some(e.to_string());

                if result.consecutive_failures >= self.failure_threshold {
                    if result.status != HealthStatus::Down {
                        error!(
                            consecutive_failures = result.consecutive_failures,
                            error = %e,
                            "Prometheus connection is down"
                        );
                        result.status = HealthStatus::Down;
                    }
                } else if result.status != HealthStatus::Degraded {
                    warn!(
                        consecutive_failures = result.consecutive_failures,
                        error = %e,
                        "Prometheus connection is degraded"
                    );
                    result.status = HealthStatus::Degraded;
                }
            }
        }
    }

    /// Gets the current health status.
    pub async fn get_health(&self) -> HealthCheckResult {
        self.health_result.read().await.clone()
    }

    /// Checks if the connection is currently healthy.
    pub async fn is_healthy(&self) -> bool {
        self.health_result.read().await.status.is_healthy()
    }

    /// Performs an immediate health check and returns the result.
    pub async fn check_now(&self) -> HealthCheckResult {
        self.check_health().await;
        self.get_health().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prometheus::PrometheusConfig;

    #[test]
    fn test_health_status_is_healthy() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(!HealthStatus::Degraded.is_healthy());
        assert!(!HealthStatus::Down.is_healthy());
        assert!(!HealthStatus::Unknown.is_healthy());
    }

    #[test]
    fn test_health_status_is_unhealthy() {
        assert!(!HealthStatus::Healthy.is_unhealthy());
        assert!(HealthStatus::Degraded.is_unhealthy());
        assert!(HealthStatus::Down.is_unhealthy());
        assert!(!HealthStatus::Unknown.is_unhealthy());
    }

    #[test]
    fn test_health_check_result_default() {
        let result = HealthCheckResult::default();
        assert_eq!(result.status, HealthStatus::Unknown);
        assert!(result.last_success.is_none());
        assert!(result.last_failure.is_none());
        assert!(result.last_error.is_none());
        assert_eq!(result.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let config = PrometheusConfig::default();
        let client = PrometheusClient::new(config).unwrap();
        let monitor = HealthMonitor::new(client, Duration::from_secs(30), 3);

        let health = monitor.get_health().await;
        assert_eq!(health.status, HealthStatus::Unknown);
    }

    #[tokio::test]
    async fn test_health_monitor_check_now_unhealthy() {
        let config = PrometheusConfig {
            url: "http://localhost:19090".to_string(),
            ..Default::default()
        };
        let client = PrometheusClient::new(config).unwrap();
        let monitor = HealthMonitor::new(client, Duration::from_secs(30), 3);

        let result = monitor.check_now().await;
        assert!(result.status.is_unhealthy() || result.status == HealthStatus::Unknown);
    }

    #[tokio::test]
    async fn test_health_monitor_is_healthy() {
        let config = PrometheusConfig {
            url: "http://localhost:19090".to_string(),
            ..Default::default()
        };
        let client = PrometheusClient::new(config).unwrap();
        let monitor = HealthMonitor::new(client, Duration::from_secs(30), 3);

        let is_healthy = monitor.is_healthy().await;
        assert!(!is_healthy);
    }

    #[tokio::test]
    async fn test_health_monitor_consecutive_failures() {
        let config = PrometheusConfig {
            url: "http://localhost:19090".to_string(),
            ..Default::default()
        };
        let client = PrometheusClient::new(config).unwrap();
        let monitor = HealthMonitor::new(client, Duration::from_secs(30), 2);

        // First failure - should be degraded
        monitor.check_health().await;
        let result = monitor.get_health().await;
        assert_eq!(result.consecutive_failures, 1);

        // Second failure - should be down
        monitor.check_health().await;
        let result = monitor.get_health().await;
        assert_eq!(result.consecutive_failures, 2);
        assert_eq!(result.status, HealthStatus::Down);
    }
}
