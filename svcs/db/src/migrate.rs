//! Database migration management.

use crate::error::DbResult;
use crate::pool::DatabasePool;
use tracing::info;

/// Run all pending migrations.
///
/// Migrations are embedded in the binary and run automatically on startup.
/// This function is idempotent and safe to call multiple times.
pub async fn run_migrations(pool: &DatabasePool) -> DbResult<()> {
    info!("Running database migrations");

    match pool {
        DatabasePool::Sqlite(sqlite_pool) => {
            sqlx::migrate!("./migrations").run(sqlite_pool).await?;
            info!("SQLite migrations completed successfully");
        }
        DatabasePool::Postgres(pg_pool) => {
            sqlx::migrate!("./migrations").run(pg_pool).await?;
            info!("PostgreSQL migrations completed successfully");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::config::{AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, WebConfig};
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

    #[tokio::test]
    async fn test_run_migrations() {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();

        // Run migrations
        let result = run_migrations(&pool).await;
        assert!(result.is_ok());

        // Running migrations again should be idempotent
        let result = run_migrations(&pool).await;
        assert!(result.is_ok());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_analytics_functions_schema_created() {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();

        // Run migrations
        run_migrations(&pool).await.unwrap();

        // Verify table exists and seed data was inserted
        match &pool {
            DatabasePool::Sqlite(sqlite_pool) => {
                // Check analytics_functions table exists
                let result = sqlx::query("SELECT COUNT(*) FROM analytics_functions")
                    .fetch_one(sqlite_pool)
                    .await;
                assert!(result.is_ok());

                // Verify seed data (4 predefined functions)
                let count: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM analytics_functions WHERE type = 'predefined'",
                )
                .fetch_one(sqlite_pool)
                .await
                .unwrap();
                assert_eq!(count.0, 4);

                // Verify specific functions exist
                let names: Vec<(String,)> =
                    sqlx::query_as("SELECT name FROM analytics_functions ORDER BY name")
                        .fetch_all(sqlite_pool)
                        .await
                        .unwrap();
                assert_eq!(names.len(), 4);
                assert_eq!(names[0].0, "arima");
                assert_eq!(names[1].0, "linear_regression");
                assert_eq!(names[2].0, "seasonal_decomposition");
                assert_eq!(names[3].0, "threshold_rules");
            }
            DatabasePool::Postgres(_) => {}
        }

        pool.close().await;
    }

    #[tokio::test]
    async fn test_forecasts_schema_created() {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();

        // Run migrations
        run_migrations(&pool).await.unwrap();

        // Verify tables exist by querying them
        match &pool {
            DatabasePool::Sqlite(sqlite_pool) => {
                // Check forecasts table
                let result = sqlx::query("SELECT COUNT(*) FROM forecasts")
                    .fetch_one(sqlite_pool)
                    .await;
                assert!(result.is_ok());

                // Check forecast_points table
                let result = sqlx::query("SELECT COUNT(*) FROM forecast_points")
                    .fetch_one(sqlite_pool)
                    .await;
                assert!(result.is_ok());

                // Check recommendations table
                let result = sqlx::query("SELECT COUNT(*) FROM recommendations")
                    .fetch_one(sqlite_pool)
                    .await;
                assert!(result.is_ok());
            }
            DatabasePool::Postgres(_) => {}
        }

        pool.close().await;
    }

    #[tokio::test]
    async fn test_resources_schema_created() {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();

        // Run migrations
        run_migrations(&pool).await.unwrap();

        // Verify tables exist by querying them
        match &pool {
            DatabasePool::Sqlite(sqlite_pool) => {
                // Check resource_groups table
                let result = sqlx::query("SELECT COUNT(*) FROM resource_groups")
                    .fetch_one(sqlite_pool)
                    .await;
                assert!(result.is_ok());

                // Check resources table
                let result = sqlx::query("SELECT COUNT(*) FROM resources")
                    .fetch_one(sqlite_pool)
                    .await;
                assert!(result.is_ok());

                // Check scaling_targets table
                let result = sqlx::query("SELECT COUNT(*) FROM scaling_targets")
                    .fetch_one(sqlite_pool)
                    .await;
                assert!(result.is_ok());
            }
            DatabasePool::Postgres(_) => {}
        }

        pool.close().await;
    }

    #[tokio::test]
    async fn test_metrics_schema_created() {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();

        // Run migrations
        run_migrations(&pool).await.unwrap();

        // Verify tables exist by querying them
        match &pool {
            DatabasePool::Sqlite(sqlite_pool) => {
                // Check metric_sources table
                let result = sqlx::query("SELECT COUNT(*) FROM metric_sources")
                    .fetch_one(sqlite_pool)
                    .await;
                assert!(result.is_ok());

                // Check metrics table
                let result = sqlx::query("SELECT COUNT(*) FROM metrics")
                    .fetch_one(sqlite_pool)
                    .await;
                assert!(result.is_ok());

                // Check metric_labels table
                let result = sqlx::query("SELECT COUNT(*) FROM metric_labels")
                    .fetch_one(sqlite_pool)
                    .await;
                assert!(result.is_ok());
            }
            DatabasePool::Postgres(_) => {}
        }

        pool.close().await;
    }
}
