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
            telemetry: domain::config::TelemetryConfig::default(),
            metrics: domain::config::MetricsConfig::default(),
            retention: domain::config::RetentionConfig::default(),
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
}
