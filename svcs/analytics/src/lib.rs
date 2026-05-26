//! Analytics orchestration and Python sidecar IPC for Klyster.

pub mod runtime;

/// Generated protobuf code for analytics service.
pub mod proto {
    tonic::include_proto!("analytics");
}

pub use runtime::{PythonRuntime, RuntimeError};
