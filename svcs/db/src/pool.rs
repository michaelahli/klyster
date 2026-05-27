//! Database connection pool abstraction.

use crate::error::{DbError, DbResult};
use domain::Config;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use tracing::info;

/// Database connection pool that supports both `SQLite` and `PostgreSQL`.
#[derive(Debug, Clone)]
pub enum DatabasePool {
    /// `SQLite` connection pool
    Sqlite(SqlitePool),
    /// `PostgreSQL` connection pool
    Postgres(PgPool),
}

impl DatabasePool {
    /// Create a new database pool based on configuration.
    ///
    /// If `PostgreSQL` URL is configured, creates a `PostgreSQL` pool.
    /// Otherwise, creates a `SQLite` pool with WAL mode enabled.
    pub async fn new(config: &Config) -> DbResult<Self> {
        if let Some(postgres_url) = &config.database.postgres_url {
            info!("Connecting to PostgreSQL database");
            let pool = PgPoolOptions::new()
                .max_connections(config.database.pool_size)
                .connect(postgres_url)
                .await
                .map_err(|e| {
                    DbError::Connection(format!("Failed to connect to PostgreSQL: {e}"))
                })?;

            info!("PostgreSQL connection pool created");
            Ok(DatabasePool::Postgres(pool))
        } else {
            info!(path = %config.database.sqlite_path, "Connecting to SQLite database");

            // Ensure parent directory exists
            let db_path = Path::new(&config.database.sqlite_path);
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut options = SqliteConnectOptions::from_str(&format!(
                "sqlite://{}",
                config.database.sqlite_path
            ))
            .map_err(|e| DbError::Config(format!("Invalid SQLite path: {e}")))?
            .create_if_missing(true);

            // Enable WAL mode if configured
            if config.database.wal_mode {
                options = options.journal_mode(SqliteJournalMode::Wal);
                info!("SQLite WAL mode enabled");
            }

            let pool = SqlitePoolOptions::new()
                .max_connections(config.database.pool_size)
                .connect_with(options)
                .await
                .map_err(|e| DbError::Connection(format!("Failed to connect to SQLite: {e}")))?;

            info!("SQLite connection pool created");
            Ok(DatabasePool::Sqlite(pool))
        }
    }

    /// Check database connection health.
    pub async fn ping(&self) -> DbResult<()> {
        match self {
            DatabasePool::Sqlite(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
        }
        Ok(())
    }

    /// Close the database connection pool.
    pub async fn close(&self) {
        match self {
            DatabasePool::Sqlite(pool) => {
                info!("Closing SQLite connection pool");
                pool.close().await;
            }
            DatabasePool::Postgres(pool) => {
                info!("Closing PostgreSQL connection pool");
                pool.close().await;
            }
        }
    }

    /// Get the database type as a string.
    pub fn db_type(&self) -> &str {
        match self {
            DatabasePool::Sqlite(_) => "sqlite",
            DatabasePool::Postgres(_) => "postgres",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::config::{AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, WebConfig};

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
        }
    }

    #[tokio::test]
    async fn test_sqlite_pool_creation() {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();

        assert_eq!(pool.db_type(), "sqlite");

        // Test ping
        pool.ping().await.unwrap();

        pool.close().await;
    }

    #[tokio::test]
    async fn test_sqlite_pool_health_check() {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();

        // Ping should succeed
        assert!(pool.ping().await.is_ok());

        pool.close().await;
    }
}
