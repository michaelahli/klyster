//! Analytics function repository for data access operations.

use crate::error::DbResult;
use crate::pool::DatabasePool;
use domain::models::AnalyticsFunction;
use tracing::{debug, info};

/// Analytics function repository for CRUD operations.
pub struct AnalyticsFunctionRepository<'a> {
    pool: &'a DatabasePool,
}

impl<'a> AnalyticsFunctionRepository<'a> {
    /// Create a new analytics function repository.
    pub fn new(pool: &'a DatabasePool) -> Self {
        Self { pool }
    }

    /// Create a new analytics function.
    pub async fn create(&self, function: &AnalyticsFunction) -> DbResult<i64> {
        debug!(name = %function.name, "Creating analytics function");

        let id = match self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query(
                "INSERT INTO analytics_functions (name, description, type, language, source_code, parameters_schema, is_active) 
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&function.name)
            .bind(&function.description)
            .bind(&function.function_type)
            .bind(&function.language)
            .bind(&function.source_code)
            .bind(&function.parameters_schema)
            .bind(function.is_active)
            .execute(pool)
            .await?
            .last_insert_rowid(),
            DatabasePool::Postgres(pool) => sqlx::query_scalar(
                "INSERT INTO analytics_functions (name, description, type, language, source_code, parameters_schema, is_active) 
                 VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
            )
            .bind(&function.name)
            .bind(&function.description)
            .bind(&function.function_type)
            .bind(&function.language)
            .bind(&function.source_code)
            .bind(&function.parameters_schema)
            .bind(function.is_active)
            .fetch_one(pool)
            .await?,
        };

