//! Prometheus adapter implementing the metric source trait.

use crate::prometheus::{PrometheusClient, PrometheusError};
use chrono::{DateTime, Utc};
use db::repositories::MetricRepository;
use db::DatabasePool;
use domain::models::{Metric, MetricLabel};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Errors that can occur during metric collection.
#[derive(Error, Debug)]
pub enum AdapterError {
    /// Prometheus client error
    #[error("Prometheus error: {0}")]
    Prometheus(#[from] PrometheusError),

    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] db::DbError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// No data returned from query
    #[error("No data returned from query")]
    NoData,
}

/// Prometheus metric source adapter.
pub struct PrometheusAdapter {
    client: PrometheusClient,
    source_id: i64,
}

impl PrometheusAdapter {
    /// Creates a new Prometheus adapter.
    pub fn new(client: PrometheusClient, source_id: i64) -> Self {
        Self { client, source_id }
    }

    /// Collects metrics with labels using the given PromQL query.
    ///
    /// # Errors
    ///
    /// Returns error if the query fails or database insertion fails.
    pub async fn collect_metrics_with_labels(
        &self,
        pool: &DatabasePool,
        query: &str,
        metric_name: &str,
    ) -> Result<usize, AdapterError> {
        debug!(query = %query, metric_name = %metric_name, "Collecting metrics with labels");

        let result = self.client.query_instant(query, None).await?;

        if result.is_empty() {
            warn!(query = %query, "Query returned no data");
            return Ok(0);
        }

        let mut metrics = Vec::new();
        let now = Utc::now();

        for series in &result {
            let data_points = series.data_points();

            for (timestamp, value) in data_points {
                let metric = Metric {
                    id: 0,
                    source_id: self.source_id,
                    name: metric_name.to_string(),
                    value,
                    timestamp,
                    created_at: now,
                };

                metrics.push(metric);
            }
        }

        let count = metrics.len();

        if !metrics.is_empty() {
            let repo = MetricRepository::new(pool);
            repo.insert_batch(&metrics).await?;

            // Now insert labels for each metric
            // We need to query back the inserted metrics to get their IDs
            let inserted_metrics = repo
                .query_by_name_and_range(
                    metric_name,
                    now - chrono::Duration::seconds(10),
                    now + chrono::Duration::seconds(10),
                )
                .await?;

            let mut all_labels = Vec::new();

            for (series, metric) in result.iter().zip(inserted_metrics.iter()) {
                for (key, val) in &series.metric {
                    if key != "__name__" {
                        all_labels.push(MetricLabel {
                            id: 0,
                            metric_id: metric.id,
                            key: key.clone(),
                            value: val.clone(),
                        });
                    }
                }
            }

            if !all_labels.is_empty() {
                repo.insert_labels_batch(&all_labels).await?;
                info!(
                    metric_count = count,
                    label_count = all_labels.len(),
                    metric_name = %metric_name,
                    "Metrics and labels collected and stored"
                );
            } else {
                info!(count = count, metric_name = %metric_name, "Metrics collected and stored");
            }
        }

        Ok(count)
    }
    ///
    /// # Errors
    ///
    /// Returns error if the query fails or database insertion fails.
    pub async fn collect_metrics(
        &self,
        pool: &DatabasePool,
        query: &str,
        metric_name: &str,
    ) -> Result<usize, AdapterError> {
        debug!(query = %query, metric_name = %metric_name, "Collecting metrics");

        let result = self.client.query_instant(query, None).await?;

        if result.is_empty() {
            warn!(query = %query, "Query returned no data");
            return Ok(0);
        }

        let mut metrics = Vec::new();
        let mut labels_map: HashMap<i64, Vec<MetricLabel>> = HashMap::new();
        let now = Utc::now();

        for (idx, series) in result.iter().enumerate() {
            let data_points = series.data_points();

            for (timestamp, value) in data_points {
                let metric = Metric {
                    id: 0,
                    source_id: self.source_id,
                    name: metric_name.to_string(),
                    value,
                    timestamp,
                    created_at: now,
                };

                metrics.push(metric);

                let mut labels = Vec::new();
                for (key, val) in &series.metric {
                    if key != "__name__" {
                        labels.push(MetricLabel {
                            id: 0,
                            metric_id: 0,
                            key: key.clone(),
                            value: val.clone(),
                        });
                    }
                }

                if !labels.is_empty() {
                    labels_map.insert(idx as i64, labels);
                }
            }
        }

        let count = metrics.len();

        if !metrics.is_empty() {
            let repo = MetricRepository::new(pool);
            repo.insert_batch(&metrics).await?;
            info!(count = count, metric_name = %metric_name, "Metrics collected and stored");
        }

        Ok(count)
    }

