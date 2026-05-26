//! Metric source repository for data access operations.

use crate::error::DbResult;
use crate::pool::DatabasePool;
use domain::models::MetricSource;
use tracing::{debug, info};

/// Metric source repository for CRUD operations.
pub struct MetricSourceRepository<'a> {
    pool: &'a DatabasePool,
}

impl<'a> MetricSourceRepository<'a> {
    /// Create a new metric source repository.
    pub fn new(pool: &'a DatabasePool) -> Self {
        Self { pool }
    }

    /// Create a new metric source.
    pub async fn create(
        &self,
        name: &str,
        source_type: &str,
        config: &str,
    ) -> DbResult<MetricSource> {
        debug!(name, source_type, "Creating metric source");

        let id = match self.pool {
            DatabasePool::Sqlite(pool) => {
                let result =
                    sqlx::query("INSERT INTO metric_sources (name, type, config) VALUES (?, ?, ?)")
                        .bind(name)
                        .bind(source_type)
                        .bind(config)
                        .execute(pool)
                        .await?;
                result.last_insert_rowid()
            }
            DatabasePool::Postgres(pool) => {
                let row: (i64,) = sqlx::query_as(
                    "INSERT INTO metric_sources (name, type, config) VALUES ($1, $2, $3) RETURNING id",
                )
                .bind(name)
                .bind(source_type)
                .bind(config)
                .fetch_one(pool)
                .await?;
                row.0
            }
        };

        info!(id, name, "Metric source created");
        self.get_by_id(id).await?.ok_or_else(|| {
            crate::error::DbError::NotFound(format!("Metric source {} not found after insert", id))
        })
    }

    /// Get a metric source by ID.
    pub async fn get_by_id(&self, id: i64) -> DbResult<Option<MetricSource>> {
        debug!(id, "Getting metric source by ID");

        let source = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, MetricSource>(
                    "SELECT id, name, type, config, created_at, updated_at 
                     FROM metric_sources WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, MetricSource>(
                    "SELECT id, name, type, config, created_at, updated_at 
                     FROM metric_sources WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(source)
    }

    /// Get a metric source by name.
    pub async fn get_by_name(&self, name: &str) -> DbResult<Option<MetricSource>> {
        debug!(name, "Getting metric source by name");

        let source = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, MetricSource>(
                    "SELECT id, name, type, config, created_at, updated_at 
                     FROM metric_sources WHERE name = ?",
                )
                .bind(name)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, MetricSource>(
                    "SELECT id, name, type, config, created_at, updated_at 
                     FROM metric_sources WHERE name = $1",
                )
                .bind(name)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(source)
    }

    /// List all metric sources.
    pub async fn list(&self) -> DbResult<Vec<MetricSource>> {
        debug!("Listing all metric sources");

        let sources = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, MetricSource>(
                    "SELECT id, name, type, config, created_at, updated_at 
                     FROM metric_sources ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, MetricSource>(
                    "SELECT id, name, type, config, created_at, updated_at 
                     FROM metric_sources ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await?
            }
        };

        debug!(count = sources.len(), "Listed metric sources");
        Ok(sources)
    }

    /// Update a metric source.
    pub async fn update(
        &self,
        id: i64,
        name: &str,
        source_type: &str,
        config: &str,
    ) -> DbResult<MetricSource> {
        debug!(id, name, source_type, "Updating metric source");

        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query(
                    "UPDATE metric_sources 
                     SET name = ?, type = ?, config = ?, updated_at = CURRENT_TIMESTAMP 
                     WHERE id = ?",
                )
                .bind(name)
                .bind(source_type)
                .bind(config)
                .bind(id)
                .execute(pool)
                .await?;
                result.rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query(
                    "UPDATE metric_sources 
                     SET name = $1, type = $2, config = $3, updated_at = CURRENT_TIMESTAMP 
                     WHERE id = $4",
                )
                .bind(name)
                .bind(source_type)
                .bind(config)
                .bind(id)
                .execute(pool)
                .await?;
                result.rows_affected()
            }
        };

        if rows_affected == 0 {
            return Err(crate::error::DbError::NotFound(format!(
                "Metric source {} not found",
                id
            )));
        }

        info!(id, "Metric source updated");
        self.get_by_id(id).await?.ok_or_else(|| {
            crate::error::DbError::NotFound(format!("Metric source {} not found after update", id))
        })
    }

    /// Delete a metric source.
    pub async fn delete(&self, id: i64) -> DbResult<bool> {
        debug!(id, "Deleting metric source");

        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query("DELETE FROM metric_sources WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
                result.rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query("DELETE FROM metric_sources WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?;
                result.rows_affected()
            }
        };

        if rows_affected > 0 {
            info!(id, "Metric source deleted");
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::run_migrations;
    use domain::config::{
        AgentConfig, AnalyticsConfig, DatabaseConfig, LoggingConfig, MetricsConfig,
        RetentionConfig, TelemetryConfig, WebConfig,
    };
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
            },
            logging: LoggingConfig::default(),
            telemetry: TelemetryConfig::default(),
            metrics: MetricsConfig::default(),
            retention: RetentionConfig::default(),
        }
    }

    async fn setup_test_db() -> DatabasePool {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();
        run_migrations(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_create_metric_source() {
        let pool = setup_test_db().await;
        let repo = MetricSourceRepository::new(&pool);

        let source = repo
            .create(
                "test_prometheus",
                "prometheus",
                r#"{"url":"http://localhost:9090"}"#,
            )
            .await
            .unwrap();

        assert_eq!(source.name, "test_prometheus");
        assert_eq!(source.source_type, "prometheus");
        assert!(source.id > 0);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let pool = setup_test_db().await;
        let repo = MetricSourceRepository::new(&pool);

        let created = repo
            .create("test_agent", "agent", r#"{"interval":60}"#)
            .await
            .unwrap();

        let fetched = repo.get_by_id(created.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "test_agent");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let pool = setup_test_db().await;
        let repo = MetricSourceRepository::new(&pool);

        repo.create("unique_name", "prometheus", "{}")
            .await
            .unwrap();

        let fetched = repo.get_by_name("unique_name").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().source_type, "prometheus");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_list() {
        let pool = setup_test_db().await;
        let repo = MetricSourceRepository::new(&pool);

        repo.create("source1", "prometheus", "{}").await.unwrap();
        repo.create("source2", "agent", "{}").await.unwrap();

        let sources = repo.list().await.unwrap();
        assert_eq!(sources.len(), 2);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_update() {
        let pool = setup_test_db().await;
        let repo = MetricSourceRepository::new(&pool);

        let created = repo.create("old_name", "prometheus", "{}").await.unwrap();

        let updated = repo
            .update(created.id, "new_name", "agent", r#"{"new":"config"}"#)
            .await
            .unwrap();

        assert_eq!(updated.name, "new_name");
        assert_eq!(updated.source_type, "agent");
        assert_eq!(updated.config, r#"{"new":"config"}"#);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_test_db().await;
        let repo = MetricSourceRepository::new(&pool);

        let created = repo.create("to_delete", "prometheus", "{}").await.unwrap();

        let deleted = repo.delete(created.id).await.unwrap();
        assert!(deleted);

        let fetched = repo.get_by_id(created.id).await.unwrap();
        assert!(fetched.is_none());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let pool = setup_test_db().await;
        let repo = MetricSourceRepository::new(&pool);

        let deleted = repo.delete(99999).await.unwrap();
        assert!(!deleted);

        pool.close().await;
    }
}
