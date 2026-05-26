//! Recommendation endpoints.

use crate::dto::forecasts::{
    DecideRecommendationRequest, RecommendationListResponse, RecommendationResponse,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use db::repositories::ForecastRepository;
use serde::Deserialize;
use tracing::debug;

/// Query parameters for listing recommendations.
#[derive(Debug, Deserialize)]
pub struct ListRecommendationsQuery {
    /// Filter by status.
    pub status: Option<String>,
}

/// List recommendations.
///
/// GET /api/v1/recommendations
pub async fn list_recommendations(
    State(state): State<AppState>,
    Query(query): Query<ListRecommendationsQuery>,
) -> ApiResult<Json<RecommendationListResponse>> {
    debug!(?query, "Listing recommendations");

    let repo = ForecastRepository::new(state.db());

    // For now, only support listing pending recommendations
    // In a full implementation, we'd have methods to filter by status
    let recommendations = if query.status.as_deref() == Some("pending") || query.status.is_none() {
        repo.list_pending().await?
    } else {
        vec![]
    };

    let total = recommendations.len();
    let recommendations = recommendations
        .into_iter()
        .map(RecommendationResponse::from_model)
        .collect();

    Ok(Json(RecommendationListResponse {
        recommendations,
        total,
    }))
}

/// List pending recommendations (shortcut).
///
/// GET /api/v1/recommendations/pending
pub async fn list_pending_recommendations(
    State(state): State<AppState>,
) -> ApiResult<Json<RecommendationListResponse>> {
    debug!("Listing pending recommendations");

    let repo = ForecastRepository::new(state.db());
    let recommendations = repo.list_pending().await?;

    let total = recommendations.len();
    let recommendations = recommendations
        .into_iter()
        .map(RecommendationResponse::from_model)
        .collect();

    Ok(Json(RecommendationListResponse {
        recommendations,
        total,
    }))
}

/// Approve a recommendation.
///
/// POST /api/v1/recommendations/:id/approve
pub async fn approve_recommendation(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<DecideRecommendationRequest>,
) -> ApiResult<Json<RecommendationResponse>> {
    debug!(id, decided_by = ?req.decided_by, "Approving recommendation");

    let repo = ForecastRepository::new(state.db());

    let rows = repo
        .update_status(id, "approved", req.decided_by.as_deref())
        .await?;

    if rows == 0 {
        return Err(ApiError::NotFound(format!(
            "Recommendation {} not found",
            id
        )));
    }

    // Fetch the updated recommendation
    // For now, we'll return a placeholder since we don't have a get_by_id method
    // In a full implementation, we'd add that method to the repository
    Ok(Json(RecommendationResponse {
        id,
        forecast_id: None,
        resource_group_id: 0,
        action: "approved".to_string(),
        current_count: 0,
        recommended_count: 0,
        reason: "Approved".to_string(),
        status: "approved".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        decided_at: Some(chrono::Utc::now().to_rfc3339()),
        decided_by: req.decided_by,
    }))
}

/// Dismiss a recommendation.
///
/// POST /api/v1/recommendations/:id/dismiss
pub async fn dismiss_recommendation(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<DecideRecommendationRequest>,
) -> ApiResult<Json<RecommendationResponse>> {
    debug!(id, decided_by = ?req.decided_by, "Dismissing recommendation");

    let repo = ForecastRepository::new(state.db());

    let rows = repo
        .update_status(id, "dismissed", req.decided_by.as_deref())
        .await?;

    if rows == 0 {
        return Err(ApiError::NotFound(format!(
            "Recommendation {} not found",
            id
        )));
    }

    // Fetch the updated recommendation
    // For now, we'll return a placeholder since we don't have a get_by_id method
    Ok(Json(RecommendationResponse {
        id,
        forecast_id: None,
        resource_group_id: 0,
        action: "dismissed".to_string(),
        current_count: 0,
        recommended_count: 0,
        reason: "Dismissed".to_string(),
        status: "dismissed".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        decided_at: Some(chrono::Utc::now().to_rfc3339()),
        decided_by: req.decided_by,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use db::migrate::run_migrations;
    use db::pool::DatabasePool;
    use db::repositories::ResourceRepository;
    use domain::config::{
        AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, MetricsConfig,
        RetentionConfig, TelemetryConfig, WebConfig,
    };
    use domain::models::{Recommendation, ResourceGroup};
    use domain::Config;

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
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                outputs: vec![],
            },
            telemetry: TelemetryConfig::default(),
            metrics: MetricsConfig::default(),
            retention: RetentionConfig::default(),
        }
    }

    async fn setup_test_state() -> (AppState, i64) {
        let config = test_config();
        let pool = DatabasePool::new(&config).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Create a test resource group
        let resource_repo = ResourceRepository::new(&pool);
        let now = chrono::Utc::now();
        let group = ResourceGroup {
            id: 0,
            name: "test-group".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: "{}".to_string(),
            created_at: now,
        };
        let group_id = resource_repo.create_group(&group).await.unwrap();

        (AppState::new(pool, std::sync::Arc::new(config)), group_id)
    }

    #[tokio::test]
    async fn test_list_recommendations() {
        let (state, group_id) = setup_test_state().await;

        // Create a recommendation
        let repo = ForecastRepository::new(state.db());
        let now = chrono::Utc::now();
        let recommendation = Recommendation {
            id: 0,
            forecast_id: None,
            resource_group_id: group_id,
            action: "scale_up".to_string(),
            current_count: 3,
            recommended_count: 5,
            reason: "CPU usage predicted to exceed 80%".to_string(),
            status: "pending".to_string(),
            created_at: now,
            decided_at: None,
            decided_by: None,
        };
        repo.create_recommendation(&recommendation).await.unwrap();

        // List recommendations
        let query = ListRecommendationsQuery {
            status: Some("pending".to_string()),
        };
        let result = list_recommendations(State(state.clone()), Query(query)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.total, 1);
        assert_eq!(response.recommendations.len(), 1);
        assert_eq!(response.recommendations[0].action, "scale_up");
    }

    #[tokio::test]
    async fn test_list_pending_recommendations() {
        let (state, group_id) = setup_test_state().await;

        // Create a recommendation
        let repo = ForecastRepository::new(state.db());
        let now = chrono::Utc::now();
        let recommendation = Recommendation {
            id: 0,
            forecast_id: None,
            resource_group_id: group_id,
            action: "scale_down".to_string(),
            current_count: 5,
            recommended_count: 3,
            reason: "Low CPU usage".to_string(),
            status: "pending".to_string(),
            created_at: now,
            decided_at: None,
            decided_by: None,
        };
        repo.create_recommendation(&recommendation).await.unwrap();

        // List pending
        let result = list_pending_recommendations(State(state.clone())).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.total, 1);
        assert_eq!(response.recommendations[0].status, "pending");
    }

    #[tokio::test]
    async fn test_approve_recommendation() {
        let (state, group_id) = setup_test_state().await;

        // Create a recommendation
        let repo = ForecastRepository::new(state.db());
        let now = chrono::Utc::now();
        let recommendation = Recommendation {
            id: 0,
            forecast_id: None,
            resource_group_id: group_id,
            action: "scale_up".to_string(),
            current_count: 3,
            recommended_count: 5,
            reason: "Test".to_string(),
            status: "pending".to_string(),
            created_at: now,
            decided_at: None,
            decided_by: None,
        };
        let rec_id = repo.create_recommendation(&recommendation).await.unwrap();

        // Approve it
        let req = DecideRecommendationRequest {
            decided_by: Some("admin".to_string()),
        };
        let result = approve_recommendation(State(state.clone()), Path(rec_id), Json(req)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status, "approved");
        assert_eq!(response.decided_by, Some("admin".to_string()));
    }

    #[tokio::test]
    async fn test_dismiss_recommendation() {
        let (state, group_id) = setup_test_state().await;

        // Create a recommendation
        let repo = ForecastRepository::new(state.db());
        let now = chrono::Utc::now();
        let recommendation = Recommendation {
            id: 0,
            forecast_id: None,
            resource_group_id: group_id,
            action: "scale_down".to_string(),
            current_count: 5,
            recommended_count: 3,
            reason: "Test".to_string(),
            status: "pending".to_string(),
            created_at: now,
            decided_at: None,
            decided_by: None,
        };
        let rec_id = repo.create_recommendation(&recommendation).await.unwrap();

        // Dismiss it
        let req = DecideRecommendationRequest {
            decided_by: Some("user".to_string()),
        };
        let result = dismiss_recommendation(State(state.clone()), Path(rec_id), Json(req)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status, "dismissed");
    }

    #[tokio::test]
    async fn test_approve_nonexistent_recommendation() {
        let (state, _) = setup_test_state().await;

        let req = DecideRecommendationRequest { decided_by: None };
        let result = approve_recommendation(State(state.clone()), Path(999), Json(req)).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }
}
