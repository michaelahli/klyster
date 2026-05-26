//! Resource repository for data access operations.

use crate::error::DbResult;
use crate::pool::DatabasePool;
use domain::models::{Resource, ResourceGroup, ScalingTarget};
use tracing::{debug, info};

/// Resource repository for CRUD operations.
pub struct ResourceRepository<'a> {
    pool: &'a DatabasePool,
}

impl<'a> ResourceRepository<'a> {
    /// Create a new resource repository.
    pub fn new(pool: &'a DatabasePool) -> Self {
        Self { pool }
    }

    // Resource Group operations

    /// Create a new resource group.
    pub async fn create_group(&self, group: &ResourceGroup) -> DbResult<i64> {
        debug!(name = %group.name, "Creating resource group");

        let id = match self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query(
                "INSERT INTO resource_groups (name, description, provider_type, provider_config) 
                     VALUES (?, ?, ?, ?)",
            )
            .bind(&group.name)
            .bind(&group.description)
            .bind(&group.provider_type)
            .bind(&group.provider_config)
            .execute(pool)
            .await?
            .last_insert_rowid(),
            DatabasePool::Postgres(pool) => sqlx::query_scalar(
                "INSERT INTO resource_groups (name, description, provider_type, provider_config) 
                     VALUES ($1, $2, $3, $4) RETURNING id",
            )
            .bind(&group.name)
            .bind(&group.description)
            .bind(&group.provider_type)
            .bind(&group.provider_config)
            .fetch_one(pool)
            .await?,
        };

