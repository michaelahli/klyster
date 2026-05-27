//! gRPC client for the Seer (Python analytics) sidecar.

// `AnalyticsError` carries `tonic::transport::Error`, which is intrinsically large.
#![allow(clippy::result_large_err)]

use std::path::PathBuf;
use std::time::Duration;

use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint};
use tonic::{Code, Request};
use tower::service_fn;
use tracing::{debug, warn};

use crate::error::AnalyticsError;
use crate::proto::analytics_service_client::AnalyticsServiceClient;
use crate::proto::{
    Empty, ForecastRequest, ForecastResponse, FunctionCode, FunctionList, HealthStatus,
    ValidationResult,
};

const DEFAULT_TCP_ENDPOINT: &str = "http://127.0.0.1:50051";

/// Endpoint for the analytics sidecar.
#[derive(Debug, Clone)]
pub enum AnalyticsEndpoint {
    /// TCP endpoint URI (e.g. `http://127.0.0.1:50051`).
    Tcp(String),
    /// Path to a Unix domain socket.
    Unix(PathBuf),
}

impl AnalyticsEndpoint {
    /// Parse an endpoint string.
    ///
    /// Accepts `http://host:port`, `https://host:port`, or `unix:///absolute/path`.
    pub fn parse(input: &str) -> Result<Self, AnalyticsError> {
        if let Some(path) = input.strip_prefix("unix://") {
            if path.is_empty() {
                return Err(AnalyticsError::InvalidEndpoint(input.to_string()));
            }
            Ok(Self::Unix(PathBuf::from(path)))
        } else if input.starts_with("http://") || input.starts_with("https://") {
            Ok(Self::Tcp(input.to_string()))
        } else {
            Err(AnalyticsError::InvalidEndpoint(input.to_string()))
        }
    }
}

impl Default for AnalyticsEndpoint {
    fn default() -> Self {
        Self::Tcp(DEFAULT_TCP_ENDPOINT.to_string())
    }
}

/// Configuration for the analytics client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Endpoint to connect to.
    pub endpoint: AnalyticsEndpoint,
    /// Timeout applied to forecast RPC calls.
    pub forecast_timeout: Duration,
    /// Timeout applied to health check RPC calls.
    pub health_timeout: Duration,
    /// Per-attempt connection timeout.
    pub connect_timeout: Duration,
    /// Maximum number of connection attempts.
    pub max_retries: u32,
    /// Initial backoff between connection retries.
    pub initial_backoff: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            endpoint: AnalyticsEndpoint::default(),
            forecast_timeout: Duration::from_secs(30),
            health_timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(5),
            max_retries: 5,
            initial_backoff: Duration::from_millis(200),
        }
    }
}

/// gRPC client for the Seer analytics sidecar.
#[derive(Debug, Clone)]
pub struct AnalyticsClient {
    inner: AnalyticsServiceClient<Channel>,
    config: ClientConfig,
}

impl AnalyticsClient {
    /// Connect to the analytics service using the supplied configuration.
    ///
    /// Retries with exponential backoff up to `config.max_retries` attempts.
    pub async fn connect(config: ClientConfig) -> Result<Self, AnalyticsError> {
        let channel = connect_with_retry(&config).await?;
        Ok(Self {
            inner: AnalyticsServiceClient::new(channel),
            config,
        })
    }

    /// Run a forecast on the analytics sidecar.
    pub async fn run_forecast(
        &self,
        request: ForecastRequest,
    ) -> Result<ForecastResponse, AnalyticsError> {
        let mut req = Request::new(request);
        req.set_timeout(self.config.forecast_timeout);
        let response = self.inner.clone().run_forecast(req).await?;
        Ok(response.into_inner())
    }

    /// Validate user-provided function code.
    pub async fn validate_function(
        &self,
        request: FunctionCode,
    ) -> Result<ValidationResult, AnalyticsError> {
        let mut req = Request::new(request);
        req.set_timeout(self.config.forecast_timeout);
        let response = self.inner.clone().validate_function(req).await?;
        Ok(response.into_inner())
    }

    /// List the predefined forecasting functions exposed by the sidecar.
    pub async fn list_predefined_functions(&self) -> Result<FunctionList, AnalyticsError> {
        let mut req = Request::new(Empty {});
        req.set_timeout(self.config.forecast_timeout);
        let response = self.inner.clone().list_predefined_functions(req).await?;
        Ok(response.into_inner())
    }

