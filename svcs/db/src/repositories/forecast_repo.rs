//! Forecast repository for data access operations.

use crate::error::DbResult;
use crate::pool::DatabasePool;
use chrono::Utc;
use domain::models::{Forecast, ForecastPoint, Recommendation};
use tracing::{debug, info};

/// Forecast repository for CRUD operations.
pub struct ForecastRepository<'a> {
    pool: &'a DatabasePool,
}

impl<'a> ForecastRepository<'a> {
    /// Create a new forecast repository.
    #[must_use]
    pub fn new(pool: &'a DatabasePool) -> Self {
        Self { pool }
    }

    /// Create a forecast with its points in a transaction.
    pub async fn create_forecast(
        &self,
        forecast: &Forecast,
        points: &[ForecastPoint],
    ) -> DbResult<i64> {
        debug!(
            resource_group_id = forecast.resource_group_id,
            metric_name = %forecast.metric_name,
            "Creating forecast with points"
        );

        let forecast_id = match self.pool {
            DatabasePool::Sqlite(pool) => {
                let mut tx = pool.begin().await?;

                let forecast_id = sqlx::query(
                    "INSERT INTO forecasts (resource_group_id, metric_name, model_name, parameters, horizon_start, horizon_end, confidence_score) 
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(forecast.resource_group_id)
                .bind(&forecast.metric_name)
                .bind(&forecast.model_name)
                .bind(&forecast.parameters)
                .bind(forecast.horizon_start)
                .bind(forecast.horizon_end)
                .bind(forecast.confidence_score)
                .execute(&mut *tx)
                .await?
                .last_insert_rowid();

                for point in points {
                    sqlx::query(
                        "INSERT INTO forecast_points (forecast_id, timestamp, predicted_value, lower_bound, upper_bound) 
                         VALUES (?, ?, ?, ?, ?)",
                    )
                    .bind(forecast_id)
                    .bind(point.timestamp)
                    .bind(point.predicted_value)
                    .bind(point.lower_bound)
                    .bind(point.upper_bound)
                    .execute(&mut *tx)
                    .await?;
                }

                tx.commit().await?;
                forecast_id
            }
            DatabasePool::Postgres(pool) => {
                let mut tx = pool.begin().await?;

                let forecast_id: i64 = sqlx::query_scalar(
                    "INSERT INTO forecasts (resource_group_id, metric_name, model_name, parameters, horizon_start, horizon_end, confidence_score) 
                     VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
                )
                .bind(forecast.resource_group_id)
                .bind(&forecast.metric_name)
                .bind(&forecast.model_name)
                .bind(&forecast.parameters)
                .bind(forecast.horizon_start)
                .bind(forecast.horizon_end)
                .bind(forecast.confidence_score)
                .fetch_one(&mut *tx)
                .await?;

                for point in points {
                    sqlx::query(
                        "INSERT INTO forecast_points (forecast_id, timestamp, predicted_value, lower_bound, upper_bound) 
                         VALUES ($1, $2, $3, $4, $5)",
                    )
                    .bind(forecast_id)
                    .bind(point.timestamp)
                    .bind(point.predicted_value)
                    .bind(point.lower_bound)
                    .bind(point.upper_bound)
                    .execute(&mut *tx)
                    .await?;
                }

                tx.commit().await?;
                forecast_id
            }
        };

        info!(forecast_id, points_count = points.len(), "Forecast created");
        Ok(forecast_id)
    }

    /// Get a forecast by ID with its points.
    pub async fn get_forecast(&self, id: i64) -> DbResult<Option<(Forecast, Vec<ForecastPoint>)>> {
        debug!(id, "Getting forecast with points");

        let forecast = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, Forecast>(
                    "SELECT id, resource_group_id, metric_name, model_name, parameters, horizon_start, horizon_end, confidence_score, created_at 
                     FROM forecasts 
                     WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, Forecast>(
                    "SELECT id, resource_group_id, metric_name, model_name, parameters, horizon_start, horizon_end, confidence_score, created_at 
                     FROM forecasts 
                     WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };

        if let Some(forecast) = forecast {
            let points = match self.pool {
                DatabasePool::Sqlite(pool) => sqlx::query_as::<_, ForecastPoint>(
                    "SELECT id, forecast_id, timestamp, predicted_value, lower_bound, upper_bound 
                         FROM forecast_points 
                         WHERE forecast_id = ? 
                         ORDER BY timestamp ASC",
                )
                .bind(id)
                .fetch_all(pool)
                .await?,
                DatabasePool::Postgres(pool) => sqlx::query_as::<_, ForecastPoint>(
                    "SELECT id, forecast_id, timestamp, predicted_value, lower_bound, upper_bound 
                         FROM forecast_points 
                         WHERE forecast_id = $1 
                         ORDER BY timestamp ASC",
                )
                .bind(id)
                .fetch_all(pool)
                .await?,
            };

            Ok(Some((forecast, points)))
        } else {
            Ok(None)
        }
    }

    /// List forecasts by resource group.
    pub async fn list_by_resource_group(&self, group_id: i64) -> DbResult<Vec<Forecast>> {
        debug!(group_id, "Listing forecasts by resource group");

        let forecasts = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, Forecast>(
                    "SELECT id, resource_group_id, metric_name, model_name, parameters, horizon_start, horizon_end, confidence_score, created_at 
                     FROM forecasts 
                     WHERE resource_group_id = ? 
                     ORDER BY created_at DESC",
                )
                .bind(group_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, Forecast>(
                    "SELECT id, resource_group_id, metric_name, model_name, parameters, horizon_start, horizon_end, confidence_score, created_at 
                     FROM forecasts 
                     WHERE resource_group_id = $1 
                     ORDER BY created_at DESC",
                )
                .bind(group_id)
                .fetch_all(pool)
                .await?
            }
        };

        debug!(count = forecasts.len(), "Forecasts listed");
        Ok(forecasts)
    }

    /// Get the latest forecast for a resource group and metric.
    pub async fn get_latest_forecast(
        &self,
        group_id: i64,
        metric_name: &str,
    ) -> DbResult<Option<Forecast>> {
        debug!(group_id, metric_name, "Getting latest forecast");

        let forecast = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, Forecast>(
                    "SELECT id, resource_group_id, metric_name, model_name, parameters, horizon_start, horizon_end, confidence_score, created_at 
                     FROM forecasts 
                     WHERE resource_group_id = ? AND metric_name = ? 
                     ORDER BY created_at DESC 
                     LIMIT 1",
                )
                .bind(group_id)
                .bind(metric_name)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, Forecast>(
                    "SELECT id, resource_group_id, metric_name, model_name, parameters, horizon_start, horizon_end, confidence_score, created_at 
                     FROM forecasts 
                     WHERE resource_group_id = $1 AND metric_name = $2 
                     ORDER BY created_at DESC 
                     LIMIT 1",
                )
                .bind(group_id)
                .bind(metric_name)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(forecast)
    }

    /// Create a recommendation.
    pub async fn create_recommendation(&self, recommendation: &Recommendation) -> DbResult<i64> {
        debug!(
            resource_group_id = recommendation.resource_group_id,
            action = %recommendation.action,
            "Creating recommendation"
        );

        let id = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO recommendations (forecast_id, resource_group_id, action, current_count, recommended_count, reason, status) 
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(recommendation.forecast_id)
                .bind(recommendation.resource_group_id)
                .bind(&recommendation.action)
                .bind(recommendation.current_count)
                .bind(recommendation.recommended_count)
                .bind(&recommendation.reason)
                .bind(&recommendation.status)
                .execute(pool)
                .await?
                .last_insert_rowid()
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar(
                    "INSERT INTO recommendations (forecast_id, resource_group_id, action, current_count, recommended_count, reason, status) 
                     VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
                )
                .bind(recommendation.forecast_id)
                .bind(recommendation.resource_group_id)
                .bind(&recommendation.action)
                .bind(recommendation.current_count)
                .bind(recommendation.recommended_count)
                .bind(&recommendation.reason)
                .bind(&recommendation.status)
                .fetch_one(pool)
                .await?
            }
        };

