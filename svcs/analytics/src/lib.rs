//! Analytics orchestration and Python sidecar IPC for Klyster.

pub mod client;
pub mod error;
pub mod forecast_handler;
pub mod process;
pub mod runtime;
pub mod serialization;
pub mod supervisor;

/// Generated protobuf code for analytics service.
#[allow(clippy::pedantic, missing_docs)]
pub mod proto {
    tonic::include_proto!("analytics");
}

pub use client::{AnalyticsClient, AnalyticsEndpoint, ClientConfig};
pub use error::AnalyticsError;
pub use forecast_handler::{
    persist as persist_forecast, ForecastContext, ForecastHandlerError, PersistedForecast,
};
pub use process::{ProcessConfig, SidecarProcess};
pub use runtime::{PythonRuntime, RuntimeError};
pub use serialization::{
    data_points_to_metrics, metric_to_data_point, metrics_to_data_points, SerializationError,
};
pub use supervisor::{Supervisor, SupervisorConfig, SupervisorError, SupervisorEvent};