    /// Probe the health of the analytics sidecar.
    pub async fn health_check(&self) -> Result<HealthStatus, AnalyticsError> {
        let mut req = Request::new(Empty {});
        req.set_timeout(self.config.health_timeout);
        let response = self.inner.clone().health_check(req).await?;
        Ok(response.into_inner())
    }
}

async fn connect_with_retry(config: &ClientConfig) -> Result<Channel, AnalyticsError> {
    let max_attempts = config.max_retries.max(1);
    let mut backoff = config.initial_backoff;
    let mut last_err: Option<tonic::transport::Error> = None;

    for attempt in 1..=max_attempts {
        match connect_once(config).await {
            Ok(channel) => {
                debug!(attempt, "connected to analytics sidecar");
                return Ok(channel);
            }
            Err(err) => {
                warn!(attempt, error = %err, "failed to connect to analytics sidecar");
                last_err = Some(err);
                if attempt < max_attempts {
                    tokio::time::sleep(backoff).await;
                    backoff = backoff.saturating_mul(2);
                }
            }
        }
    }

    Err(AnalyticsError::ConnectionFailed {
        attempts: max_attempts,
        source: last_err.expect("retry loop runs at least once"),
    })
}

async fn connect_once(config: &ClientConfig) -> Result<Channel, tonic::transport::Error> {
    match &config.endpoint {
        AnalyticsEndpoint::Tcp(uri) => {
            Endpoint::from_shared(uri.clone())
                .expect("validated by AnalyticsEndpoint::parse")
                .connect_timeout(config.connect_timeout)
                .connect()
                .await
        }
        AnalyticsEndpoint::Unix(path) => {
            let path = path.clone();
            Endpoint::try_from("http://[::]:50051")
                .expect("static URI is valid")
                .connect_timeout(config.connect_timeout)
                .connect_with_connector(service_fn(move |_: tonic::transport::Uri| {
                    let path = path.clone();
                    async move {
                        let stream = UnixStream::connect(path).await?;
                        Ok::<_, std::io::Error>(TokioIo::new(stream))
                    }
                }))
                .await
        }
    }
}

/// Map a gRPC `Code` to a stable string label, useful for metrics.
#[must_use]
pub fn code_label(code: Code) -> &'static str {
    match code {
        Code::Ok => "ok",
        Code::Cancelled => "cancelled",
        Code::Unknown => "unknown",
        Code::InvalidArgument => "invalid_argument",
        Code::DeadlineExceeded => "deadline_exceeded",
        Code::NotFound => "not_found",
        Code::AlreadyExists => "already_exists",
        Code::PermissionDenied => "permission_denied",
        Code::ResourceExhausted => "resource_exhausted",
        Code::FailedPrecondition => "failed_precondition",
        Code::Aborted => "aborted",
        Code::OutOfRange => "out_of_range",
        Code::Unimplemented => "unimplemented",
        Code::Internal => "internal",
        Code::Unavailable => "unavailable",
        Code::DataLoss => "data_loss",
        Code::Unauthenticated => "unauthenticated",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tcp_endpoint() {
        let endpoint = AnalyticsEndpoint::parse("http://127.0.0.1:50051").unwrap();
        assert!(matches!(endpoint, AnalyticsEndpoint::Tcp(uri) if uri == "http://127.0.0.1:50051"));
    }

    #[test]
    fn parse_unix_endpoint() {
        let endpoint = AnalyticsEndpoint::parse("unix:///tmp/seer.sock").unwrap();
        match endpoint {
            AnalyticsEndpoint::Unix(path) => {
                assert_eq!(path, PathBuf::from("/tmp/seer.sock"));
            }
            AnalyticsEndpoint::Tcp(_) => panic!("expected unix endpoint"),
        }
    }

    #[test]
    fn parse_rejects_unknown_scheme() {
        let err = AnalyticsEndpoint::parse("tcp://localhost:50051").unwrap_err();
        assert!(matches!(err, AnalyticsError::InvalidEndpoint(_)));
    }

    #[test]
    fn parse_rejects_empty_unix_path() {
        let err = AnalyticsEndpoint::parse("unix://").unwrap_err();
        assert!(matches!(err, AnalyticsError::InvalidEndpoint(_)));
    }
}