        info!(id, "Recommendation created");
        Ok(id)
    }

    /// List pending recommendations.
    pub async fn list_pending(&self) -> DbResult<Vec<Recommendation>> {
        debug!("Listing pending recommendations");

        let recommendations = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, Recommendation>(
                    "SELECT id, forecast_id, resource_group_id, action, current_count, recommended_count, reason, status, created_at, decided_at, decided_by 
                     FROM recommendations 
                     WHERE status = 'pending' 
                     ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, Recommendation>(
                    "SELECT id, forecast_id, resource_group_id, action, current_count, recommended_count, reason, status, created_at, decided_at, decided_by 
                     FROM recommendations 
                     WHERE status = 'pending' 
                     ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await?
            }
        };

        debug!(
            count = recommendations.len(),
            "Pending recommendations listed"
        );
        Ok(recommendations)
    }

    /// Update recommendation status (approve/dismiss/execute).
    pub async fn update_status(
        &self,
        id: i64,
        status: &str,
        decided_by: Option<&str>,
    ) -> DbResult<u64> {
        info!(id, status, "Updating recommendation status");

        let now = Utc::now();
        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query(
                "UPDATE recommendations 
                     SET status = ?, decided_at = ?, decided_by = ? 
                     WHERE id = ?",
            )
            .bind(status)
            .bind(now)
            .bind(decided_by)
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE recommendations 
                     SET status = $1, decided_at = $2, decided_by = $3 
                     WHERE id = $4",
            )
            .bind(status)
            .bind(now)
            .bind(decided_by)
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected(),
        };

        info!(id, rows_affected, "Recommendation status updated");
        Ok(rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::run_migrations;
    use crate::repositories::ResourceRepository;
    use chrono::Duration;
    use domain::config::{AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, WebConfig};
    use domain::models::ResourceGroup;
    use domain::Config;

    fn test_config_sqlite() -> Config {
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
                python_path: None,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                outputs: vec![],
            },
            telemetry: domain::config::TelemetryConfig::default(),
            metrics: domain::config::MetricsConfig::default(),
            retention: domain::config::RetentionConfig::default(),
            kubernetes: domain::config::KubernetesConfig::default(),
        }
    }

    async fn setup_test_db() -> (DatabasePool, i64) {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Create a test resource group
        let resource_repo = ResourceRepository::new(&pool);
        let now = Utc::now();
        let group = ResourceGroup {
            id: 0,
            name: "test-group".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: "{}".to_string(),
            created_at: now,
        };
        let group_id = resource_repo.create_group(&group).await.unwrap();

        (pool, group_id)
    }

    #[tokio::test]
    async fn test_create_and_get_forecast() {
        let (pool, group_id) = setup_test_db().await;
        let repo = ForecastRepository::new(&pool);

        let now = Utc::now();
        let forecast = Forecast {
            id: 0,
            resource_group_id: group_id,
            metric_name: "cpu_usage".to_string(),
            model_name: "linear_regression".to_string(),
            parameters: Some(r#"{"lookback_days": 7}"#.to_string()),
            horizon_start: now,
            horizon_end: now + Duration::hours(24),
            confidence_score: Some(0.85),
            created_at: now,
        };

        let points = vec![
            ForecastPoint {
                id: 0,
                forecast_id: 0,
                timestamp: now + Duration::hours(1),
                predicted_value: 0.6,
                lower_bound: Some(0.5),
                upper_bound: Some(0.7),
            },
            ForecastPoint {
                id: 0,
                forecast_id: 0,
                timestamp: now + Duration::hours(2),
                predicted_value: 0.65,
                lower_bound: Some(0.55),
                upper_bound: Some(0.75),
            },
        ];

        let forecast_id = repo.create_forecast(&forecast, &points).await.unwrap();
        assert!(forecast_id > 0);

        let result = repo.get_forecast(forecast_id).await.unwrap();
        assert!(result.is_some());
        let (fetched_forecast, fetched_points) = result.unwrap();
        assert_eq!(fetched_forecast.metric_name, "cpu_usage");
        assert_eq!(fetched_points.len(), 2);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_list_and_get_latest_forecast() {
        let (pool, group_id) = setup_test_db().await;
        let repo = ForecastRepository::new(&pool);

        let now = Utc::now();
        let forecast = Forecast {
            id: 0,
            resource_group_id: group_id,
            metric_name: "memory_usage".to_string(),
            model_name: "arima".to_string(),
            parameters: None,
            horizon_start: now,
            horizon_end: now + Duration::hours(12),
            confidence_score: Some(0.9),
            created_at: now,
        };

        repo.create_forecast(&forecast, &[]).await.unwrap();

        let forecasts = repo.list_by_resource_group(group_id).await.unwrap();
        assert_eq!(forecasts.len(), 1);

        let latest = repo
            .get_latest_forecast(group_id, "memory_usage")
            .await
            .unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().model_name, "arima");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_recommendation_workflow() {
        let (pool, group_id) = setup_test_db().await;
        let repo = ForecastRepository::new(&pool);

        let now = Utc::now();
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

        let id = repo.create_recommendation(&recommendation).await.unwrap();
        assert!(id > 0);

        let pending = repo.list_pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action, "scale_up");

        let rows = repo
            .update_status(id, "approved", Some("admin"))
            .await
            .unwrap();
        assert_eq!(rows, 1);

        let pending_after = repo.list_pending().await.unwrap();
        assert_eq!(pending_after.len(), 0);

        pool.close().await;
    }
}
