//! Push metrics endpoint.

use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use serde::Deserialize;
use tracing::{debug, error};

/// Request body for pushing metrics.
#[derive(Debug, Deserialize)]
pub struct PushMetricRequest {
    pub name: String,
    pub value: f64,
    pub timestamp: Option<String>,
}

/// Push metrics endpoint.
///
/// POST /metrics
pub async fn push_metrics(
    Json(metrics): Json<Vec<PushMetricRequest>>,
) -> Result<StatusCode, StatusCode> {
    debug!("Received {} metrics", metrics.len());

    // Validate request size
    if metrics.len() > 1000 {
        error!("Too many metrics: {}", metrics.len());
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    // Process metrics (simplified for now - will add DB storage)
    for metric in metrics {
        if metric.name.is_empty() {
            error!("Invalid metric: empty name");
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(StatusCode::ACCEPTED)
}
