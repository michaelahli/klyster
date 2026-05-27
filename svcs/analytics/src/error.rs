//! Error types for analytics client and orchestration.

use thiserror::Error;

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
}
