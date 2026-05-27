//! Analytics orchestration and Python sidecar IPC for Klyster.

pub mod client;
pub mod error;
pub mod runtime;

/// Generated protobuf code for analytics service.
#[allow(clippy::pedantic, missing_docs)]
pub mod proto {
    tonic::include_proto!("analytics");
}

pub use client::{AnalyticsClient, AnalyticsEndpoint, ClientConfig};
pub use error::AnalyticsError;
pub use runtime::{PythonRuntime, RuntimeError};
