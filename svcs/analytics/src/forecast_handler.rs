//! Convert protobuf `ForecastResponse` payloads into domain models, persist
//! them via [`ForecastRepository`], and emit a recommendation against the
//! resource group's scaling target.

// `AnalyticsError` carries `tonic::transport::Error`, which is intrinsically large.
#![allow(clippy::result_large_err)]

use chrono::{DateTime, TimeZone, Utc};
use db::repositories::ForecastRepository;
use db::DatabasePool;
use domain::models::{
    Forecast, ForecastPoint, Recommendation, RecommendationAction, RecommendationStatus,
    ScalingTarget,
};
use domain::recommendation_engine::{evaluate, ForecastSummary, RecommendationPolicy};
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::proto::{ForecastPoint as ProtoForecastPoint, ForecastResponse};

/// Errors produced while parsing or persisting a forecast.
#[derive(Error, Debug)]
pub enum ForecastHandlerError {
    /// Forecast response had no points to persist.
    #[error("forecast response contained no points")]
    EmptyForecast,

    /// Required protobuf metadata was missing.
    #[error("forecast response missing metadata")]
    MissingMetadata,

    /// Timestamp could not be represented as a UTC `DateTime`.
    #[error("invalid forecast timestamp {0}")]
    InvalidTimestamp(i64),

    /// Database operation failed while persisting the forecast.
    #[error("database error: {0}")]
    Db(#[from] db::error::DbError),
}

/// Inputs that the analytics caller has on hand but the gRPC payload does not carry.
#[derive(Debug, Clone)]
pub struct ForecastContext<'a> {
    /// Resource group the forecast was generated for.
    pub resource_group_id: i64,
    /// Metric name being forecasted (e.g. `cpu_usage`).
    pub metric_name: &'a str,
    /// Optional scaling target; when present, a recommendation is emitted alongside the forecast.
    pub scaling_target: Option<&'a ScalingTarget>,
    /// Current replica count for the resource group; used to size recommendations.
    pub current_replicas: i32,
    /// Policy thresholds; defaults are sensible for most workloads.
    pub policy: RecommendationPolicy,
}

/// Result of persisting a forecast and (optionally) a recommendation.
#[derive(Debug, Clone, Copy)]
pub struct PersistedForecast {
    /// `forecasts.id` of the row that was inserted.
    pub forecast_id: i64,
    /// `recommendations.id` if one was generated and stored.
    pub recommendation_id: Option<i64>,
    /// Action chosen by the recommendation engine (`None` if no target).
    pub recommendation_action: Option<RecommendationAction>,
}

/// Convert a `ForecastResponse` into a [`Forecast`] plus its [`ForecastPoint`]s.
///
/// The forecast row is filled in from the gRPC metadata; database-managed
/// columns (`id`, `created_at`) are populated with sentinel values.
///
/// # Panics
///
/// Does not panic in practice: the `min`/`max` lookups operate on a slice that
/// is guaranteed non-empty by the [`ForecastHandlerError::EmptyForecast`] check
/// above.
pub fn parse_response(
    response: &ForecastResponse,
    context: &ForecastContext<'_>,
) -> Result<(Forecast, Vec<ForecastPoint>), ForecastHandlerError> {
    if response.points.is_empty() {
        return Err(ForecastHandlerError::EmptyForecast);
    }
    let metadata = response
        .metadata
        .as_ref()
        .ok_or(ForecastHandlerError::MissingMetadata)?;

    let mut points = Vec::with_capacity(response.points.len());
    for point in &response.points {
        points.push(parse_point(point)?);
    }

    let horizon_start = points
        .iter()
        .map(|p| p.timestamp)
        .min()
        .expect("non-empty after EmptyForecast check");
    let horizon_end = points
        .iter()
        .map(|p| p.timestamp)
        .max()
        .expect("non-empty after EmptyForecast check");

    let parameters = if metadata.parameters.is_empty() {
        None
    } else {
        Some(metadata.parameters.clone())
    };
    let confidence_score = quality_metric(&metadata.quality_metrics, "confidence_score")
        .or_else(|| quality_metric(&metadata.quality_metrics, "r_squared"));

    let now = Utc::now();
    let forecast = Forecast {
        id: 0,
        resource_group_id: context.resource_group_id,
        metric_name: context.metric_name.to_string(),
        model_name: metadata.function_name.clone(),
        parameters,
        horizon_start,
        horizon_end,
        confidence_score,
        created_at: now,
    };

    Ok((forecast, points))
}

