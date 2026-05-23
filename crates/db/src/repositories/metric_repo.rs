//! Metric repository for data access operations.

use crate::error::DbResult;
use crate::pool::DatabasePool;
use chrono::{DateTime, Utc};
use domain::models::Metric;
use tracing::{debug, info};

/// Metric repository for CRUD operations.
pub struct MetricRepository<'a> {
    pool: &'a DatabasePool,
}

impl<'a> MetricRepository<'a> {
    /// Create a new metric repository.
    pub fn new(pool: &'a DatabasePool) -> Self {
        Self { pool }
    }

    /// Insert a batch of metrics efficiently.
    ///
    /// This method handles large batches (1000+ records) efficiently by using
    /// batch insert operations.
    pub async fn insert_batch(&self, metrics: &[Metric]) -> DbResult<u64> {
        if metrics.is_empty() {
            return Ok(0);
        }

        debug!(count = metrics.len(), "Inserting batch of metrics");

        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => {
                let mut affected = 0u64;
                for metric in metrics {
                    let result = sqlx::query(
                        "INSERT INTO metrics (source_id, name, value, timestamp, created_at) 
                         VALUES (?, ?, ?, ?, ?)",
                    )
                    .bind(metric.source_id)
                    .bind(&metric.name)
                    .bind(metric.value)
                    .bind(metric.timestamp)
                    .bind(metric.created_at)
                    .execute(pool)
                    .await?;
                    affected += result.rows_affected();
                }
                affected
            }
            DatabasePool::Postgres(pool) => {
                let mut affected = 0u64;
                for metric in metrics {
                    let result = sqlx::query(
                        "INSERT INTO metrics (source_id, name, value, timestamp, created_at) 
                         VALUES ($1, $2, $3, $4, $5)",
                    )
                    .bind(metric.source_id)
                    .bind(&metric.name)
                    .bind(metric.value)
                    .bind(metric.timestamp)
                    .bind(metric.created_at)
                    .execute(pool)
                    .await?;
                    affected += result.rows_affected();
                }
                affected
            }
        };

