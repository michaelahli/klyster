//! Prometheus metrics collection and exposition.

use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, Encoder, HistogramVec,
    IntCounterVec, IntGauge, TextEncoder,
};

lazy_static! {
    /// HTTP request counter by method, path, and status code.
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "klyster_http_requests_total",
        "Total number of HTTP requests",
        &["method", "path", "status"]
    )
    .unwrap();

    /// HTTP request duration histogram by method and path.
    pub static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "klyster_http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "path"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .unwrap();

    /// Number of in-flight HTTP requests.
    pub static ref HTTP_REQUESTS_IN_FLIGHT: IntGauge = register_int_gauge!(
        "klyster_http_requests_in_flight",
        "Number of HTTP requests currently being processed"
    )
    .unwrap();

    /// Database query counter by operation.
    pub static ref DB_QUERIES_TOTAL: IntCounterVec = register_int_counter_vec!(
        "klyster_db_queries_total",
        "Total number of database queries",
        &["operation"]
    )
    .unwrap();

    /// Database query duration histogram by operation.
    pub static ref DB_QUERY_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "klyster_db_query_duration_seconds",
        "Database query duration in seconds",
        &["operation"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .unwrap();

    /// Number of active database connections.
    pub static ref DB_CONNECTIONS_ACTIVE: IntGauge = register_int_gauge!(
        "klyster_db_connections_active",
        "Number of active database connections"
    )
    .unwrap();

    /// Forecast generation counter by model.
    pub static ref FORECASTS_GENERATED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "klyster_forecasts_generated_total",
        "Total number of forecasts generated",
        &["model"]
    )
    .unwrap();

    /// Recommendation counter by action and status.
    pub static ref RECOMMENDATIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "klyster_recommendations_total",
        "Total number of recommendations",
        &["action", "status"]
    )
    .unwrap();

    /// Metric collection counter by source.
    pub static ref METRICS_COLLECTED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "klyster_metrics_collected_total",
        "Total number of metrics collected",
        &["source"]
    )
    .unwrap();
}

/// Gather all metrics and encode them in Prometheus text format.
pub fn gather_metrics() -> Result<String, String> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();

    encoder
        .encode(&metric_families, &mut buffer)
        .map_err(|e| format!("Failed to encode metrics: {e}"))?;

    String::from_utf8(buffer).map_err(|e| format!("Failed to convert metrics to string: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registration() {
        // Just verify that metrics are registered without panicking
        let _ = &*HTTP_REQUESTS_TOTAL;
        let _ = &*HTTP_REQUEST_DURATION_SECONDS;
        let _ = &*HTTP_REQUESTS_IN_FLIGHT;
        let _ = &*DB_QUERIES_TOTAL;
        let _ = &*DB_QUERY_DURATION_SECONDS;
        let _ = &*DB_CONNECTIONS_ACTIVE;
        let _ = &*FORECASTS_GENERATED_TOTAL;
        let _ = &*RECOMMENDATIONS_TOTAL;
        let _ = &*METRICS_COLLECTED_TOTAL;
    }

    #[test]
    fn test_gather_metrics() {
        // Increment a counter
        HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/test", "200"])
            .inc();

        // Gather metrics
        let result = gather_metrics();
        assert!(result.is_ok());

        let metrics_text = result.unwrap();
        assert!(metrics_text.contains("klyster_http_requests_total"));
    }

    #[test]
    fn test_http_metrics() {
        HTTP_REQUESTS_TOTAL
            .with_label_values(&["POST", "/api/v1/test", "201"])
            .inc();

        HTTP_REQUEST_DURATION_SECONDS
            .with_label_values(&["POST", "/api/v1/test"])
            .observe(0.123);

        HTTP_REQUESTS_IN_FLIGHT.inc();
        HTTP_REQUESTS_IN_FLIGHT.dec();

        let metrics = gather_metrics().unwrap();
        assert!(metrics.contains("klyster_http_requests_total"));
        assert!(metrics.contains("klyster_http_request_duration_seconds"));
    }

    #[test]
    fn test_db_metrics() {
        DB_QUERIES_TOTAL.with_label_values(&["select"]).inc();
        DB_QUERY_DURATION_SECONDS
            .with_label_values(&["insert"])
            .observe(0.05);
        DB_CONNECTIONS_ACTIVE.set(5);

        let metrics = gather_metrics().unwrap();
        assert!(metrics.contains("klyster_db_queries_total"));
        assert!(metrics.contains("klyster_db_connections_active"));
    }

    #[test]
    fn test_business_metrics() {
        FORECASTS_GENERATED_TOTAL
            .with_label_values(&["linear_regression"])
            .inc();

        RECOMMENDATIONS_TOTAL
            .with_label_values(&["scale_up", "pending"])
            .inc();

        METRICS_COLLECTED_TOTAL
            .with_label_values(&["prometheus"])
            .inc();

        let metrics = gather_metrics().unwrap();
        assert!(metrics.contains("klyster_forecasts_generated_total"));
        assert!(metrics.contains("klyster_recommendations_total"));
        assert!(metrics.contains("klyster_metrics_collected_total"));
    }
}
