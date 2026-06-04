//! Integration tests for Prometheus adapter.

use agent::prometheus::{
    CollectorConfig, CustomQuery, MetricCollector, PrometheusAdapter, PrometheusClient,
    PrometheusConfig,
};
use db::{run_migrations, DatabasePool};
use domain::config::{
    AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, MetricsConfig, RetentionConfig,
    TelemetryConfig, WebConfig,
};
use domain::Config;
use std::time::Duration;

async fn create_test_pool() -> DatabasePool {
    let config = Config {
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
        logging: LoggingConfig::default(),
        telemetry: TelemetryConfig::default(),
        metrics: MetricsConfig::default(),
        retention: RetentionConfig::default(),
        kubernetes: domain::config::KubernetesConfig::default(),
    };

    let pool = DatabasePool::new(&config).await.unwrap();
    run_migrations(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn test_adapter_with_mock_prometheus() {
    let config = PrometheusConfig {
        url: "http://localhost:19090".to_string(),
        timeout: Duration::from_secs(5),
        auth_token: None,
    };

    let client = PrometheusClient::new(config).unwrap();
    let adapter = PrometheusAdapter::new(client, 1);

    assert_eq!(adapter.source_id(), 1);
}

#[tokio::test]
async fn test_collector_with_database() {
    let pool = create_test_pool().await;

    let prom_config = PrometheusConfig {
        url: "http://localhost:19090".to_string(),
        timeout: Duration::from_secs(5),
        auth_token: None,
    };

    let client = PrometheusClient::new(prom_config).unwrap();
    let adapter = PrometheusAdapter::new(client, 1);

    let collector_config = CollectorConfig {
        interval: Duration::from_secs(60),
        collect_infrastructure: false,
        collect_kubernetes: false,
        custom_queries: vec![],
    };

    let collector = MetricCollector::new(adapter, pool.clone(), collector_config);

    // Should not panic even if Prometheus is not available
    let result = collector.collect().await;
    assert!(result.is_ok());

    pool.close().await;
}

#[tokio::test]
async fn test_collector_with_custom_queries() {
    let pool = create_test_pool().await;

    let prom_config = PrometheusConfig {
        url: "http://localhost:19090".to_string(),
        timeout: Duration::from_secs(5),
        auth_token: None,
    };

    let client = PrometheusClient::new(prom_config).unwrap();
    let adapter = PrometheusAdapter::new(client, 1);

    let collector_config = CollectorConfig {
        interval: Duration::from_secs(60),
        collect_infrastructure: false,
        collect_kubernetes: false,
        custom_queries: vec![
            CustomQuery {
                name: "test_metric_1".to_string(),
                query: "up".to_string(),
            },
            CustomQuery {
                name: "test_metric_2".to_string(),
                query: "node_cpu_seconds_total".to_string(),
            },
        ],
    };

    let collector = MetricCollector::new(adapter, pool.clone(), collector_config);

    // Should handle errors gracefully when Prometheus is not available
    let result = collector.collect().await;
    assert!(result.is_ok());

    pool.close().await;
}

#[tokio::test]
async fn test_adapter_collect_metrics_no_prometheus() {
    let pool = create_test_pool().await;

    let prom_config = PrometheusConfig {
        url: "http://localhost:19090".to_string(),
        timeout: Duration::from_secs(5),
        auth_token: None,
    };

    let client = PrometheusClient::new(prom_config).unwrap();
    let adapter = PrometheusAdapter::new(client, 1);

    // Should return error or 0 when Prometheus is not available
    let result = adapter.collect_metrics(&pool, "up", "test_metric").await;

    assert!(result.is_err() || result.unwrap() == 0);

    pool.close().await;
}

#[tokio::test]
async fn test_adapter_collect_common_metrics_no_prometheus() {
    let pool = create_test_pool().await;

    let prom_config = PrometheusConfig {
        url: "http://localhost:19090".to_string(),
        timeout: Duration::from_secs(5),
        auth_token: None,
    };

    let client = PrometheusClient::new(prom_config).unwrap();
    let adapter = PrometheusAdapter::new(client, 1);

    // Should handle errors gracefully
    let result = adapter.collect_common_metrics(&pool).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);

    pool.close().await;
}

#[tokio::test]
async fn test_adapter_collect_k8s_metrics_no_prometheus() {
    let pool = create_test_pool().await;

    let prom_config = PrometheusConfig {
        url: "http://localhost:19090".to_string(),
        timeout: Duration::from_secs(5),
        auth_token: None,
    };

    let client = PrometheusClient::new(prom_config).unwrap();
    let adapter = PrometheusAdapter::new(client, 1);

    // Should handle errors gracefully
    let result = adapter.collect_k8s_metrics(&pool).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);

    pool.close().await;
}

#[tokio::test]
async fn test_multiple_adapters_same_pool() {
    let pool = create_test_pool().await;

    let prom_config = PrometheusConfig {
        url: "http://localhost:19090".to_string(),
        timeout: Duration::from_secs(5),
        auth_token: None,
    };

    let client1 = PrometheusClient::new(prom_config.clone()).unwrap();
    let adapter1 = PrometheusAdapter::new(client1, 1);

    let client2 = PrometheusClient::new(prom_config).unwrap();
    let adapter2 = PrometheusAdapter::new(client2, 2);

    // Both adapters should work with the same pool
    let result1 = adapter1.collect_common_metrics(&pool).await;
    let result2 = adapter2.collect_common_metrics(&pool).await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    pool.close().await;
}