/// Persist the forecast and, when a scaling target is supplied, a recommendation.
///
/// The forecast and its points are inserted in a single transaction by
/// [`ForecastRepository::create_forecast`]. The recommendation is inserted
/// separately so the forecast is preserved even if recommendation persistence
/// fails (in which case we log and surface the error).
pub async fn persist(
    pool: &DatabasePool,
    response: &ForecastResponse,
    context: &ForecastContext<'_>,
) -> Result<PersistedForecast, ForecastHandlerError> {
    let (forecast, points) = parse_response(response, context)?;
    let repo = ForecastRepository::new(pool);
    let forecast_id = repo.create_forecast(&forecast, &points).await?;
    info!(
        forecast_id,
        resource_group_id = context.resource_group_id,
        metric = context.metric_name,
        model = forecast.model_name.as_str(),
        points = points.len(),
        "persisted forecast"
    );

    let Some(target) = context.scaling_target else {
        return Ok(PersistedForecast {
            forecast_id,
            recommendation_id: None,
            recommendation_action: None,
        });
    };

    let summary = build_summary(&points, context.current_replicas);
    let draft = evaluate(&summary, target, context.policy);
    debug!(
        forecast_id,
        action = draft.action.as_str(),
        current = draft.current_count,
        recommended = draft.recommended_count,
        "evaluated recommendation"
    );

    let recommendation = Recommendation {
        id: 0,
        forecast_id: Some(forecast_id),
        resource_group_id: context.resource_group_id,
        action: draft.action.as_str().to_string(),
        current_count: draft.current_count,
        recommended_count: draft.recommended_count,
        reason: draft.reason,
        status: RecommendationStatus::Pending.as_str().to_string(),
        created_at: Utc::now(),
        decided_at: None,
        decided_by: None,
    };

    let recommendation_id = match repo.create_recommendation(&recommendation).await {
        Ok(id) => Some(id),
        Err(err) => {
            warn!(
                forecast_id,
                error = %err,
                "failed to persist recommendation; forecast remains stored",
            );
            return Err(ForecastHandlerError::Db(err));
        }
    };

    Ok(PersistedForecast {
        forecast_id,
        recommendation_id,
        recommendation_action: Some(draft.action),
    })
}

fn parse_point(point: &ProtoForecastPoint) -> Result<ForecastPoint, ForecastHandlerError> {
    let timestamp = unix_seconds_to_datetime(point.timestamp)?;
    let lower = sanitize_optional(point.lower_bound);
    let upper = sanitize_optional(point.upper_bound);
    Ok(ForecastPoint {
        id: 0,
        forecast_id: 0,
        timestamp,
        predicted_value: point.predicted_value,
        lower_bound: lower,
        upper_bound: upper,
    })
}

fn unix_seconds_to_datetime(seconds: i64) -> Result<DateTime<Utc>, ForecastHandlerError> {
    Utc.timestamp_opt(seconds, 0)
        .single()
        .ok_or(ForecastHandlerError::InvalidTimestamp(seconds))
}

fn sanitize_optional(value: f64) -> Option<f64> {
    if value.is_finite() {
        Some(value)
    } else {
        None
    }
}

fn quality_metric(metrics: &std::collections::HashMap<String, f64>, key: &str) -> Option<f64> {
    metrics
        .get(key)
        .copied()
        .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
}

