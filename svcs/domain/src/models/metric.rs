//! Metric domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;

/// Metric source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricSourceType {
    /// Prometheus data source
    Prometheus,
    /// Agent-based data source
    Agent,
}

impl MetricSourceType {
    /// Convert to database string representation.
    #[must_use] 
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricSourceType::Prometheus => "prometheus",
            MetricSourceType::Agent => "agent",
        }
    }
}

impl FromStr for MetricSourceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "prometheus" => Ok(MetricSourceType::Prometheus),
            "agent" => Ok(MetricSourceType::Agent),
            _ => Err(format!("Invalid metric source type: {s}")),
        }
    }
}

/// Metric source configuration.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MetricSource {
    /// Unique identifier
    pub id: i64,
    /// Source name
    pub name: String,
    /// Source type
    #[sqlx(rename = "type")]
    pub source_type: String,
    /// JSON configuration
    pub config: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl MetricSource {
    /// Get the source type as enum.
    #[must_use] 
    pub fn get_type(&self) -> Option<MetricSourceType> {
        self.source_type.parse().ok()
    }
}

/// Time-series metric data point.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Metric {
    /// Unique identifier
    pub id: i64,
    /// Source ID reference
    pub source_id: i64,
    /// Metric name
    pub name: String,
    /// Metric value
    pub value: f64,
    /// Timestamp of the metric
    pub timestamp: DateTime<Utc>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Metric label (dimensional data).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MetricLabel {
    /// Unique identifier
    pub id: i64,
    /// Metric ID reference
    pub metric_id: i64,
    /// Label key
    pub key: String,
    /// Label value
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_source_type_serialization() {
        let prometheus = MetricSourceType::Prometheus;
        let json = serde_json::to_string(&prometheus).unwrap();
        assert_eq!(json, "\"prometheus\"");

        let agent = MetricSourceType::Agent;
        let json = serde_json::to_string(&agent).unwrap();
        assert_eq!(json, "\"agent\"");
    }

    #[test]
    fn test_metric_source_type_deserialization() {
        let prometheus: MetricSourceType = serde_json::from_str("\"prometheus\"").unwrap();
        assert_eq!(prometheus, MetricSourceType::Prometheus);

        let agent: MetricSourceType = serde_json::from_str("\"agent\"").unwrap();
        assert_eq!(agent, MetricSourceType::Agent);
    }

    #[test]
    fn test_metric_source_type_as_str() {
        assert_eq!(MetricSourceType::Prometheus.as_str(), "prometheus");
        assert_eq!(MetricSourceType::Agent.as_str(), "agent");
    }

    #[test]
    fn test_metric_source_type_from_str() {
        assert_eq!(
            "prometheus".parse::<MetricSourceType>().unwrap(),
            MetricSourceType::Prometheus
        );
        assert_eq!(
            "agent".parse::<MetricSourceType>().unwrap(),
            MetricSourceType::Agent
        );
        assert!("invalid".parse::<MetricSourceType>().is_err());
    }
}
