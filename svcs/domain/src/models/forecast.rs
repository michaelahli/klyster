//! Forecast and recommendation domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;

/// Recommendation action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationAction {
    /// Scale up resources
    ScaleUp,
    /// Scale down resources
    ScaleDown,
    /// No action needed
    None,
}

impl RecommendationAction {
    /// Convert to database string representation.
    #[must_use] 
    pub fn as_str(&self) -> &'static str {
        match self {
            RecommendationAction::ScaleUp => "scale_up",
            RecommendationAction::ScaleDown => "scale_down",
            RecommendationAction::None => "none",
        }
    }
}

impl FromStr for RecommendationAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "scale_up" => Ok(RecommendationAction::ScaleUp),
            "scale_down" => Ok(RecommendationAction::ScaleDown),
            "none" => Ok(RecommendationAction::None),
            _ => Err(format!("Invalid recommendation action: {s}")),
        }
    }
}

/// Recommendation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecommendationStatus {
    /// Pending review
    Pending,
    /// Approved for execution
    Approved,
    /// Dismissed/rejected
    Dismissed,
    /// Executed
    Executed,
}

impl RecommendationStatus {
    /// Convert to database string representation.
    #[must_use] 
    pub fn as_str(&self) -> &'static str {
        match self {
            RecommendationStatus::Pending => "pending",
            RecommendationStatus::Approved => "approved",
            RecommendationStatus::Dismissed => "dismissed",
            RecommendationStatus::Executed => "executed",
        }
    }
}

impl FromStr for RecommendationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(RecommendationStatus::Pending),
            "approved" => Ok(RecommendationStatus::Approved),
            "dismissed" => Ok(RecommendationStatus::Dismissed),
            "executed" => Ok(RecommendationStatus::Executed),
            _ => Err(format!("Invalid recommendation status: {s}")),
        }
    }
}

/// Forecast result.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Forecast {
    /// Unique identifier
    pub id: i64,
    /// Resource group ID reference
    pub resource_group_id: i64,
    /// Metric name
    pub metric_name: String,
    /// Model name used for forecasting
    pub model_name: String,
    /// JSON parameters used
    pub parameters: Option<String>,
    /// Forecast horizon start
    pub horizon_start: DateTime<Utc>,
    /// Forecast horizon end
    pub horizon_end: DateTime<Utc>,
    /// Confidence score (0.0 to 1.0)
    pub confidence_score: Option<f64>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Individual forecast point in time series.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ForecastPoint {
    /// Unique identifier
    pub id: i64,
    /// Forecast ID reference
    pub forecast_id: i64,
    /// Timestamp of prediction
    pub timestamp: DateTime<Utc>,
    /// Predicted value
    pub predicted_value: f64,
    /// Lower bound of prediction interval
    pub lower_bound: Option<f64>,
    /// Upper bound of prediction interval
    pub upper_bound: Option<f64>,
}

/// Scaling recommendation.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Recommendation {
    /// Unique identifier
    pub id: i64,
    /// Forecast ID reference (optional)
    pub forecast_id: Option<i64>,
    /// Resource group ID reference
    pub resource_group_id: i64,
    /// Recommended action
    pub action: String,
    /// Current resource count
    pub current_count: i32,
    /// Recommended resource count
    pub recommended_count: i32,
    /// Reason for recommendation
    pub reason: String,
    /// Recommendation status
    pub status: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Decision timestamp
    pub decided_at: Option<DateTime<Utc>>,
    /// Who made the decision
    pub decided_by: Option<String>,
}

impl Recommendation {
    /// Get the action as enum.
    #[must_use] 
    pub fn get_action(&self) -> Option<RecommendationAction> {
        self.action.parse().ok()
    }

    /// Get the status as enum.
    #[must_use] 
    pub fn get_status(&self) -> Option<RecommendationStatus> {
        self.status.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommendation_action_serialization() {
        let scale_up = RecommendationAction::ScaleUp;
        let json = serde_json::to_string(&scale_up).unwrap();
        assert_eq!(json, "\"scale_up\"");

        let none = RecommendationAction::None;
        let json = serde_json::to_string(&none).unwrap();
        assert_eq!(json, "\"none\"");
    }

    #[test]
    fn test_recommendation_action_deserialization() {
        let scale_up: RecommendationAction = serde_json::from_str("\"scale_up\"").unwrap();
        assert_eq!(scale_up, RecommendationAction::ScaleUp);

        let scale_down: RecommendationAction = serde_json::from_str("\"scale_down\"").unwrap();
        assert_eq!(scale_down, RecommendationAction::ScaleDown);
    }

    #[test]
    fn test_recommendation_action_from_str() {
        assert_eq!(
            "scale_up".parse::<RecommendationAction>().unwrap(),
            RecommendationAction::ScaleUp
        );
        assert_eq!(
            "scale_down".parse::<RecommendationAction>().unwrap(),
            RecommendationAction::ScaleDown
        );
        assert_eq!(
            "none".parse::<RecommendationAction>().unwrap(),
            RecommendationAction::None
        );
        assert!("invalid".parse::<RecommendationAction>().is_err());
    }

    #[test]
    fn test_recommendation_status_serialization() {
        let pending = RecommendationStatus::Pending;
        let json = serde_json::to_string(&pending).unwrap();
        assert_eq!(json, "\"pending\"");

        let executed = RecommendationStatus::Executed;
        let json = serde_json::to_string(&executed).unwrap();
        assert_eq!(json, "\"executed\"");
    }

    #[test]
    fn test_recommendation_status_deserialization() {
        let approved: RecommendationStatus = serde_json::from_str("\"approved\"").unwrap();
        assert_eq!(approved, RecommendationStatus::Approved);

        let dismissed: RecommendationStatus = serde_json::from_str("\"dismissed\"").unwrap();
        assert_eq!(dismissed, RecommendationStatus::Dismissed);
    }

    #[test]
    fn test_recommendation_status_from_str() {
        assert_eq!(
            "pending".parse::<RecommendationStatus>().unwrap(),
            RecommendationStatus::Pending
        );
        assert_eq!(
            "approved".parse::<RecommendationStatus>().unwrap(),
            RecommendationStatus::Approved
        );
        assert_eq!(
            "dismissed".parse::<RecommendationStatus>().unwrap(),
            RecommendationStatus::Dismissed
        );
        assert_eq!(
            "executed".parse::<RecommendationStatus>().unwrap(),
            RecommendationStatus::Executed
        );
        assert!("invalid".parse::<RecommendationStatus>().is_err());
    }

    #[test]
    fn test_recommendation_action_as_str() {
        assert_eq!(RecommendationAction::ScaleUp.as_str(), "scale_up");
        assert_eq!(RecommendationAction::ScaleDown.as_str(), "scale_down");
        assert_eq!(RecommendationAction::None.as_str(), "none");
    }

    #[test]
    fn test_recommendation_status_as_str() {
        assert_eq!(RecommendationStatus::Pending.as_str(), "pending");
        assert_eq!(RecommendationStatus::Approved.as_str(), "approved");
        assert_eq!(RecommendationStatus::Dismissed.as_str(), "dismissed");
        assert_eq!(RecommendationStatus::Executed.as_str(), "executed");
    }
}
