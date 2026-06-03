//! DTOs for metric query endpoints.

use serde::{Deserialize, Serialize};

/// Query parameters for metric data.
#[derive(Debug, Clone, Deserialize)]
pub struct MetricQueryParams {
    /// Start time (ISO8601 timestamp).
    pub start: String,
    /// End time (ISO8601 timestamp, default: now).
    pub end: Option<String>,
    /// Source ID filter (optional).
    pub source_id: Option<i64>,
    /// Aggregation interval: 1m, 5m, 1h, 1d (optional).
    pub step: Option<String>,
    /// Maximum number of data points (default: 1000).
    pub limit: Option<i64>,
}

/// Response for a single metric data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDataPoint {
    /// Metric ID.
    pub id: i64,
    /// Source ID.
    pub source_id: i64,
    /// Metric name.
    pub name: String,
    /// Metric value.
    pub value: f64,
    /// Timestamp (ISO8601).
    pub timestamp: String,
}

impl MetricDataPoint {
    /// Convert from domain model.
    #[must_use] 
    pub fn from_model(metric: domain::models::Metric) -> Self {
        Self {
            id: metric.id,
            source_id: metric.source_id,
            name: metric.name,
            value: metric.value,
            timestamp: metric.timestamp.to_rfc3339(),
        }
    }
}

/// Response for metric query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricQueryResponse {
    /// Metric name.
    pub name: String,
    /// Data points.
    pub data: Vec<MetricDataPoint>,
    /// Total count of data points.
    pub count: usize,
    /// Query parameters used.
    pub query: MetricQueryInfo,
}

/// Query information included in response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricQueryInfo {
    /// Start time.
    pub start: String,
    /// End time.
    pub end: String,
    /// Source ID filter (if applied).
    pub source_id: Option<i64>,
    /// Limit applied.
    pub limit: i64,
}

/// Response for listing available metric names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricNamesResponse {
    /// List of unique metric names.
    pub names: Vec<String>,
    /// Total count.
    pub count: usize,
}

/// Response for latest metric values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestMetricsResponse {
    /// Latest metric values per name.
    pub metrics: Vec<LatestMetric>,
    /// Total count.
    pub count: usize,
}

/// Latest metric value for a specific name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestMetric {
    /// Metric name.
    pub name: String,
    /// Latest value.
    pub value: f64,
    /// Timestamp of latest value (ISO8601).
    pub timestamp: String,
    /// Source ID.
    pub source_id: i64,
}
