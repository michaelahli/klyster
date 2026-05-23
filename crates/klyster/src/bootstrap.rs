//! Application bootstrap and component orchestration.

use db::{run_migrations, DatabasePool};
use domain::shutdown::ShutdownCoordinator;
use domain::Config;
use tracing::{error, info};

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

    // Create shutdown coordinator
    let shutdown = ShutdownCoordinator::default();
    let shutdown_signal = shutdown.signal();

    // Start components as tokio tasks
    let mut handles = vec![];

    if components.web {
        info!("Starting web component");
        let mut rx = shutdown_signal.subscribe();
        let handle = tokio::spawn(async move {
            tokio::select! {
                () = run_web_component() => {
                    info!("Web component stopped");
                }
                _ = rx.recv() => {
                    info!("Web component received shutdown signal");
                }
            }
        });
        handles.push(handle);
    }

    if components.agent {
        info!("Starting agent component");
        let mut rx = shutdown_signal.subscribe();
        let handle = tokio::spawn(async move {
            tokio::select! {
                () = run_agent_component() => {
                    info!("Agent component stopped");
                }
                _ = rx.recv() => {
                    info!("Agent component received shutdown signal");
                }
            }
        });
        handles.push(handle);
    }

    if components.analytics {
        info!("Starting analytics component");
        let mut rx = shutdown_signal.subscribe();
        let handle = tokio::spawn(async move {
            tokio::select! {
                () = run_analytics_component() => {
                    info!("Analytics component stopped");
                }
                _ = rx.recv() => {
                    info!("Analytics component received shutdown signal");
                }
            }
        });
        handles.push(handle);
    }

    if components.ui {
        info!("Starting UI component");
        let mut rx = shutdown_signal.subscribe();
        let handle = tokio::spawn(async move {
            tokio::select! {
                () = run_ui_component() => {
                    info!("UI component stopped");
                }
                _ = rx.recv() => {
                    info!("UI component received shutdown signal");
                }
            }
        });
        handles.push(handle);
    }

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

/// Run web component (placeholder).
async fn run_web_component() {
    info!("Web component running");
    // Placeholder: actual implementation will be in M2
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

/// Run agent component (placeholder).
async fn run_agent_component() {
    info!("Agent component running");
    // Placeholder: actual implementation will be in M6
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
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