        info!(id, "Analytics function created");
        Ok(id)
    }

    /// List all analytics functions.
    pub async fn list_all(&self) -> DbResult<Vec<AnalyticsFunction>> {
        debug!("Listing all analytics functions");

        let functions = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, AnalyticsFunction>(
                    "SELECT id, name, description, type, language, source_code, parameters_schema, is_active, created_at, updated_at 
                     FROM analytics_functions 
                     ORDER BY name ASC",
                )
                .fetch_all(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, AnalyticsFunction>(
                    "SELECT id, name, description, type, language, source_code, parameters_schema, is_active, created_at, updated_at 
                     FROM analytics_functions 
                     ORDER BY name ASC",
                )
                .fetch_all(pool)
                .await?
            }
        };

        debug!(count = functions.len(), "Analytics functions listed");
        Ok(functions)
    }

    /// Get an analytics function by ID.
    pub async fn get_by_id(&self, id: i64) -> DbResult<Option<AnalyticsFunction>> {
        debug!(id, "Getting analytics function");

        let function = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, AnalyticsFunction>(
                    "SELECT id, name, description, type, language, source_code, parameters_schema, is_active, created_at, updated_at 
                     FROM analytics_functions 
                     WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, AnalyticsFunction>(
                    "SELECT id, name, description, type, language, source_code, parameters_schema, is_active, created_at, updated_at 
                     FROM analytics_functions 
                     WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(function)
    }

    /// Get an analytics function by name.
    pub async fn get_by_name(&self, name: &str) -> DbResult<Option<AnalyticsFunction>> {
        debug!(name, "Getting analytics function by name");

        let function = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, AnalyticsFunction>(
                    "SELECT id, name, description, type, language, source_code, parameters_schema, is_active, created_at, updated_at 
                     FROM analytics_functions 
                     WHERE name = ?",
                )
                .bind(name)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, AnalyticsFunction>(
                    "SELECT id, name, description, type, language, source_code, parameters_schema, is_active, created_at, updated_at 
                     FROM analytics_functions 
                     WHERE name = $1",
                )
                .bind(name)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(function)
    }

    /// Update an analytics function.
    pub async fn update(&self, function: &AnalyticsFunction) -> DbResult<u64> {
        debug!(id = function.id, "Updating analytics function");

        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query(
                "UPDATE analytics_functions 
                 SET name = ?, description = ?, type = ?, language = ?, source_code = ?, parameters_schema = ?, is_active = ?, updated_at = CURRENT_TIMESTAMP 
                 WHERE id = ?",
            )
            .bind(&function.name)
            .bind(&function.description)
            .bind(&function.function_type)
            .bind(&function.language)
            .bind(&function.source_code)
            .bind(&function.parameters_schema)
            .bind(function.is_active)
            .bind(function.id)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE analytics_functions 
                 SET name = $1, description = $2, type = $3, language = $4, source_code = $5, parameters_schema = $6, is_active = $7, updated_at = CURRENT_TIMESTAMP 
                 WHERE id = $8",
            )
            .bind(&function.name)
            .bind(&function.description)
            .bind(&function.function_type)
            .bind(&function.language)
            .bind(&function.source_code)
            .bind(&function.parameters_schema)
            .bind(function.is_active)
            .bind(function.id)
            .execute(pool)
            .await?
            .rows_affected(),
        };

        info!(id = function.id, rows_affected, "Analytics function updated");
        Ok(rows_affected)
    }

    /// Delete an analytics function.
    pub async fn delete(&self, id: i64) -> DbResult<u64> {
        info!(id, "Deleting analytics function");

        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM analytics_functions WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM analytics_functions WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };

        info!(id, rows_affected, "Analytics function deleted");
        Ok(rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::run_migrations;
    use chrono::Utc;
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

    async fn setup_test_db() -> DatabasePool {
        let config = test_config_sqlite();
        let pool = DatabasePool::new(&config).await.unwrap();
        run_migrations(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_analytics_function_crud() {
        let pool = setup_test_db().await;
        let repo = AnalyticsFunctionRepository::new(&pool);

        let now = Utc::now();
        let function = AnalyticsFunction {
            id: 0,
            name: "test_linear_regression".to_string(),
            description: "Simple linear regression forecasting".to_string(),
            function_type: "predefined".to_string(),
            language: "python".to_string(),
            source_code: None,
            parameters_schema: Some(r#"{"lookback_days": "integer"}"#.to_string()),
            is_active: true,
            created_at: now,
            updated_at: now,
        };

        // Create
        let id = repo.create(&function).await.unwrap();
        assert!(id > 0);

        // Get by ID
        let fetched = repo.get_by_id(id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "test_linear_regression");

        // Get by name
        let fetched = repo.get_by_name("test_linear_regression").await.unwrap();
        assert!(fetched.is_some());

        // List
        let functions = repo.list_all().await.unwrap();
        assert!(functions.len() >= 1);

        // Update
        let mut updated_function = function.clone();
        updated_function.id = id;
        updated_function.description = "Updated description".to_string();
        let rows = repo.update(&updated_function).await.unwrap();
        assert_eq!(rows, 1);

        // Delete
        let rows = repo.delete(id).await.unwrap();
        assert_eq!(rows, 1);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_custom_function() {
        let pool = setup_test_db().await;
        let repo = AnalyticsFunctionRepository::new(&pool);

        let now = Utc::now();
        let function = AnalyticsFunction {
            id: 0,
            name: "my_custom_model".to_string(),
            description: "Custom ML model".to_string(),
            function_type: "custom".to_string(),
            language: "python".to_string(),
            source_code: Some("def forecast(data): return data".to_string()),
            parameters_schema: Some(r#"{"param1": "string"}"#.to_string()),
            is_active: true,
            created_at: now,
            updated_at: now,
        };

        let id = repo.create(&function).await.unwrap();
        let fetched = repo.get_by_id(id).await.unwrap().unwrap();

        assert_eq!(fetched.name, "my_custom_model");
        assert!(fetched.source_code.is_some());
        assert_eq!(fetched.function_type, "custom");

        pool.close().await;
    }
}
