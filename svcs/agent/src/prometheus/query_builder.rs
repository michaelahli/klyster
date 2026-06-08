//! `PromQL` query builder for common metric queries.

use std::collections::HashMap;

/// Builder for constructing `PromQL` queries.
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    metric: String,
    labels: HashMap<String, String>,
    aggregation: Option<Aggregation>,
    range: Option<String>,
    rate_interval: Option<String>,
}

/// Aggregation functions supported by Prometheus.
#[derive(Debug, Clone, Copy)]
pub enum Aggregation {
    /// Sum of values
    Sum,
    /// Average of values
    Avg,
    /// Minimum value
    Min,
    /// Maximum value
    Max,
    /// Count of values
    Count,
}

impl Aggregation {
    fn as_str(self) -> &'static str {
        match self {
            Aggregation::Sum => "sum",
            Aggregation::Avg => "avg",
            Aggregation::Min => "min",
            Aggregation::Max => "max",
            Aggregation::Count => "count",
        }
    }
}

impl QueryBuilder {
    /// Creates a new query builder for the given metric.
    pub fn new(metric: impl Into<String>) -> Self {
        Self {
            metric: metric.into(),
            labels: HashMap::new(),
            aggregation: None,
            range: None,
            rate_interval: None,
        }
    }

    /// Adds a label selector.
    #[must_use]
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Adds multiple label selectors.
    #[must_use]
    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels.extend(labels);
        self
    }

    /// Applies an aggregation function.
    #[must_use]
    pub fn aggregate(mut self, agg: Aggregation) -> Self {
        self.aggregation = Some(agg);
        self
    }

    /// Applies `rate()` function with the given interval.
    #[must_use]
    pub fn rate(mut self, interval: impl Into<String>) -> Self {
        self.rate_interval = Some(interval.into());
        self
    }

    /// Applies range selector (e.g., "[5m]").
    #[must_use]
    pub fn range(mut self, range: impl Into<String>) -> Self {
        self.range = Some(range.into());
        self
    }

    /// Builds the `PromQL` query string.
    #[must_use]
    pub fn build(self) -> String {
        let mut query = self.metric.clone();

        if !self.labels.is_empty() {
            let label_selectors: Vec<String> = self
                .labels
                .iter()
                .map(|(k, v)| format!("{k}=\"{v}\""))
                .collect();
            query.push('{');
            query.push_str(&label_selectors.join(","));
            query.push('}');
        }

        if let Some(range) = self.range {
            query.push('[');
            query.push_str(&range);
            query.push(']');
        }

        if let Some(interval) = self.rate_interval {
            query = format!("rate({query}[{interval}])");
        }

        if let Some(agg) = self.aggregation {
            query = format!("{}({query})", agg.as_str());
        }

        query
    }
}

/// Common `PromQL` queries for infrastructure metrics.
pub struct CommonQueries;

impl CommonQueries {
    /// CPU usage percentage (0-100) per instance.
    #[must_use]
    pub fn cpu_usage() -> String {
        "100 * (1 - avg by(instance) (rate(node_cpu_seconds_total{mode=\"idle\"}[5m])))".to_string()
    }

    /// CPU usage percentage aggregated across all instances.
    #[must_use]
    pub fn cpu_usage_total() -> String {
        "100 * (1 - avg(rate(node_cpu_seconds_total{mode=\"idle\"}[5m])))".to_string()
    }

    /// Memory usage percentage per instance.
    #[must_use]
    pub fn memory_usage() -> String {
        "(1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)) * 100".to_string()
    }

    /// Memory usage in bytes per instance.
    #[must_use]
    pub fn memory_usage_bytes() -> String {
        QueryBuilder::new("node_memory_MemTotal_bytes")
            .build()
            .replace(
                "node_memory_MemTotal_bytes",
                "(node_memory_MemTotal_bytes - node_memory_MemAvailable_bytes)",
            )
    }

    /// Disk usage percentage per instance and device.
    #[must_use]
    pub fn disk_usage() -> String {
        "(1 - (node_filesystem_avail_bytes / node_filesystem_size_bytes)) * 100".to_string()
    }

    /// Disk I/O read rate in bytes per second.
    #[must_use]
    pub fn disk_read_rate() -> String {
        QueryBuilder::new("node_disk_read_bytes_total")
            .rate("5m")
            .build()
    }

    /// Disk I/O write rate in bytes per second.
    #[must_use]
    pub fn disk_write_rate() -> String {
        QueryBuilder::new("node_disk_written_bytes_total")
            .rate("5m")
            .build()
    }

    /// Network receive rate in bytes per second.
    #[must_use]
    pub fn network_receive_rate() -> String {
        QueryBuilder::new("node_network_receive_bytes_total")
            .rate("5m")
            .build()
    }

