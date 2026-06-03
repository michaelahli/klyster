//! Shared application state for HTTP handlers.

use db::DatabasePool;
use domain::Config;
use std::sync::Arc;

/// Shared state injected into every request handler.
///
/// Cheaply cloneable. The inner fields are reference-counted so cloning the
/// state does not duplicate the database pool or configuration.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    db: DatabasePool,
    config: Arc<Config>,
}

impl AppState {
    /// Create a new `AppState` with the given database pool and configuration.
    #[must_use]
    pub fn new(db: DatabasePool, config: Arc<Config>) -> Self {
        Self {
            inner: Arc::new(AppStateInner { db, config }),
        }
    }

    /// Access the database pool.
    #[must_use]
    pub fn db(&self) -> &DatabasePool {
        &self.inner.db
    }

    /// Access the application configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }
}