        info!(id, "Resource group created");
        Ok(id)
    }

    /// List all resource groups.
    pub async fn list_groups(&self) -> DbResult<Vec<ResourceGroup>> {
        debug!("Listing all resource groups");

        let groups = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, ResourceGroup>(
                    "SELECT id, name, description, provider_type, provider_config, created_at 
                     FROM resource_groups 
                     ORDER BY name ASC",
                )
                .fetch_all(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, ResourceGroup>(
                    "SELECT id, name, description, provider_type, provider_config, created_at 
                     FROM resource_groups 
                     ORDER BY name ASC",
                )
                .fetch_all(pool)
                .await?
            }
        };

        debug!(count = groups.len(), "Resource groups listed");
        Ok(groups)
    }

    /// Get a resource group by ID.
    pub async fn get_group(&self, id: i64) -> DbResult<Option<ResourceGroup>> {
        debug!(id, "Getting resource group");

        let group = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, ResourceGroup>(
                    "SELECT id, name, description, provider_type, provider_config, created_at 
                     FROM resource_groups 
                     WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, ResourceGroup>(
                    "SELECT id, name, description, provider_type, provider_config, created_at 
                     FROM resource_groups 
                     WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(group)
    }

    /// Update a resource group.
    pub async fn update_group(&self, group: &ResourceGroup) -> DbResult<u64> {
        debug!(id = group.id, "Updating resource group");

        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query(
                "UPDATE resource_groups 
                     SET name = ?, description = ?, provider_type = ?, provider_config = ? 
                     WHERE id = ?",
            )
            .bind(&group.name)
            .bind(&group.description)
            .bind(&group.provider_type)
            .bind(&group.provider_config)
            .bind(group.id)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE resource_groups 
                     SET name = $1, description = $2, provider_type = $3, provider_config = $4 
                     WHERE id = $5",
            )
            .bind(&group.name)
            .bind(&group.description)
            .bind(&group.provider_type)
            .bind(&group.provider_config)
            .bind(group.id)
            .execute(pool)
            .await?
            .rows_affected(),
        };

        info!(id = group.id, rows_affected, "Resource group updated");
        Ok(rows_affected)
    }

    /// Delete a resource group.
    pub async fn delete_group(&self, id: i64) -> DbResult<u64> {
        info!(id, "Deleting resource group");

        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query("DELETE FROM resource_groups WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM resource_groups WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };

        info!(id, rows_affected, "Resource group deleted");
        Ok(rows_affected)
    }

    // Resource operations

    /// Create a new resource.
    pub async fn create_resource(&self, resource: &Resource) -> DbResult<i64> {
        debug!(name = %resource.name, "Creating resource");

        let id = match self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query(
                "INSERT INTO resources (group_id, name, namespace, kind, labels, status) 
                     VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(resource.group_id)
            .bind(&resource.name)
            .bind(&resource.namespace)
            .bind(&resource.kind)
            .bind(&resource.labels)
            .bind(&resource.status)
            .execute(pool)
            .await?
            .last_insert_rowid(),
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar(
                    "INSERT INTO resources (group_id, name, namespace, kind, labels, status) 
                     VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
                )
                .bind(resource.group_id)
                .bind(&resource.name)
                .bind(&resource.namespace)
                .bind(&resource.kind)
                .bind(&resource.labels)
                .bind(&resource.status)
                .fetch_one(pool)
                .await?
            }
        };

        info!(id, "Resource created");
        Ok(id)
    }

    /// List resources by group ID.
    pub async fn list_by_group(&self, group_id: i64) -> DbResult<Vec<Resource>> {
        debug!(group_id, "Listing resources by group");

        let resources = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, Resource>(
                    "SELECT id, group_id, name, namespace, kind, labels, status, created_at, updated_at 
                     FROM resources 
                     WHERE group_id = ? 
                     ORDER BY name ASC",
                )
                .bind(group_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, Resource>(
                    "SELECT id, group_id, name, namespace, kind, labels, status, created_at, updated_at 
                     FROM resources 
                     WHERE group_id = $1 
                     ORDER BY name ASC",
                )
                .bind(group_id)
                .fetch_all(pool)
                .await?
            }
        };

        debug!(count = resources.len(), "Resources listed");
        Ok(resources)
    }

    /// Get a resource by ID.
    pub async fn get_resource(&self, id: i64) -> DbResult<Option<Resource>> {
        debug!(id, "Getting resource");

        let resource = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, Resource>(
                    "SELECT id, group_id, name, namespace, kind, labels, status, created_at, updated_at 
                     FROM resources 
                     WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, Resource>(
                    "SELECT id, group_id, name, namespace, kind, labels, status, created_at, updated_at 
                     FROM resources 
                     WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(resource)
    }

    /// Update a resource.
    pub async fn update_resource(&self, resource: &Resource) -> DbResult<u64> {
        debug!(id = resource.id, "Updating resource");

        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE resources 
                     SET name = ?, namespace = ?, kind = ?, labels = ?, status = ?, updated_at = CURRENT_TIMESTAMP 
                     WHERE id = ?",
                )
                .bind(&resource.name)
                .bind(&resource.namespace)
                .bind(&resource.kind)
                .bind(&resource.labels)
                .bind(&resource.status)
                .bind(resource.id)
                .execute(pool)
                .await?
                .rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE resources 
                     SET name = $1, namespace = $2, kind = $3, labels = $4, status = $5, updated_at = CURRENT_TIMESTAMP 
                     WHERE id = $6",
                )
                .bind(&resource.name)
                .bind(&resource.namespace)
                .bind(&resource.kind)
                .bind(&resource.labels)
                .bind(&resource.status)
                .bind(resource.id)
                .execute(pool)
                .await?
                .rows_affected()
            }
        };

        info!(id = resource.id, rows_affected, "Resource updated");
        Ok(rows_affected)
    }

    /// Delete a resource.
    pub async fn delete_resource(&self, id: i64) -> DbResult<u64> {
        info!(id, "Deleting resource");

        let rows_affected = match self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query("DELETE FROM resources WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query("DELETE FROM resources WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
        };

        info!(id, rows_affected, "Resource deleted");
        Ok(rows_affected)
    }

    // Scaling Target operations

    /// Set a scaling target for a resource group.
    pub async fn set_scaling_target(&self, target: &ScalingTarget) -> DbResult<i64> {
        debug!(resource_group_id = target.resource_group_id, metric_name = %target.metric_name, "Setting scaling target");

        let id = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO scaling_targets (resource_group_id, metric_name, min_replicas, max_replicas, target_value) 
                     VALUES (?, ?, ?, ?, ?) 
                     ON CONFLICT(resource_group_id, metric_name) 
                     DO UPDATE SET min_replicas = ?, max_replicas = ?, target_value = ?, updated_at = CURRENT_TIMESTAMP",
                )
                .bind(target.resource_group_id)
                .bind(&target.metric_name)
                .bind(target.min_replicas)
                .bind(target.max_replicas)
                .bind(target.target_value)
                .bind(target.min_replicas)
                .bind(target.max_replicas)
                .bind(target.target_value)
                .execute(pool)
                .await?
                .last_insert_rowid()
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar(
                    "INSERT INTO scaling_targets (resource_group_id, metric_name, min_replicas, max_replicas, target_value) 
                     VALUES ($1, $2, $3, $4, $5) 
                     ON CONFLICT(resource_group_id, metric_name) 
                     DO UPDATE SET min_replicas = $3, max_replicas = $4, target_value = $5, updated_at = CURRENT_TIMESTAMP 
                     RETURNING id",
                )
                .bind(target.resource_group_id)
                .bind(&target.metric_name)
                .bind(target.min_replicas)
                .bind(target.max_replicas)
                .bind(target.target_value)
                .fetch_one(pool)
                .await?
            }
        };

        info!(id, "Scaling target set");
        Ok(id)
    }

    /// Get scaling targets for a resource group.
    pub async fn get_scaling_targets_by_group(
        &self,
        group_id: i64,
    ) -> DbResult<Vec<ScalingTarget>> {
        debug!(group_id, "Getting scaling targets by group");

        let targets = match self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, ScalingTarget>(
                    "SELECT id, resource_group_id, metric_name, min_replicas, max_replicas, target_value, created_at, updated_at 
                     FROM scaling_targets 
                     WHERE resource_group_id = ? 
                     ORDER BY metric_name ASC",
                )
                .bind(group_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, ScalingTarget>(
                    "SELECT id, resource_group_id, metric_name, min_replicas, max_replicas, target_value, created_at, updated_at 
                     FROM scaling_targets 
                     WHERE resource_group_id = $1 
                     ORDER BY metric_name ASC",
                )
                .bind(group_id)
                .fetch_all(pool)
                .await?
            }
        };

        debug!(count = targets.len(), "Scaling targets retrieved");
        Ok(targets)
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
    async fn test_resource_group_crud() {
        let pool = setup_test_db().await;
        let repo = ResourceRepository::new(&pool);

        let now = Utc::now();
        let group = ResourceGroup {
            id: 0,
            name: "test-cluster".to_string(),
            description: Some("Test Kubernetes cluster".to_string()),
            provider_type: "kubernetes".to_string(),
            provider_config: "{}".to_string(),
            created_at: now,
        };

        // Create
        let id = repo.create_group(&group).await.unwrap();
        assert!(id > 0);

        // Get
        let fetched = repo.get_group(id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "test-cluster");

        // List
        let groups = repo.list_groups().await.unwrap();
        assert_eq!(groups.len(), 1);

        // Update
        let mut updated_group = group.clone();
        updated_group.id = id;
        updated_group.description = Some("Updated description".to_string());
        let rows = repo.update_group(&updated_group).await.unwrap();
        assert_eq!(rows, 1);

        // Delete
        let rows = repo.delete_group(id).await.unwrap();
        assert_eq!(rows, 1);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_resource_crud() {
        let pool = setup_test_db().await;
        let repo = ResourceRepository::new(&pool);

        let now = Utc::now();
        let group = ResourceGroup {
            id: 0,
            name: "test-group".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: "{}".to_string(),
            created_at: now,
        };
        let group_id = repo.create_group(&group).await.unwrap();

        let resource = Resource {
            id: 0,
            group_id,
            name: "test-pod".to_string(),
            namespace: Some("default".to_string()),
            kind: "pod".to_string(),
            labels: Some(r#"{"app":"test"}"#.to_string()),
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        };

        // Create
        let id = repo.create_resource(&resource).await.unwrap();
        assert!(id > 0);

        // Get
        let fetched = repo.get_resource(id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "test-pod");

        // List by group
        let resources = repo.list_by_group(group_id).await.unwrap();
        assert_eq!(resources.len(), 1);

        // Update
        let mut updated_resource = resource.clone();
        updated_resource.id = id;
        updated_resource.status = "inactive".to_string();
        let rows = repo.update_resource(&updated_resource).await.unwrap();
        assert_eq!(rows, 1);

        // Delete
        let rows = repo.delete_resource(id).await.unwrap();
        assert_eq!(rows, 1);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_scaling_target_operations() {
        let pool = setup_test_db().await;
        let repo = ResourceRepository::new(&pool);

        let now = Utc::now();
        let group = ResourceGroup {
            id: 0,
            name: "scaling-group".to_string(),
            description: None,
            provider_type: "kubernetes".to_string(),
            provider_config: "{}".to_string(),
            created_at: now,
        };
        let group_id = repo.create_group(&group).await.unwrap();

        let target = ScalingTarget {
            id: 0,
            resource_group_id: group_id,
            metric_name: "cpu_usage".to_string(),
            min_replicas: 2,
            max_replicas: 10,
            target_value: 0.7,
            created_at: now,
            updated_at: now,
        };

        // Set target
        let id = repo.set_scaling_target(&target).await.unwrap();
        assert!(id > 0);

        // Get targets
        let targets = repo.get_scaling_targets_by_group(group_id).await.unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].metric_name, "cpu_usage");
        assert_eq!(targets[0].min_replicas, 2);
        assert_eq!(targets[0].max_replicas, 10);

        // Update target (upsert)
        let mut updated_target = target.clone();
        updated_target.max_replicas = 15;
        repo.set_scaling_target(&updated_target).await.unwrap();

        let targets = repo.get_scaling_targets_by_group(group_id).await.unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].max_replicas, 15);

        pool.close().await;
    }
}