    /// Network transmit rate in bytes per second.
    #[must_use]
    pub fn network_transmit_rate() -> String {
        QueryBuilder::new("node_network_transmit_bytes_total")
            .rate("5m")
            .build()
    }

    /// Kubernetes pod CPU usage.
    #[must_use]
    pub fn k8s_pod_cpu_usage() -> String {
        QueryBuilder::new("container_cpu_usage_seconds_total")
            .rate("5m")
            .build()
    }

    /// Kubernetes pod memory usage in bytes.
    #[must_use]
    pub fn k8s_pod_memory_usage() -> String {
        QueryBuilder::new("container_memory_working_set_bytes").build()
    }

    /// Kubernetes pod network receive rate.
    #[must_use]
    pub fn k8s_pod_network_receive_rate() -> String {
        QueryBuilder::new("container_network_receive_bytes_total")
            .rate("5m")
            .build()
    }

    /// Kubernetes pod network transmit rate.
    #[must_use]
    pub fn k8s_pod_network_transmit_rate() -> String {
        QueryBuilder::new("container_network_transmit_bytes_total")
            .rate("5m")
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_metric() {
        let query = QueryBuilder::new("up").build();
        assert_eq!(query, "up");
    }

    #[test]
    fn test_metric_with_single_label() {
        let query = QueryBuilder::new("up")
            .with_label("job", "prometheus")
            .build();
        assert_eq!(query, "up{job=\"prometheus\"}");
    }

    #[test]
    fn test_metric_with_multiple_labels() {
        let query = QueryBuilder::new("up")
            .with_label("job", "prometheus")
            .with_label("instance", "localhost:9090")
            .build();

        assert!(query.starts_with("up{"));
        assert!(query.contains("job=\"prometheus\""));
        assert!(query.contains("instance=\"localhost:9090\""));
        assert!(query.ends_with('}'));
    }

    #[test]
    fn test_metric_with_range() {
        let query = QueryBuilder::new("up").range("5m").build();
        assert_eq!(query, "up[5m]");
    }

    #[test]
    fn test_metric_with_rate() {
        let query = QueryBuilder::new("http_requests_total").rate("5m").build();
        assert_eq!(query, "rate(http_requests_total[5m])");
    }

    #[test]
    fn test_metric_with_aggregation() {
        let query = QueryBuilder::new("up").aggregate(Aggregation::Sum).build();
        assert_eq!(query, "sum(up)");
    }

    #[test]
    fn test_complex_query() {
        let query = QueryBuilder::new("http_requests_total")
            .with_label("job", "api")
            .with_label("status", "200")
            .rate("5m")
            .aggregate(Aggregation::Sum)
            .build();

        assert!(query.starts_with("sum(rate(http_requests_total{"));
        assert!(query.contains("job=\"api\""));
        assert!(query.contains("status=\"200\""));
        assert!(query.ends_with("}[5m]))"));
    }

    #[test]
    fn test_cpu_usage_query() {
        let query = CommonQueries::cpu_usage();
        assert!(query.contains("node_cpu_seconds_total"));
        assert!(query.contains("mode=\"idle\""));
        assert!(query.contains("rate"));
    }

    #[test]
    fn test_memory_usage_query() {
        let query = CommonQueries::memory_usage();
        assert!(query.contains("node_memory_MemAvailable_bytes"));
        assert!(query.contains("node_memory_MemTotal_bytes"));
    }

    #[test]
    fn test_disk_usage_query() {
        let query = CommonQueries::disk_usage();
        assert!(query.contains("node_filesystem_avail_bytes"));
        assert!(query.contains("node_filesystem_size_bytes"));
    }

    #[test]
    fn test_network_receive_rate_query() {
        let query = CommonQueries::network_receive_rate();
        assert!(query.contains("node_network_receive_bytes_total"));
        assert!(query.contains("rate"));
    }

    #[test]
    fn test_k8s_pod_cpu_usage_query() {
        let query = CommonQueries::k8s_pod_cpu_usage();
        assert!(query.contains("container_cpu_usage_seconds_total"));
        assert!(query.contains("rate"));
    }

    #[test]
    fn test_k8s_pod_memory_usage_query() {
        let query = CommonQueries::k8s_pod_memory_usage();
        assert_eq!(query, "container_memory_working_set_bytes");
    }

    #[test]
    fn test_aggregation_as_str() {
        assert_eq!(Aggregation::Sum.as_str(), "sum");
        assert_eq!(Aggregation::Avg.as_str(), "avg");
        assert_eq!(Aggregation::Min.as_str(), "min");
        assert_eq!(Aggregation::Max.as_str(), "max");
        assert_eq!(Aggregation::Count.as_str(), "count");
    }
}
