//! HTTP API server for Klyster.

/// OpenAPI documentation.
pub mod docs;
/// Data Transfer Objects.
pub mod dto;
/// Error types and handling.
pub mod error;
/// Custom extractors.
pub mod extractors;
/// Prometheus metrics.
pub mod metrics;
/// HTTP middleware.
pub mod middleware;
/// HTTP route handlers.
pub mod routes;
/// HTTP server lifecycle (binding, serving, graceful shutdown).
pub mod server;
/// Shared application state used by request handlers.
pub mod state;
/// Embedded UI bundle and SPA fallback.
pub mod static_files;

pub use error::{ApiError, ApiResult};
pub use server::{bind, build_router, run, ServerError};
pub use state::AppState;
