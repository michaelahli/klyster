//! HTTP API server for Klyster.

/// HTTP server lifecycle (binding, serving, graceful shutdown).
pub mod server;
/// Shared application state used by request handlers.
pub mod state;

pub use server::{bind, build_router, run, ServerError};
pub use state::AppState;
