//! Scheduled metric collection from Prometheus.

use crate::prometheus::{AdapterError, PrometheusAdapter};
use db::DatabasePool;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info};

/// Configuration for the metric collector.
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    /// Collection interval
    pub interval: Duration,
    /// Whether to collect common infrastructure metrics
    pub collect_infrastructure: bool,
    /// Whether to collect Kubernetes metrics
    pub collect_kubernetes: bool,
    /// Custom queries to execute
    pub custom_queries: Vec<CustomQuery>,
}

/// Custom query configuration.
#[derive(Debug, Clone)]
pub struct CustomQuery {
    /// Metric name to store
    pub name: String,
    /// `PromQL` query
    pub query: String,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
            collect_infrastructure: true,
            collect_kubernetes: false,
            custom_queries: Vec::new(),
        }
    }
}

/// Scheduled metric collector.
pub struct MetricCollector {
    adapter: Arc<PrometheusAdapter>,
    pool: DatabasePool,
    config: CollectorConfig,
}

impl MetricCollector {
    /// Creates a new metric collector.
    #[must_use]
    pub fn new(adapter: PrometheusAdapter, pool: DatabasePool, config: CollectorConfig) -> Self {
        Self {
            adapter: Arc::new(adapter),
            pool,
            config,
        }
    }

    /// Starts the collection loop.
    ///
    /// Runs until the provided shutdown signal is received.
    pub async fn run(self, mut shutdown: tokio::sync::broadcast::Receiver<()>) {
        info!(
            interval_secs = self.config.interval.as_secs(),
            "Starting metric collection loop"
        );

        let mut ticker = interval(self.config.interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.collect_once().await {
                        error!(error = %e, "Metric collection failed");
                    }
                }
                _ = shutdown.recv() => {
                    info!("Metric collector received shutdown signal");
                    break;
                }
            }
        }

        info!("Metric collector stopped");
    }

    /// Performs a single collection cycle.
    async fn collect_once(&self) -> Result<(), AdapterError> {
        debug!("Starting collection cycle");

        let mut total_collected = 0;

        if self.config.collect_infrastructure {
            match self.adapter.collect_common_metrics(&self.pool).await {
                Ok(count) => {
                    total_collected += count;
                    debug!(count = count, "Infrastructure metrics collected");
                }
                Err(e) => {
                    error!(error = %e, "Failed to collect infrastructure metrics");
                }
            }
        }

        if self.config.collect_kubernetes {
            match self.adapter.collect_k8s_metrics(&self.pool).await {
                Ok(count) => {
                    total_collected += count;
                    debug!(count = count, "Kubernetes metrics collected");
                }
                Err(e) => {
                    error!(error = %e, "Failed to collect Kubernetes metrics");
                }
            }
        }

        for custom in &self.config.custom_queries {
            match self
                .adapter
                .collect_metrics(&self.pool, &custom.query, &custom.name)
                .await
            {
                Ok(count) => {
                    total_collected += count;
                    debug!(name = %custom.name, count = count, "Custom metric collected");
                }
                Err(e) => {
                    error!(name = %custom.name, error = %e, "Failed to collect custom metric");
                }
            }
        }

        info!(total = total_collected, "Collection cycle completed");
        Ok(())
    }

    /// Performs a single collection cycle synchronously.
    ///
    /// Useful for testing or manual triggering.
    pub async fn collect(&self) -> Result<(), AdapterError> {
        self.collect_once().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prometheus::{PrometheusClient, PrometheusConfig};

    #[test]
    fn test_collector_config_default() {
        let config = CollectorConfig::default();
        assert_eq!(config.interval, Duration::from_secs(60));
        assert!(config.collect_infrastructure);
        assert!(!config.collect_kubernetes);
        assert!(config.custom_queries.is_empty());
    }

    #[test]
    fn test_custom_query_creation() {
        let query = CustomQuery {
            name: "test_metric".to_string(),
            query: "up".to_string(),
        };

        assert_eq!(query.name, "test_metric");
        assert_eq!(query.query, "up");
    }

    #[tokio::test]
    async fn test_collector_creation() {
        let prom_config = PrometheusConfig::default();
        let client = PrometheusClient::new(prom_config).unwrap();
        let adapter = PrometheusAdapter::new(client, 1);
        let pool = create_test_pool().await;
        let config = CollectorConfig::default();

        let collector = MetricCollector::new(adapter, pool, config);
        assert_eq!(collector.config.interval, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_collector_single_collection() {
        let prom_config = PrometheusConfig {
            url: "http://localhost:19090".to_string(),
            ..Default::default()
        };
        let client = PrometheusClient::new(prom_config).unwrap();
        let adapter = PrometheusAdapter::new(client, 1);
        let pool = create_test_pool().await;

        let config = CollectorConfig {
            interval: Duration::from_secs(60),
            collect_infrastructure: false,
            collect_kubernetes: false,
            custom_queries: vec![],
        };

        let collector = MetricCollector::new(adapter, pool, config);
        let result = collector.collect().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_collector_with_custom_queries() {
        let prom_config = PrometheusConfig {
            url: "http://localhost:19090".to_string(),
            ..Default::default()
        };
        let client = PrometheusClient::new(prom_config).unwrap();
        let adapter = PrometheusAdapter::new(client, 1);
        let pool = create_test_pool().await;

        let config = CollectorConfig {
            interval: Duration::from_secs(60),
            collect_infrastructure: false,
            collect_kubernetes: false,
            custom_queries: vec![CustomQuery {
                name: "test_up".to_string(),
                query: "up".to_string(),
            }],
        };

        let collector = MetricCollector::new(adapter, pool, config);
        let result = collector.collect().await;

        assert!(result.is_ok());
    }

    async fn create_test_pool() -> DatabasePool {
        let config = domain::Config {
            database: domain::config::DatabaseConfig {
                db_type: "sqlite".to_string(),
                sqlite_path: ":memory:".to_string(),
                postgres_url: None,
                pool_size: 5,
                wal_mode: false,
            },
            web: domain::config::WebConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: 1,
            },
            agent: domain::config::AgentConfig {
                enabled: false,
                collection_interval_secs: 60,
                prometheus: domain::config::PrometheusAgentConfig::default(),
            },
            analytics: domain::config::AnalyticsConfig {
                enabled: false,
                grpc_endpoint: "http://localhost:50051".to_string(),
                python_path: None,
            },
            logging: domain::config::LoggingConfig::default(),
            telemetry: domain::config::TelemetryConfig::default(),
            metrics: domain::config::MetricsConfig::default(),
            retention: domain::config::RetentionConfig::default(),
        };

        DatabasePool::new(&config).await.unwrap()
    }
}