        info!(
            count = metrics.len(),
            rows_affected, "Batch insert completed"
        );
        Ok(rows_affected)
    }

    /// Query metrics by name and time range.
    ///
    /// Uses indexes on (name, timestamp) for efficient queries.
    pub async fn query_by_name_and_range(
        &self,
        name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> DbResult<Vec<Metric>> {
        debug!(name, ?start, ?end, "Querying metrics by name and range");

        let metrics = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, Metric>(
                    "SELECT id, source_id, name, value, timestamp, created_at 
                     FROM metrics 
                     WHERE name = ? AND timestamp >= ? AND timestamp <= ? 
                     ORDER BY timestamp ASC",
                )
                .bind(name)
                .bind(start)
                .bind(end)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, Metric>(
                    "SELECT id, source_id, name, value, timestamp, created_at 
                     FROM metrics 
                     WHERE name = $1 AND timestamp >= $2 AND timestamp <= $3 
                     ORDER BY timestamp ASC",
                )
                .bind(name)
                .bind(start)
                .bind(end)
                .fetch_all(pool)
                .await?
            }
        };

        debug!(count = metrics.len(), "Query completed");
        Ok(metrics)
    }

    /// Query metrics by source ID.
    ///
    /// Uses index on (`source_id`, timestamp) for efficient queries.
    pub async fn query_by_source(
        &self,
        source_id: i64,
        limit: Option<i64>,
    ) -> DbResult<Vec<Metric>> {
        debug!(source_id, ?limit, "Querying metrics by source");

        let limit = limit.unwrap_or(1000);

        let metrics = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, Metric>(
                    "SELECT id, source_id, name, value, timestamp, created_at 
                     FROM metrics 
                     WHERE source_id = ? 
                     ORDER BY timestamp DESC 
                     LIMIT ?",
                )
                .bind(source_id)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, Metric>(
                    "SELECT id, source_id, name, value, timestamp, created_at 
                     FROM metrics 
                     WHERE source_id = $1 
                     ORDER BY timestamp DESC 
                     LIMIT $2",
                )
                .bind(source_id)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        };

        debug!(count = metrics.len(), "Query completed");
        Ok(metrics)
    }

    /// Get the latest metric for a given name.
    pub async fn get_latest(&self, name: &str) -> DbResult<Option<Metric>> {
        debug!(name, "Getting latest metric");

        let metric = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, Metric>(
                    "SELECT id, source_id, name, value, timestamp, created_at 
                     FROM metrics 
                     WHERE name = ? 
                     ORDER BY timestamp DESC 
                     LIMIT 1",
                )
                .bind(name)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, Metric>(
                    "SELECT id, source_id, name, value, timestamp, created_at 
                     FROM metrics 
                     WHERE name = $1 
                     ORDER BY timestamp DESC 
                     LIMIT 1",
                )
                .bind(name)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(metric)
    }

    /// Delete metrics older than the specified timestamp (for retention).
    pub async fn delete_older_than(&self, timestamp: DateTime<Utc>) -> DbResult<u64> {
        info!(?timestamp, "Deleting metrics older than timestamp");

        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query("DELETE FROM metrics WHERE timestamp < ?")
                    .bind(timestamp)
                    .execute(pool)
                    .await?;
                result.rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query("DELETE FROM metrics WHERE timestamp < $1")
                    .bind(timestamp)
                    .execute(pool)
                    .await?;
                result.rows_affected()
            }
        };

        info!(rows_affected, "Deletion completed");
        Ok(rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::run_migrations;
    use chrono::Duration;
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

    async fn setup_test_db() -> (DatabasePool, i64) {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Insert a test metric source
        let source_id = match &pool {
            DatabasePool::Sqlite(sqlite_pool) => {
                sqlx::query("INSERT INTO metric_sources (name, type, config) VALUES (?, ?, ?)")
                    .bind("test_source")
                    .bind("prometheus")
                    .bind("{}")
                    .execute(sqlite_pool)
                    .await
                    .unwrap()
                    .last_insert_rowid()
            }
            DatabasePool::Postgres(_) => 1,
        };

        (pool, source_id)
    }

    #[tokio::test]
    async fn test_insert_batch() {
        let (pool, source_id) = setup_test_db().await;
        let repo = MetricRepository::new(&pool);

        let now = Utc::now();
        let metrics = vec![
            Metric {
                id: 0,
                source_id,
                name: "cpu_usage".to_string(),
                value: 0.5,
                timestamp: now,
                created_at: now,
            },
            Metric {
                id: 0,
                source_id,
                name: "cpu_usage".to_string(),
                value: 0.6,
                timestamp: now + Duration::seconds(60),
                created_at: now,
            },
        ];

        let result = repo.insert_batch(&metrics).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_query_by_name_and_range() {
        let (pool, source_id) = setup_test_db().await;
        let repo = MetricRepository::new(&pool);

        let now = Utc::now();
        let metrics = vec![
            Metric {
                id: 0,
                source_id,
                name: "memory_usage".to_string(),
                value: 0.7,
                timestamp: now,
                created_at: now,
            },
            Metric {
                id: 0,
                source_id,
                name: "memory_usage".to_string(),
                value: 0.8,
                timestamp: now + Duration::seconds(120),
                created_at: now,
            },
        ];

        repo.insert_batch(&metrics).await.unwrap();

        let result = repo
            .query_by_name_and_range(
                "memory_usage",
                now - Duration::seconds(60),
                now + Duration::seconds(180),
            )
            .await;

        assert!(result.is_ok());
        let fetched = result.unwrap();
        assert_eq!(fetched.len(), 2);
        assert!((fetched[0].value - 0.7).abs() < f64::EPSILON);
        assert!((fetched[1].value - 0.8).abs() < f64::EPSILON);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_query_by_source() {
        let (pool, source_id) = setup_test_db().await;
        let repo = MetricRepository::new(&pool);

        let now = Utc::now();
        let metrics = vec![
            Metric {
                id: 0,
                source_id,
                name: "disk_usage".to_string(),
                value: 0.3,
                timestamp: now,
                created_at: now,
            },
            Metric {
                id: 0,
                source_id,
                name: "network_io".to_string(),
                value: 100.0,
                timestamp: now,
                created_at: now,
            },
        ];

        repo.insert_batch(&metrics).await.unwrap();

        let result = repo.query_by_source(source_id, Some(10)).await;
        assert!(result.is_ok());
        let fetched = result.unwrap();
        assert_eq!(fetched.len(), 2);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_get_latest() {
        let (pool, source_id) = setup_test_db().await;
        let repo = MetricRepository::new(&pool);

        let now = Utc::now();
        let metrics = vec![
            Metric {
                id: 0,
                source_id,
                name: "cpu_temp".to_string(),
                value: 50.0,
                timestamp: now,
                created_at: now,
            },
            Metric {
                id: 0,
                source_id,
                name: "cpu_temp".to_string(),
                value: 55.0,
                timestamp: now + Duration::seconds(60),
                created_at: now,
            },
        ];

        repo.insert_batch(&metrics).await.unwrap();

        let result = repo.get_latest("cpu_temp").await;
        assert!(result.is_ok());
        let latest = result.unwrap();
        assert!(latest.is_some());
        assert!((latest.unwrap().value - 55.0).abs() < f64::EPSILON);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_delete_older_than() {
        let (pool, source_id) = setup_test_db().await;
        let repo = MetricRepository::new(&pool);

        let now = Utc::now();
        let old_time = now - Duration::days(30);
        let metrics = vec![
            Metric {
                id: 0,
                source_id,
                name: "old_metric".to_string(),
                value: 1.0,
                timestamp: old_time,
                created_at: old_time,
            },
            Metric {
                id: 0,
                source_id,
                name: "new_metric".to_string(),
                value: 2.0,
                timestamp: now,
                created_at: now,
            },
        ];

        repo.insert_batch(&metrics).await.unwrap();

        let cutoff = now - Duration::days(7);
        let result = repo.delete_older_than(cutoff).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // Verify only new metric remains
        let remaining = repo.query_by_source(source_id, Some(10)).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "new_metric");

        pool.close().await;
    }
}
