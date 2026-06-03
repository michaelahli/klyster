//! Prometheus HTTP API client.

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tracing::debug;
use url::Url;

/// Prometheus client configuration.
#[derive(Debug, Clone)]
pub struct PrometheusConfig {
    /// Prometheus server URL (e.g., "<http://localhost:9090>")
    pub url: String,
    /// Request timeout
    pub timeout: Duration,
    /// Optional authentication token
    pub auth_token: Option<String>,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:9090".to_string(),
            timeout: Duration::from_secs(30),
            auth_token: None,
        }
    }
}

/// Prometheus client errors.
#[derive(Error, Debug)]
pub enum PrometheusError {
    /// Invalid URL format
    #[error("Invalid Prometheus URL: {0}")]
    InvalidUrl(String),

    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    /// API returned error response
    #[error("Prometheus API error: {status} - {error}")]
    ApiError { status: String, error: String },

    /// Failed to parse response
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// Query returned no data
    #[error("Query returned no data")]
    NoData,
}

/// Prometheus HTTP API client.
pub struct PrometheusClient {
    config: PrometheusConfig,
    client: Client,
    base_url: Url,
}

impl PrometheusClient {
    /// Creates a new Prometheus client.
    ///
    /// # Errors
    ///
    /// Returns `PrometheusError::InvalidUrl` if the URL is malformed.
    pub fn new(config: PrometheusConfig) -> Result<Self, PrometheusError> {
        let base_url =
            Url::parse(&config.url).map_err(|e| PrometheusError::InvalidUrl(e.to_string()))?;

        let mut client_builder = Client::builder().timeout(config.timeout);

        if let Some(token) = &config.auth_token {
            let mut headers = reqwest::header::HeaderMap::new();
            let auth_value = format!("Bearer {token}");
            headers.insert(
                reqwest::header::AUTHORIZATION,
                auth_value
                    .parse()
                    .map_err(|e| PrometheusError::InvalidUrl(format!("Invalid auth token: {e}")))?,
            );
            client_builder = client_builder.default_headers(headers);
        }

        let client = client_builder
            .build()
            .map_err(|e| PrometheusError::InvalidUrl(format!("Failed to build client: {e}")))?;

        Ok(Self {
            config,
            client,
            base_url,
        })
    }

    /// Executes an instant query.
    ///
    /// Queries Prometheus at a single point in time.
    ///
    /// # Errors
    ///
    /// Returns error if the request fails or the response is invalid.
    pub async fn query_instant(
        &self,
        query: &str,
        time: Option<DateTime<Utc>>,
    ) -> Result<QueryResult, PrometheusError> {
        debug!(query = %query, "Executing instant query");

        let mut url = self
            .base_url
            .join("/api/v1/query")
            .map_err(|e| PrometheusError::InvalidUrl(e.to_string()))?;

        url.query_pairs_mut().append_pair("query", query);

        if let Some(t) = time {
            url.query_pairs_mut()
                .append_pair("time", &t.timestamp().to_string());
        }

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            let status = response.status().to_string();
            let error = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(PrometheusError::ApiError { status, error });
        }

        let api_response: ApiResponse = response.json().await.map_err(|e| {
            PrometheusError::ParseError(format!("Failed to parse JSON response: {e}"))
        })?;

        if api_response.status != "success" {
            return Err(PrometheusError::ApiError {
                status: api_response.status,
                error: api_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            });
        }

        api_response
            .data
            .ok_or(PrometheusError::NoData)
            .and_then(|data| data.result.ok_or(PrometheusError::NoData))
    }

    /// Executes a range query.
    ///
    /// Queries Prometheus over a time range.
    ///
    /// # Errors
    ///
    /// Returns error if the request fails or the response is invalid.
    pub async fn query_range(
        &self,
        query: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        step: Duration,
    ) -> Result<QueryResult, PrometheusError> {
        debug!(query = %query, "Executing range query");

        let mut url = self
            .base_url
            .join("/api/v1/query_range")
            .map_err(|e| PrometheusError::InvalidUrl(e.to_string()))?;

        url.query_pairs_mut()
            .append_pair("query", query)
            .append_pair("start", &start.timestamp().to_string())
            .append_pair("end", &end.timestamp().to_string())
            .append_pair("step", &format!("{}s", step.as_secs()));

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            let status = response.status().to_string();
            let error = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(PrometheusError::ApiError { status, error });
        }

        let api_response: ApiResponse = response.json().await.map_err(|e| {
            PrometheusError::ParseError(format!("Failed to parse JSON response: {e}"))
        })?;

        if api_response.status != "success" {
            return Err(PrometheusError::ApiError {
                status: api_response.status,
                error: api_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            });
        }

        api_response
            .data
            .ok_or(PrometheusError::NoData)
            .and_then(|data| data.result.ok_or(PrometheusError::NoData))
    }

    /// Checks if Prometheus is reachable.
    ///
    /// # Errors
    ///
    /// Returns error if the health check fails.
    pub async fn health_check(&self) -> Result<(), PrometheusError> {
        debug!("Performing health check");

        let url = self
            .base_url
            .join("/-/healthy")
            .map_err(|e| PrometheusError::InvalidUrl(e.to_string()))?;

        let response = self.client.get(url).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status().to_string();
            let error = response
                .text()
                .await
                .unwrap_or_else(|_| "Health check failed".to_string());
            Err(PrometheusError::ApiError { status, error })
        }
    }

    /// Gets the Prometheus server configuration.
    #[must_use] 
    pub fn config(&self) -> &PrometheusConfig {
        &self.config
    }
}