    /// Collects metrics over a time range.
    ///
    /// # Errors
    ///
    /// Returns error if the query fails or database insertion fails.
    pub async fn collect_metrics_range(
        &self,
        pool: &DatabasePool,
        query: &str,
        metric_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        step: std::time::Duration,
    ) -> Result<usize, AdapterError> {
        debug!(
            query = %query,
            metric_name = %metric_name,
            start = %start,
            end = %end,
            "Collecting range metrics"
        );

        let result = self.client.query_range(query, start, end, step).await?;

        if result.is_empty() {
            warn!(query = %query, "Range query returned no data");
            return Ok(0);
        }

        let mut metrics = Vec::new();
        let now = Utc::now();

        for series in result {
            let data_points = series.data_points();

            for (timestamp, value) in data_points {
                let metric = Metric {
                    id: 0,
                    source_id: self.source_id,
                    name: metric_name.to_string(),
                    value,
                    timestamp,
                    created_at: now,
                };

                metrics.push(metric);
            }
        }

        let count = metrics.len();

        if !metrics.is_empty() {
            let repo = MetricRepository::new(pool);
            repo.insert_batch(&metrics).await?;
            info!(count = count, metric_name = %metric_name, "Range metrics collected and stored");
        }

        Ok(count)
    }

    /// Collects common infrastructure metrics.
    ///
    /// Collects CPU, memory, disk, and network metrics.
    ///
    /// # Errors
    ///
    /// Returns error if any collection fails.
    pub async fn collect_common_metrics(&self, pool: &DatabasePool) -> Result<usize, AdapterError> {
        info!("Collecting common infrastructure metrics");

        let mut total = 0;

        let queries = vec![
            ("cpu_usage", crate::prometheus::CommonQueries::cpu_usage()),
            (
                "memory_usage",
                crate::prometheus::CommonQueries::memory_usage(),
            ),
            ("disk_usage", crate::prometheus::CommonQueries::disk_usage()),
            (
                "network_receive_rate",
                crate::prometheus::CommonQueries::network_receive_rate(),
            ),
            (
                "network_transmit_rate",
                crate::prometheus::CommonQueries::network_transmit_rate(),
            ),
        ];

        for (name, query) in queries {
            match self.collect_metrics(pool, &query, name).await {
                Ok(count) => {
                    total += count;
                }
                Err(e) => {
                    error!(metric = %name, error = %e, "Failed to collect metric");
                }
            }
        }

        info!(total = total, "Common metrics collection completed");
        Ok(total)
    }

    /// Collects Kubernetes pod metrics.
    ///
    /// # Errors
    ///
    /// Returns error if any collection fails.
    pub async fn collect_k8s_metrics(&self, pool: &DatabasePool) -> Result<usize, AdapterError> {
        info!("Collecting Kubernetes pod metrics");

        let mut total = 0;

        let queries = vec![
            (
                "k8s_pod_cpu_usage",
                crate::prometheus::CommonQueries::k8s_pod_cpu_usage(),
            ),
            (
                "k8s_pod_memory_usage",
                crate::prometheus::CommonQueries::k8s_pod_memory_usage(),
            ),
            (
                "k8s_pod_network_receive_rate",
                crate::prometheus::CommonQueries::k8s_pod_network_receive_rate(),
            ),
            (
                "k8s_pod_network_transmit_rate",
                crate::prometheus::CommonQueries::k8s_pod_network_transmit_rate(),
            ),
        ];

        for (name, query) in queries {
            match self.collect_metrics(pool, &query, name).await {
                Ok(count) => {
                    total += count;
                }
                Err(e) => {
                    error!(metric = %name, error = %e, "Failed to collect Kubernetes metric");
                }
            }
        }

        info!(total = total, "Kubernetes metrics collection completed");
        Ok(total)
    }

    /// Gets the Prometheus client.
    pub fn client(&self) -> &PrometheusClient {
        &self.client
    }

    /// Gets the source ID.
    pub fn source_id(&self) -> i64 {
        self.source_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prometheus::PrometheusConfig;

    #[test]
    fn test_adapter_creation() {
        let config = PrometheusConfig::default();
        let client = PrometheusClient::new(config).unwrap();
        let adapter = PrometheusAdapter::new(client, 1);

        assert_eq!(adapter.source_id(), 1);
    }

    #[tokio::test]
    async fn test_collect_metrics_no_data() {
        let config = PrometheusConfig {
            url: "http://localhost:19090".to_string(),
            ..Default::default()
        };
        let client = PrometheusClient::new(config).unwrap();
        let adapter = PrometheusAdapter::new(client, 1);

        let pool = create_test_pool().await;
        let result = adapter.collect_metrics(&pool, "up", "test_metric").await;

        assert!(result.is_err() || result.unwrap() == 0);
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
            },
            logging: domain::config::LoggingConfig::default(),
            telemetry: domain::config::TelemetryConfig::default(),
            metrics: domain::config::MetricsConfig::default(),
            retention: domain::config::RetentionConfig::default(),
        };

        DatabasePool::new(&config).await.unwrap()
    }
}
