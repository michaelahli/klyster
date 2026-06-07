//! Application bootstrap and component orchestration.

use std::sync::Arc;
use std::time::Duration;

use db::repositories::ResourceRepository;
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
        handles.push(start_loop_component(
            "Agent",
            run_agent_component(),
            shutdown_signal,
        ));
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

/// Run agent component - collects metrics from all configured sources.
async fn run_agent_component() {
    info!("Agent component starting - metrics collection enabled");

    // TODO: Implement full agent with dynamic source loading
    // For now, placeholder - will be implemented in phases
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        info!("Agent tick - collection cycle");
    }
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
