//! Metric query endpoints.

use crate::dto::metrics::{
    LatestMetric, LatestMetricsResponse, MetricDataPoint, MetricNamesResponse, MetricQueryInfo,
    MetricQueryParams, MetricQueryResponse,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use db::repositories::MetricRepository;
use std::collections::HashSet;
use tracing::debug;

/// List available metric names.
///
/// GET /api/v1/metrics
pub async fn list_metric_names(
    State(state): State<AppState>,
) -> ApiResult<Json<MetricNamesResponse>> {
    debug!("Listing metric names");

    let repo = MetricRepository::new(state.db());

    // Query all metrics and extract unique names
    // Note: This is a simple implementation. For production, consider adding
    // a separate table or index for metric names.
    let metrics = repo
        .query_by_source(0, Some(10000))
        .await
        .unwrap_or_default();

    let mut names: HashSet<String> = HashSet::new();
    for metric in metrics {
        names.insert(metric.name);
    }

    // Get all sources and query each
    let sources = db::repositories::MetricSourceRepository::new(state.db())
        .list()
        .await?;

    for source in sources {
        let metrics = repo.query_by_source(source.id, Some(10000)).await?;
        for metric in metrics {
            names.insert(metric.name);
        }
    }

    let mut names: Vec<String> = names.into_iter().collect();
    names.sort();
    let count = names.len();

    Ok(Json(MetricNamesResponse { names, count }))
}

/// Query metric data by name with time range and filters.
///
/// GET /api/v1/metrics/:name
pub async fn query_metrics(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<MetricQueryParams>,
) -> ApiResult<Json<MetricQueryResponse>> {
    debug!(name, ?params, "Querying metrics");

    // Parse start time
    let start = DateTime::parse_from_rfc3339(&params.start)
        .map_err(|e| ApiError::ValidationError(format!("Invalid start time: {}", e)))?
        .with_timezone(&Utc);

    // Parse end time (default to now)
    let end = if let Some(end_str) = &params.end {
        DateTime::parse_from_rfc3339(end_str)
            .map_err(|e| ApiError::ValidationError(format!("Invalid end time: {}", e)))?
            .with_timezone(&Utc)
    } else {
        Utc::now()
    };

    // Validate time range
    if start > end {
        return Err(ApiError::ValidationError(
            "Start time must be before end time".to_string(),
        ));
    }

    let limit = params.limit.unwrap_or(1000).min(10000);

    let repo = MetricRepository::new(state.db());
    let mut metrics = repo.query_by_name_and_range(&name, start, end).await?;

    // Apply source_id filter if provided
    if let Some(source_id) = params.source_id {
        metrics.retain(|m| m.source_id == source_id);
    }

    // Apply limit
    if metrics.len() > limit as usize {
        metrics.truncate(limit as usize);
    }

    let count = metrics.len();
    let data = metrics
        .into_iter()
        .map(MetricDataPoint::from_model)
        .collect();

    Ok(Json(MetricQueryResponse {
        name,
        data,
        count,
        query: MetricQueryInfo {
            start: start.to_rfc3339(),
            end: end.to_rfc3339(),
            source_id: params.source_id,
            limit,
        },
    }))
}

/// Get latest metric values for all metrics.
///
/// GET /api/v1/metrics/latest
pub async fn get_latest_metrics(
    State(state): State<AppState>,
) -> ApiResult<Json<LatestMetricsResponse>> {
    debug!("Getting latest metrics");

    let repo = MetricRepository::new(state.db());

    // Get all unique metric names first
    let sources = db::repositories::MetricSourceRepository::new(state.db())
        .list()
        .await?;

    let mut names: HashSet<String> = HashSet::new();
    for source in &sources {
        let metrics = repo.query_by_source(source.id, Some(1000)).await?;
        for metric in metrics {
            names.insert(metric.name);
        }
    }

    // Get latest value for each metric name
    let mut latest_metrics = Vec::new();
    for name in names {
        if let Some(metric) = repo.get_latest(&name).await? {
            latest_metrics.push(LatestMetric {
                name: metric.name,
                value: metric.value,
                timestamp: metric.timestamp.to_rfc3339(),
                source_id: metric.source_id,
            });
        }
    }

    let count = latest_metrics.len();

    Ok(Json(LatestMetricsResponse {
        metrics: latest_metrics,
        count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use chrono::Duration;
    use db::{run_migrations, DatabasePool};
    use domain::config::{
        AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, MetricsConfig,
        RetentionConfig, TelemetryConfig, WebConfig,
    };
    use domain::models::Metric;
    use domain::Config;
    use std::sync::Arc;

    fn test_config() -> Config {
        Config {
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
        }
    }

    async fn test_state() -> (AppState, i64) {
        let config = test_config();
        let pool = DatabasePool::new(&config).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Create a test source
        let source_repo = db::repositories::MetricSourceRepository::new(&pool);
        let source = source_repo
            .create("test_source", "prometheus", "{}")
            .await
            .unwrap();

        let state = AppState::new(pool, Arc::new(config));
        (state, source.id)
    }

    async fn insert_test_metrics(state: &AppState, source_id: i64) {
        let repo = MetricRepository::new(state.db());
        let now = Utc::now();

        let metrics = vec![
            Metric {
                id: 0,
                source_id,
                name: "cpu_usage".to_string(),
                value: 0.5,
                timestamp: now - Duration::minutes(10),
                created_at: now,
            },
            Metric {
                id: 0,
                source_id,
                name: "cpu_usage".to_string(),
                value: 0.6,
                timestamp: now - Duration::minutes(5),
                created_at: now,
            },
            Metric {
                id: 0,
                source_id,
                name: "memory_usage".to_string(),
                value: 0.7,
                timestamp: now - Duration::minutes(3),
                created_at: now,
            },
        ];

        repo.insert_batch(&metrics).await.unwrap();
    }

    #[tokio::test]
    async fn test_list_metric_names() {
        let (state, source_id) = test_state().await;
        insert_test_metrics(&state, source_id).await;

        let result = list_metric_names(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.count, 2);
        assert!(response.names.contains(&"cpu_usage".to_string()));
        assert!(response.names.contains(&"memory_usage".to_string()));
    }

    #[tokio::test]
    async fn test_query_metrics() {
        let (state, source_id) = test_state().await;
        insert_test_metrics(&state, source_id).await;

        let now = Utc::now();
        let params = MetricQueryParams {
            start: (now - Duration::minutes(15)).to_rfc3339(),
            end: Some(now.to_rfc3339()),
            source_id: None,
            step: None,
            limit: None,
        };

        let result =
            query_metrics(State(state), Path("cpu_usage".to_string()), Query(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.name, "cpu_usage");
        assert_eq!(response.count, 2);
        assert_eq!(response.data.len(), 2);
    }

    #[tokio::test]
    async fn test_query_metrics_with_source_filter() {
        let (state, source_id) = test_state().await;
        insert_test_metrics(&state, source_id).await;

        let now = Utc::now();
        let params = MetricQueryParams {
            start: (now - Duration::minutes(15)).to_rfc3339(),
            end: Some(now.to_rfc3339()),
            source_id: Some(source_id),
            step: None,
            limit: None,
        };

        let result =
            query_metrics(State(state), Path("cpu_usage".to_string()), Query(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.count, 2);
        assert_eq!(response.query.source_id, Some(source_id));
    }

    #[tokio::test]
    async fn test_query_metrics_invalid_start_time() {
        let (state, _) = test_state().await;

        let params = MetricQueryParams {
            start: "invalid".to_string(),
            end: None,
            source_id: None,
            step: None,
            limit: None,
        };

        let result =
            query_metrics(State(state), Path("cpu_usage".to_string()), Query(params)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_query_metrics_start_after_end() {
        let (state, _) = test_state().await;

        let now = Utc::now();
        let params = MetricQueryParams {
            start: now.to_rfc3339(),
            end: Some((now - Duration::hours(1)).to_rfc3339()),
            source_id: None,
            step: None,
            limit: None,
        };

        let result =
            query_metrics(State(state), Path("cpu_usage".to_string()), Query(params)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_get_latest_metrics() {
        let (state, source_id) = test_state().await;
        insert_test_metrics(&state, source_id).await;

        let result = get_latest_metrics(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.count, 2);

        // Find cpu_usage in response
        let cpu_metric = response
            .metrics
            .iter()
            .find(|m| m.name == "cpu_usage")
            .unwrap();
        assert!((cpu_metric.value - 0.6).abs() < f64::EPSILON); // Latest value
    }

    #[tokio::test]
    async fn test_query_metrics_with_limit() {
        let (state, source_id) = test_state().await;
        insert_test_metrics(&state, source_id).await;

        let now = Utc::now();
        let params = MetricQueryParams {
            start: (now - Duration::minutes(15)).to_rfc3339(),
            end: Some(now.to_rfc3339()),
            source_id: None,
            step: None,
            limit: Some(1),
        };

        let result =
            query_metrics(State(state), Path("cpu_usage".to_string()), Query(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.count, 1);
        assert_eq!(response.query.limit, 1);
    }
}
