//! Application bootstrap and component orchestration.

use std::sync::Arc;
use std::time::Duration;

use agent::prometheus::{
    CollectorConfig, CustomQuery, MetricCollector, PrometheusAdapter, PrometheusClient,
    PrometheusConfig,
};
use db::repositories::{MetricSourceRepository, ResourceRepository};
use db::{run_migrations, DatabasePool};
use domain::k8s::watcher::discovery_sync_interval;
use domain::provider::kubernetes::KubernetesProvider;
use domain::provider::InfraProvider;
use domain::shutdown::ShutdownCoordinator;
use domain::Config;
use tracing::{error, info};
use web::AppState;

/// Application components that can be started.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct Components {
    /// Web server component
    pub web: bool,
    /// Agent component
    pub agent: bool,
    /// Analytics component
    pub analytics: bool,
    /// UI component
    pub ui: bool,
}

/// Bootstrap the application with full startup sequence.
pub async fn bootstrap(
    config: Config,
    components: Components,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Klyster application");
    info!(
        web = components.web,
        agent = components.agent,
        analytics = components.analytics,
        ui = components.ui,
        "Components configuration"
    );

    // Connect to database
    info!("Connecting to database");
    let pool = DatabasePool::new(&config).await.map_err(|e| {
        error!("Failed to connect to database: {}", e);
        e
    })?;
    info!(db_type = pool.db_type(), "Database connected");

    // Run migrations
    info!("Running database migrations");
    run_migrations(&pool).await.map_err(|e| {
        error!("Failed to run migrations: {}", e);
        e
    })?;
    info!("Database migrations completed");

    // Build shared application state
    let config = Arc::new(config);
    let app_state = AppState::new(pool.clone(), Arc::clone(&config));

    // Create shutdown coordinator
    let shutdown = ShutdownCoordinator::default();
    let shutdown_signal = shutdown.signal();

    let handles = start_components(
        &components,
        &config,
        &pool,
        app_state.clone(),
        &shutdown_signal,
    );

    info!("All components started successfully");

    // Wait for shutdown signal
    shutdown.wait_and_shutdown().await;

    // Wait for all component tasks to complete
    for handle in handles {
        let _ = handle.await;
    }

    // Close database pool
    info!("Closing database connection");
    pool.close().await;

    info!("Klyster application stopped");
    Ok(())
}

fn start_components(
    components: &Components,
    config: &Arc<Config>,
    pool: &DatabasePool,
    app_state: AppState,
    shutdown_signal: &domain::shutdown::ShutdownSignal,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    if config.kubernetes.enabled {
        info!("Starting Kubernetes discovery sync component");
        let mut rx = shutdown_signal.subscribe();
        let sync_config = Arc::clone(config);
        let sync_pool = pool.clone();
        handles.push(tokio::spawn(async move {
            tokio::select! {
                () = run_kubernetes_discovery_sync(sync_pool, sync_config) => {
                    info!("Kubernetes discovery sync component stopped");
                }
                _ = rx.recv() => {
                    info!("Kubernetes discovery sync component received shutdown signal");
                }
            }
        }));
    }

    if components.web {
        info!("Starting web component");
        let mut rx = shutdown_signal.subscribe();
        handles.push(tokio::spawn(async move {
            if let Err(e) = run_web_component(app_state, async move {
                let _ = rx.recv().await;
            })
            .await
            {
                error!(error = %e, "Web component failed");
            } else {
                info!("Web component stopped");
            }
        }));
    }

    if components.agent {
        info!(enabled = config.agent.enabled, "Starting agent component");
        let agent_pool = pool.clone();
        let agent_config = Arc::clone(config);
        let mut rx = shutdown_signal.subscribe();
        handles.push(tokio::spawn(async move {
            tokio::select! {
                () = run_agent_component(agent_pool, agent_config) => {
                    info!("Agent component stopped");
                }
                _ = rx.recv() => {
                    info!("Agent component received shutdown signal");
                }
            }
        }));
    }

    if components.analytics {
        handles.push(start_loop_component(
            "Analytics",
            run_analytics_component(),
            shutdown_signal,
        ));
    }

    if components.ui {
        handles.push(start_loop_component(
            "UI",
            run_ui_component(),
            shutdown_signal,
        ));
    }

    handles
}

fn start_loop_component<F>(
    name: &'static str,
    component: F,
    shutdown_signal: &domain::shutdown::ShutdownSignal,
) -> tokio::task::JoinHandle<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    info!(component = name, "Starting component");
    let mut rx = shutdown_signal.subscribe();
    tokio::spawn(async move {
        tokio::select! {
            () = component => {
                info!(component = name, "Component stopped");
            }
            _ = rx.recv() => {
                info!(component = name, "Component received shutdown signal");
            }
        }
    })
}

