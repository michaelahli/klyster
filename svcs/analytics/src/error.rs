//! Error types for analytics client and orchestration.

use std::time::Duration;

use thiserror::Error;
use tonic::Code;

/// Errors returned by the analytics client.
#[derive(Error, Debug)]
#[allow(clippy::result_large_err)]
pub enum AnalyticsError {
    /// Endpoint string is malformed or unsupported.
    #[error("invalid endpoint: {0}")]
    InvalidEndpoint(String),

    /// Underlying transport (HTTP/2, Unix socket) failed.
    #[error("transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    /// Remote service returned a gRPC error status.
    #[error("rpc failed: {0}")]
    Rpc(#[from] tonic::Status),

    /// Failed to establish a connection within the configured retry budget.
    #[error("failed to connect after {attempts} attempts: {source}")]
    ConnectionFailed {
        /// Number of attempts performed before giving up.
        attempts: u32,
        /// Last transport error encountered.
        #[source]
        source: tonic::transport::Error,
    },

    /// I/O error (typically while binding or reading a Unix socket).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Operation timed out client-side. The wrapped status code may be
    /// `DeadlineExceeded` (server cancelled) or `Cancelled` (client cancelled).
    #[error("{operation} timed out after {elapsed:?}")]
    Timeout {
        /// Logical name of the call (e.g. `run_forecast`).
        operation: &'static str,
        /// Duration the client waited before timing out.
        elapsed: Duration,
    },

    /// Circuit breaker is open and rejected the call.
    #[error("circuit breaker open; retry after {retry_after:?}")]
    CircuitOpen {
        /// Hint for the caller about when the next probe is permitted.
        retry_after: Duration,
    },
}

impl AnalyticsError {
    /// Return a stable, low-cardinality label for this error suitable for use
    /// as a Prometheus metric label.
    #[must_use]
    pub fn metric_label(&self) -> &'static str {
        match self {
            AnalyticsError::InvalidEndpoint(_) => "invalid_endpoint",
            AnalyticsError::Transport(_) => "transport",
            AnalyticsError::Rpc(status) => crate::client::code_label(status.code()),
            AnalyticsError::ConnectionFailed { .. } => "connection_failed",
            AnalyticsError::Io(_) => "io",
            AnalyticsError::Timeout { .. } => "timeout",
            AnalyticsError::CircuitOpen { .. } => "circuit_open",
        }
    }

    /// Whether the error is worth retrying. Used by `ResilientClient`.
    #[must_use]
    pub fn is_transient(&self) -> bool {
        match self {
            AnalyticsError::Transport(_)
            | AnalyticsError::ConnectionFailed { .. }
            | AnalyticsError::Io(_)
            | AnalyticsError::Timeout { .. } => true,
            AnalyticsError::Rpc(status) => matches!(
                status.code(),
                Code::Unavailable
                    | Code::DeadlineExceeded
                    | Code::ResourceExhausted
                    | Code::Aborted
                    | Code::Internal
            ),
            AnalyticsError::InvalidEndpoint(_) | AnalyticsError::CircuitOpen { .. } => false,
        }
    }
}
