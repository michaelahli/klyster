//! HTTP API server for Klyster.

/// HTTP server lifecycle (binding, serving, graceful shutdown).
pub mod server;
/// Shared application state used by request handlers.
pub mod state;
/// HTTP route handlers.
pub mod routes;
/// Error types and handling.
pub mod error;

pub use error::{ApiError, ApiResult};
pub use server::{bind, build_router, run, ServerError};
pub use state::AppState;
