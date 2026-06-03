//! DTOs for forecast and recommendation endpoints.

use serde::{Deserialize, Serialize};

/// Response for a forecast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResponse {
    /// Forecast ID.
    pub id: i64,
    /// Resource group ID.
    pub resource_group_id: i64,
    /// Metric name.
    pub metric_name: String,
    /// Model name used.
    pub model_name: String,
    /// JSON parameters.
    pub parameters: Option<serde_json::Value>,
    /// Forecast horizon start (ISO8601).
    pub horizon_start: String,
    /// Forecast horizon end (ISO8601).
    pub horizon_end: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence_score: Option<f64>,
    /// Creation timestamp (ISO8601).
    pub created_at: String,
}

impl ForecastResponse {
    /// Convert from domain model.
    #[must_use] 
    pub fn from_model(forecast: domain::models::Forecast) -> Self {
        let parameters: Option<serde_json::Value> = forecast
            .parameters
            .and_then(|p| serde_json::from_str(&p).ok());

        Self {
            id: forecast.id,
            resource_group_id: forecast.resource_group_id,
            metric_name: forecast.metric_name,
            model_name: forecast.model_name,
            parameters,
            horizon_start: forecast.horizon_start.to_rfc3339(),
            horizon_end: forecast.horizon_end.to_rfc3339(),
            confidence_score: forecast.confidence_score,
            created_at: forecast.created_at.to_rfc3339(),
        }
    }
}

/// Response for a forecast with data points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastDetailResponse {
    /// Forecast information.
    #[serde(flatten)]
    pub forecast: ForecastResponse,
    /// Forecast data points.
    pub points: Vec<ForecastPointResponse>,
}

/// Response for a forecast point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPointResponse {
    /// Point ID.
    pub id: i64,
    /// Forecast ID.
    pub forecast_id: i64,
    /// Timestamp (ISO8601).
    pub timestamp: String,
    /// Predicted value.
    pub predicted_value: f64,
    /// Lower bound.
    pub lower_bound: Option<f64>,
    /// Upper bound.
    pub upper_bound: Option<f64>,
}

impl ForecastPointResponse {
    /// Convert from domain model.
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_model(point: domain::models::ForecastPoint) -> Self {
        Self {
            id: point.id,
            forecast_id: point.forecast_id,
            timestamp: point.timestamp.to_rfc3339(),
            predicted_value: point.predicted_value,
            lower_bound: point.lower_bound,
            upper_bound: point.upper_bound,
        }
    }
}

/// Response for forecast list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastListResponse {
    /// List of forecasts.
    pub forecasts: Vec<ForecastResponse>,
    /// Total count.
    pub total: usize,
}

/// Request to trigger a forecast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerForecastRequest {
    /// Metric name to forecast.
    pub metric_name: String,
    /// Model name to use (optional, defaults to "`linear_regression`").
    pub model_name: Option<String>,
    /// Forecast horizon in hours (optional, defaults to 24).
    pub horizon_hours: Option<i64>,
}

/// Response for a recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationResponse {
    /// Recommendation ID.
    pub id: i64,
    /// Forecast ID (optional).
    pub forecast_id: Option<i64>,
    /// Resource group ID.
    pub resource_group_id: i64,
    /// Recommended action.
    pub action: String,
    /// Current resource count.
    pub current_count: i32,
    /// Recommended resource count.
    pub recommended_count: i32,
    /// Reason for recommendation.
    pub reason: String,
    /// Recommendation status.
    pub status: String,
    /// Creation timestamp (ISO8601).
    pub created_at: String,
    /// Decision timestamp (ISO8601).
    pub decided_at: Option<String>,
    /// Who made the decision.
    pub decided_by: Option<String>,
}

impl RecommendationResponse {
    /// Convert from domain model.
    #[must_use] 
    pub fn from_model(recommendation: domain::models::Recommendation) -> Self {
        Self {
            id: recommendation.id,
            forecast_id: recommendation.forecast_id,
            resource_group_id: recommendation.resource_group_id,
            action: recommendation.action,
            current_count: recommendation.current_count,
            recommended_count: recommendation.recommended_count,
            reason: recommendation.reason,
            status: recommendation.status,
            created_at: recommendation.created_at.to_rfc3339(),
            decided_at: recommendation.decided_at.map(|dt| dt.to_rfc3339()),
            decided_by: recommendation.decided_by,
        }
    }
}

/// Response for recommendation list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationListResponse {
    /// List of recommendations.
    pub recommendations: Vec<RecommendationResponse>,
    /// Total count.
    pub total: usize,
}

/// Request to approve or dismiss a recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecideRecommendationRequest {
    /// Who is making the decision (optional).
    pub decided_by: Option<String>,
}
