//! DTOs for metric source endpoints.

use serde::{Deserialize, Serialize};

/// Request to create a new metric source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSourceRequest {
    /// Source name (must be unique).
    pub name: String,
    /// Source type: "prometheus" or "agent".
    #[serde(rename = "type")]
    pub source_type: String,
    /// JSON configuration for the source.
    pub config: serde_json::Value,
}

/// Request to update an existing metric source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSourceRequest {
    /// Source name (must be unique).
    pub name: String,
    /// Source type: "prometheus" or "agent".
    #[serde(rename = "type")]
    pub source_type: String,
    /// JSON configuration for the source.
    pub config: serde_json::Value,
}

/// Response for a metric source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceResponse {
    /// Source ID.
    pub id: i64,
    /// Source name.
    pub name: String,
    /// Source type.
    #[serde(rename = "type")]
    pub source_type: String,
    /// JSON configuration.
    pub config: serde_json::Value,
    /// Creation timestamp (ISO8601).
    pub created_at: String,
    /// Last update timestamp (ISO8601).
    pub updated_at: String,
}

impl SourceResponse {
    /// Convert from domain model.
    #[must_use] 
    pub fn from_model(source: domain::models::MetricSource) -> Self {
        Self {
            id: source.id,
            name: source.name,
            source_type: source.source_type,
            config: serde_json::from_str(&source.config).unwrap_or(serde_json::json!({})),
            created_at: source.created_at.to_rfc3339(),
            updated_at: source.updated_at.to_rfc3339(),
        }
    }
}

/// List of metric sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceListResponse {
    /// List of sources.
    pub sources: Vec<SourceResponse>,
    /// Total count.
    pub total: usize,
}