/// Prometheus API response wrapper.
#[derive(Debug, Deserialize)]
struct ApiResponse {
    status: String,
    data: Option<ApiData>,
    #[serde(rename = "errorType")]
    #[allow(dead_code)]
    error_type: Option<String>,
    error: Option<String>,
}

/// Prometheus API data section.
#[derive(Debug, Deserialize)]
struct ApiData {
    #[serde(rename = "resultType")]
    #[allow(dead_code)]
    result_type: String,
    result: Option<QueryResult>,
}

/// Query result containing time series data.
pub type QueryResult = Vec<TimeSeries>;

/// A single time series with metric labels and values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    /// Metric labels
    pub metric: MetricLabels,
    /// Data points (for instant queries: single value, for range queries: multiple values)
    #[serde(flatten)]
    pub data: TimeSeriesData,
}

/// Metric labels as key-value pairs.
pub type MetricLabels = std::collections::HashMap<String, String>;

/// Time series data variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TimeSeriesData {
    /// Instant query result: [timestamp, value]
    Instant { value: (f64, String) },
    /// Range query result: [[timestamp, value], ...]
    Range { values: Vec<(f64, String)> },
}

impl TimeSeries {
    /// Gets the metric name from labels.
    pub fn metric_name(&self) -> Option<&str> {
        self.metric.get("__name__").map(std::string::String::as_str)
    }

    /// Extracts all data points as (timestamp, value) pairs.
    pub fn data_points(&self) -> Vec<(DateTime<Utc>, f64)> {
        match &self.data {
            TimeSeriesData::Instant { value } => {
                #[allow(clippy::cast_possible_truncation)]
                let timestamp =
                    DateTime::from_timestamp(value.0 as i64, 0).unwrap_or_else(Utc::now);
                let val = value.1.parse::<f64>().unwrap_or(0.0);
                vec![(timestamp, val)]
            }
            TimeSeriesData::Range { values } => values
                .iter()
                .filter_map(|(ts, val_str)| {
                    #[allow(clippy::cast_possible_truncation)]
                    let timestamp =
                        DateTime::from_timestamp(*ts as i64, 0).unwrap_or_else(Utc::now);
                    let val = val_str.parse::<f64>().ok()?;
                    Some((timestamp, val))
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = PrometheusConfig::default();
        assert_eq!(config.url, "http://localhost:9090");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.auth_token.is_none());
    }

    #[test]
    fn test_client_creation_valid_url() {
        let config = PrometheusConfig::default();
        let result = PrometheusClient::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_creation_invalid_url() {
        let config = PrometheusConfig {
            url: "not a url".to_string(),
            ..Default::default()
        };
        let result = PrometheusClient::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_time_series_metric_name() {
        let mut labels = MetricLabels::new();
        labels.insert("__name__".to_string(), "cpu_usage".to_string());
        labels.insert("instance".to_string(), "localhost".to_string());

        let ts = TimeSeries {
            metric: labels,
            data: TimeSeriesData::Instant {
                value: (1_234_567_890.0, "0.5".to_string()),
            },
        };

        assert_eq!(ts.metric_name(), Some("cpu_usage"));
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_time_series_data_points_instant() {
        let ts = TimeSeries {
            metric: MetricLabels::new(),
            data: TimeSeriesData::Instant {
                value: (1_234_567_890.0, "0.75".to_string()),
            },
        };

        let points = ts.data_points();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].1, 0.75);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_time_series_data_points_range() {
        let ts = TimeSeries {
            metric: MetricLabels::new(),
            data: TimeSeriesData::Range {
                values: vec![
                    (1_234_567_890.0, "0.5".to_string()),
                    (1_234_567_900.0, "0.6".to_string()),
                    (1_234_567_910.0, "0.7".to_string()),
                ],
            },
        };

        let points = ts.data_points();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].1, 0.5);
        assert_eq!(points[1].1, 0.6);
        assert_eq!(points[2].1, 0.7);
    }
}