fn build_summary(points: &[ForecastPoint], current_replicas: i32) -> ForecastSummary {
    let mut peak = f64::NEG_INFINITY;
    let mut sum = 0.0;
    let mut count = 0u32;
    for point in points {
        if !point.predicted_value.is_finite() {
            continue;
        }
        if point.predicted_value > peak {
            peak = point.predicted_value;
        }
        sum += point.predicted_value;
        count += 1;
    }
    let mean = if count > 0 {
        sum / f64::from(count)
    } else {
        0.0
    };
    if !peak.is_finite() {
        peak = 0.0;
    }
    ForecastSummary {
        peak_predicted: peak,
        mean_predicted: mean,
        current_replicas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::proto::{ForecastMetadata, ForecastPoint as ProtoForecastPoint, ForecastResponse};

    fn response(predicted: &[f64]) -> ForecastResponse {
        let points = predicted
            .iter()
            .enumerate()
            .map(|(i, v)| ProtoForecastPoint {
                timestamp: 1_700_000_000 + i64::try_from(i).unwrap() * 60,
                predicted_value: *v,
                lower_bound: *v - 0.1,
                upper_bound: *v + 0.1,
            })
            .collect();
        ForecastResponse {
            points,
            metadata: Some(ForecastMetadata {
                function_name: "linear_regression".to_string(),
                execution_time_ms: 12,
                parameters: r#"{"confidence_interval":0.95}"#.to_string(),
                quality_metrics: HashMap::from([("confidence_score".to_string(), 0.87)]),
            }),
        }
    }

    fn context(target: Option<&ScalingTarget>, current: i32) -> ForecastContext<'_> {
        ForecastContext {
            resource_group_id: 7,
            metric_name: "cpu_usage",
            scaling_target: target,
            current_replicas: current,
            policy: RecommendationPolicy::default(),
        }
    }

    #[test]
    fn parse_response_extracts_metadata_and_horizon() {
        let resp = response(&[0.5, 0.6, 0.7]);
        let (forecast, points) = parse_response(&resp, &context(None, 3)).unwrap();
        assert_eq!(forecast.resource_group_id, 7);
        assert_eq!(forecast.metric_name, "cpu_usage");
        assert_eq!(forecast.model_name, "linear_regression");
        assert!(forecast.parameters.is_some());
        assert!((forecast.confidence_score.unwrap() - 0.87).abs() < f64::EPSILON);
        assert_eq!(points.len(), 3);
        assert!(forecast.horizon_start <= forecast.horizon_end);
    }

    #[test]
    fn parse_response_rejects_empty_points() {
        let mut resp = response(&[]);
        resp.metadata = Some(ForecastMetadata::default());
        let err = parse_response(&resp, &context(None, 1)).unwrap_err();
        assert!(matches!(err, ForecastHandlerError::EmptyForecast));
    }

    #[test]
    fn parse_response_rejects_missing_metadata() {
        let mut resp = response(&[0.1]);
        resp.metadata = None;
        let err = parse_response(&resp, &context(None, 1)).unwrap_err();
        assert!(matches!(err, ForecastHandlerError::MissingMetadata));
    }

    #[test]
    fn parse_response_filters_non_finite_bounds() {
        let mut resp = response(&[0.5]);
        resp.points[0].lower_bound = f64::NAN;
        resp.points[0].upper_bound = f64::INFINITY;
        let (_, points) = parse_response(&resp, &context(None, 1)).unwrap();
        assert!(points[0].lower_bound.is_none());
        assert!(points[0].upper_bound.is_none());
    }

    #[test]
    fn build_summary_handles_non_finite_values() {
        let now = Utc::now();
        let points = vec![
            ForecastPoint {
                id: 0,
                forecast_id: 0,
                timestamp: now,
                predicted_value: 0.4,
                lower_bound: None,
                upper_bound: None,
            },
            ForecastPoint {
                id: 0,
                forecast_id: 0,
                timestamp: now,
                predicted_value: f64::NAN,
                lower_bound: None,
                upper_bound: None,
            },
            ForecastPoint {
                id: 0,
                forecast_id: 0,
                timestamp: now,
                predicted_value: 0.8,
                lower_bound: None,
                upper_bound: None,
            },
        ];
        let summary = build_summary(&points, 4);
        assert!((summary.peak_predicted - 0.8).abs() < f64::EPSILON);
        assert!((summary.mean_predicted - 0.6).abs() < f64::EPSILON);
        assert_eq!(summary.current_replicas, 4);
    }

    #[test]
    fn quality_metric_drops_out_of_range_values() {
        let mut metrics = HashMap::new();
        metrics.insert("confidence_score".to_string(), 1.5);
        assert!(quality_metric(&metrics, "confidence_score").is_none());
        metrics.insert("confidence_score".to_string(), 0.5);
        assert!((quality_metric(&metrics, "confidence_score").unwrap() - 0.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn persist_stores_forecast_and_recommendation() {
        let pool = setup_pool().await;
        let group_id = create_group(&pool).await;
        let scaling_target = ScalingTarget {
            id: 0,
            resource_group_id: group_id,
            metric_name: "cpu_usage".to_string(),
            min_replicas: 1,
            max_replicas: 10,
            target_value: 0.5,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        db::repositories::ResourceRepository::new(&pool)
            .set_scaling_target(&scaling_target)
            .await
            .unwrap();

        let resp = response(&[0.9, 0.95, 1.0]);
        let mut ctx = context(Some(&scaling_target), 3);
        ctx.resource_group_id = group_id;

        let outcome = persist(&pool, &resp, &ctx).await.unwrap();
        assert!(outcome.forecast_id > 0);
        assert!(outcome.recommendation_id.is_some());
        assert_eq!(
            outcome.recommendation_action,
            Some(RecommendationAction::ScaleUp)
        );

        let stored = ForecastRepository::new(&pool)
            .get_forecast(outcome.forecast_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.0.metric_name, "cpu_usage");
        assert_eq!(stored.1.len(), 3);

        let pending = ForecastRepository::new(&pool).list_pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action, "scale_up");
        assert_eq!(pending[0].forecast_id, Some(outcome.forecast_id));

        pool.close().await;
    }

    #[tokio::test]
    async fn persist_skips_recommendation_when_no_target() {
        let pool = setup_pool().await;
        let group_id = create_group(&pool).await;

        let resp = response(&[0.5, 0.6]);
        let mut ctx = context(None, 3);
        ctx.resource_group_id = group_id;

        let outcome = persist(&pool, &resp, &ctx).await.unwrap();
        assert!(outcome.forecast_id > 0);
        assert!(outcome.recommendation_id.is_none());
        assert!(outcome.recommendation_action.is_none());

        let pending = ForecastRepository::new(&pool).list_pending().await.unwrap();
        assert!(pending.is_empty());

        pool.close().await;
    }

    async fn setup_pool() -> DatabasePool {
        use domain::config::{
            AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, MetricsConfig,
            PrometheusAgentConfig, RetentionConfig, TelemetryConfig, WebConfig,
        };
        use domain::Config;

        let config = Config {
            database: DatabaseConfig {
                db_type: "sqlite".to_string(),
                sqlite_path: ":memory:".to_string(),
                postgres_url: None,
                pool_size: 5,
                wal_mode: false,
            },
            web: WebConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
                workers: 1,
            },
            agent: AgentConfig {
                enabled: false,
                collection_interval_secs: 60,
                prometheus: PrometheusAgentConfig::default(),
            },
            analytics: AnalyticsConfig {
                enabled: false,
                grpc_endpoint: "http://127.0.0.1:50051".to_string(),
                python_path: None,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                outputs: vec![],
            },
            telemetry: TelemetryConfig::default(),
            metrics: MetricsConfig::default(),
            retention: RetentionConfig::default(),
        };
        let pool = DatabasePool::new(&config).await.unwrap();
        db::migrate::run_migrations(&pool).await.unwrap();
        pool
    }

    async fn create_group(pool: &DatabasePool) -> i64 {
        use domain::models::ResourceGroup;
        let now = Utc::now();
        db::repositories::ResourceRepository::new(pool)
            .create_group(&ResourceGroup {
                id: 0,
                name: "test-group".to_string(),
                description: None,
                provider_type: "kubernetes".to_string(),
                provider_config: "{}".to_string(),
                created_at: now,
            })
            .await
            .unwrap()
    }
}