async fn run_kubernetes_discovery_sync(pool: DatabasePool, config: Arc<Config>) {
    let mut interval = discovery_sync_interval();

    loop {
        interval.tick().await;
        if let Err(err) = sync_kubernetes_resources_once(&pool, &config).await {
            error!(error = %err, "Kubernetes discovery sync failed");
        }
    }
}

async fn sync_kubernetes_resources_once(
    pool: &DatabasePool,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let repo = ResourceRepository::new(pool);
    let groups = repo.list_groups().await?;
    let kubernetes_groups: Vec<_> = groups
        .into_iter()
        .filter(|group| group.provider_type == "kubernetes")
        .collect();

    if kubernetes_groups.is_empty() {
        info!("No Kubernetes resource groups configured for discovery sync");
        return Ok(());
    }

    let provider = KubernetesProvider::new(
        config.kubernetes.kubeconfig_path.as_deref(),
        config.kubernetes.namespaces.clone(),
    )
    .await?;
    let resources = provider.get_resources().await?;

    for group in kubernetes_groups {
        let summary = repo.sync_resources_for_group(group.id, &resources).await?;
        info!(
            group_id = group.id,
            group_name = %group.name,
            discovered = summary.discovered,
            inserted = summary.inserted,
            updated = summary.updated,
            deleted = summary.deleted,
            "Kubernetes resources synced"
        );
    }

    Ok(())
}

/// Run the web component until shutdown is signalled.
async fn run_web_component<F>(
    state: AppState,
    shutdown: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = web::bind(&state).await?;
    web::run(listener, state, shutdown, Duration::from_secs(15)).await?;
    Ok(())
}

/// Run agent component - collects metrics from configured Prometheus source.
async fn run_agent_component(pool: DatabasePool, config: Arc<Config>) {
    if !config.agent.enabled {
        info!("Agent component disabled in configuration");
        return;
    }

    info!("Agent component starting - Prometheus collection enabled");

    let prom_config = PrometheusConfig {
        url: config.agent.prometheus.url.clone(),
        timeout: Duration::from_secs(config.agent.prometheus.timeout_secs),
        auth_token: config.agent.prometheus.auth_token.clone(),
    };

    let client = match PrometheusClient::new(prom_config) {
        Ok(client) => client,
        Err(error) => {
            error!(error = %error, "Failed to create Prometheus client");
            return;
        }
    };

    if let Err(error) = client.health_check().await {
        error!(error = %error, "Prometheus health check failed");
        return;
    }

    let source_id = match ensure_default_prometheus_source(&pool, &config).await {
        Ok(source_id) => source_id,
        Err(error) => {
            error!(error = %error, "Failed to ensure default Prometheus source");
            return;
        }
    };

    let custom_queries = config
        .agent
        .prometheus
        .custom_queries
        .iter()
        .map(|query| CustomQuery {
            name: query.name.clone(),
            query: query.query.clone(),
        })
        .collect();

    let collector_config = CollectorConfig {
        interval: Duration::from_secs(config.agent.collection_interval_secs),
        collect_infrastructure: config.agent.prometheus.collect_infrastructure,
        collect_kubernetes: config.agent.prometheus.collect_kubernetes,
        custom_queries,
    };

    let adapter = PrometheusAdapter::new(client, source_id);
    let collector = MetricCollector::new(adapter, pool, collector_config);

    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    let _shutdown_guard = shutdown_tx;
    collector.run(shutdown_rx).await;
}

async fn ensure_default_prometheus_source(
    pool: &DatabasePool,
    config: &Config,
) -> Result<i64, db::DbError> {
    let repo = MetricSourceRepository::new(pool);
    let source_name = "default-prometheus";
    let source_type = "prometheus";
    let source_config = serde_json::json!({
        "url": config.agent.prometheus.url,
        "timeout_secs": config.agent.prometheus.timeout_secs,
        "collect_infrastructure": config.agent.prometheus.collect_infrastructure,
        "collect_kubernetes": config.agent.prometheus.collect_kubernetes,
        "service_discovery_enabled": config.agent.prometheus.service_discovery_enabled,
        "service_discovery_refresh_secs": config.agent.prometheus.service_discovery_refresh_secs,
    })
    .to_string();

    if let Some(existing) = repo.get_by_name(source_name).await? {
        if existing.source_type == source_type && existing.config == source_config {
            return Ok(existing.id);
        }

        let updated = repo
            .update(existing.id, source_name, source_type, &source_config)
            .await?;
        return Ok(updated.id);
    }

    let created = repo
        .create(source_name, source_type, &source_config)
        .await?;
    Ok(created.id)
}

/// Run analytics component (placeholder).
async fn run_analytics_component() {
    info!("Analytics component running");
    // Placeholder: actual implementation will be in M4
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

/// Run UI component (placeholder).
async fn run_ui_component() {
    info!("UI component running");
    // Placeholder: actual implementation will be in M5
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_components_struct() {
        let components = Components {
            web: true,
            agent: false,
            analytics: false,
            ui: false,
        };
        assert!(components.web);
        assert!(!components.agent);
    }
}
