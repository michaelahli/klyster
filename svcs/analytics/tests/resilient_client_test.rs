//! Integration tests for `ResilientClient` against an in-process mock server.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use analytics::proto::analytics_service_server::{AnalyticsService, AnalyticsServiceServer};
use analytics::proto::{
    Empty, ForecastMetadata, ForecastPoint, ForecastRequest, ForecastResponse, FunctionCode,
    FunctionList, HealthState, HealthStatus, ValidationResult,
};
use analytics::{
    AnalyticsClient, AnalyticsEndpoint, CircuitBreakerConfig, CircuitState, ClientConfig,
    ResilientClient, ResilientConfig, RetryConfig,
};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

/// Mock service whose forecast handler fails N times before succeeding.
#[derive(Debug)]
struct FlakyService {
    fail_count: u32,
    invocations: Arc<AtomicU32>,
}

#[tonic::async_trait]
impl AnalyticsService for FlakyService {
    async fn run_forecast(
        &self,
        _request: Request<ForecastRequest>,
    ) -> Result<Response<ForecastResponse>, Status> {
        let n = self.invocations.fetch_add(1, Ordering::SeqCst) + 1;
        if n <= self.fail_count {
            return Err(Status::unavailable("transient"));
        }
        Ok(Response::new(ForecastResponse {
            points: vec![ForecastPoint {
                timestamp: 0,
                predicted_value: 1.0,
                lower_bound: 0.0,
                upper_bound: 2.0,
            }],
            metadata: Some(ForecastMetadata {
                function_name: "linear_regression".to_string(),
                execution_time_ms: 1,
                parameters: String::new(),
                quality_metrics: HashMap::new(),
            }),
        }))
    }

    async fn validate_function(
        &self,
        _request: Request<FunctionCode>,
    ) -> Result<Response<ValidationResult>, Status> {
        Ok(Response::new(ValidationResult {
            valid: true,
            error_message: None,
            warnings: Vec::new(),
        }))
    }

    async fn list_predefined_functions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<FunctionList>, Status> {
        Ok(Response::new(FunctionList::default()))
    }

    async fn health_check(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<HealthStatus>, Status> {
        Ok(Response::new(HealthStatus {
            status: HealthState::Healthy as i32,
            python_version: "3.11.5".to_string(),
            packages: HashMap::new(),
            message: None,
        }))
    }
}

/// Always-failing mock to exercise the circuit breaker.
#[derive(Debug, Default)]
struct AlwaysFailService {
    invocations: Arc<AtomicU32>,
}

#[tonic::async_trait]
impl AnalyticsService for AlwaysFailService {
    async fn run_forecast(
        &self,
        _request: Request<ForecastRequest>,
    ) -> Result<Response<ForecastResponse>, Status> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Err(Status::unavailable("nope"))
    }

    async fn validate_function(
        &self,
        _request: Request<FunctionCode>,
    ) -> Result<Response<ValidationResult>, Status> {
        Err(Status::unimplemented(""))
    }

    async fn list_predefined_functions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<FunctionList>, Status> {
        Err(Status::unimplemented(""))
    }

    async fn health_check(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<HealthStatus>, Status> {
        Err(Status::unimplemented(""))
    }
}

struct TestServer {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn start_with<S>(service: S) -> Self
    where
        S: AnalyticsService + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

        let (tx, rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            Server::builder()
                .add_service(AnalyticsServiceServer::new(service))
                .serve_with_incoming_shutdown(incoming, async {
                    let _ = rx.await;
                })
                .await
                .unwrap();
        });

        Self {
            addr,
            shutdown: Some(tx),
            handle,
        }
    }

    fn endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }

    async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.handle.await;
    }
}

fn client_config(endpoint: &str) -> ClientConfig {
    ClientConfig {
        endpoint: AnalyticsEndpoint::parse(endpoint).unwrap(),
        forecast_timeout: Duration::from_secs(2),
        health_timeout: Duration::from_secs(1),
        connect_timeout: Duration::from_millis(500),
        max_retries: 3,
        initial_backoff: Duration::from_millis(20),
    }
}

fn resilient_config(retries: u32) -> ResilientConfig {
    ResilientConfig {
        retry: RetryConfig {
            max_attempts: retries,
            initial_backoff: Duration::from_millis(5),
            max_backoff: Duration::from_millis(50),
            backoff_factor: 2.0,
        },
        circuit: CircuitBreakerConfig {
            failure_threshold: 3,
            cooldown: Duration::from_millis(80),
        },
    }
}

fn forecast_request() -> ForecastRequest {
    ForecastRequest {
        function_name: "linear_regression".to_string(),
        data: Vec::new(),
        horizon: 1,
        parameters: String::new(),
        custom_code: None,
    }
}

#[tokio::test]
async fn retries_transient_failures_until_success() {
    let invocations = Arc::new(AtomicU32::new(0));
    let server = TestServer::start_with(FlakyService {
        fail_count: 2,
        invocations: invocations.clone(),
    })
    .await;

    let client = AnalyticsClient::connect(client_config(&server.endpoint()))
        .await
        .unwrap();
    let resilient = ResilientClient::new(client, resilient_config(5));

    let response = resilient.run_forecast(forecast_request()).await.unwrap();
    assert_eq!(response.points.len(), 1);
    assert_eq!(invocations.load(Ordering::SeqCst), 3);
    assert_eq!(resilient.circuit_state(), CircuitState::Closed);

    server.shutdown().await;
}

#[tokio::test]
async fn gives_up_after_max_attempts() {
    let invocations = Arc::new(AtomicU32::new(0));
    let server = TestServer::start_with(AlwaysFailService {
        invocations: invocations.clone(),
    })
    .await;

    let client = AnalyticsClient::connect(client_config(&server.endpoint()))
        .await
        .unwrap();
    let resilient = ResilientClient::new(client, resilient_config(2));

    let err = resilient
        .run_forecast(forecast_request())
        .await
        .unwrap_err();
    match err {
        analytics::AnalyticsError::Rpc(status) => {
            assert_eq!(status.code(), tonic::Code::Unavailable);
        }
        other => panic!("expected Rpc(Unavailable), got {other:?}"),
    }
    assert_eq!(invocations.load(Ordering::SeqCst), 2);

    server.shutdown().await;
}

#[tokio::test]
async fn opens_circuit_after_threshold_failures() {
    let invocations = Arc::new(AtomicU32::new(0));
    let server = TestServer::start_with(AlwaysFailService {
        invocations: invocations.clone(),
    })
    .await;

    let client = AnalyticsClient::connect(client_config(&server.endpoint()))
        .await
        .unwrap();
    // 1 attempt per call so each failure increments the breaker once.
    let resilient = ResilientClient::new(client, resilient_config(1));

    for _ in 0..3 {
        let _ = resilient.run_forecast(forecast_request()).await;
    }

    assert_eq!(resilient.circuit_state(), CircuitState::Open);
    let err = resilient
        .run_forecast(forecast_request())
        .await
        .unwrap_err();
    assert!(matches!(err, analytics::AnalyticsError::CircuitOpen { .. }));

    server.shutdown().await;
}

#[tokio::test]
async fn does_not_retry_non_transient_errors() {
    #[derive(Debug, Default)]
    struct InvalidArgService {
        invocations: Arc<AtomicU32>,
    }

    #[tonic::async_trait]
    impl AnalyticsService for InvalidArgService {
        async fn run_forecast(
            &self,
            _request: Request<ForecastRequest>,
        ) -> Result<Response<ForecastResponse>, Status> {
            self.invocations.fetch_add(1, Ordering::SeqCst);
            Err(Status::invalid_argument("bad input"))
        }
        async fn validate_function(
            &self,
            _request: Request<FunctionCode>,
        ) -> Result<Response<ValidationResult>, Status> {
            unimplemented!()
        }
        async fn list_predefined_functions(
            &self,
            _request: Request<Empty>,
        ) -> Result<Response<FunctionList>, Status> {
            unimplemented!()
        }
        async fn health_check(
            &self,
            _request: Request<Empty>,
        ) -> Result<Response<HealthStatus>, Status> {
            unimplemented!()
        }
    }

    let invocations = Arc::new(AtomicU32::new(0));
    let server = TestServer::start_with(InvalidArgService {
        invocations: invocations.clone(),
    })
    .await;

    let client = AnalyticsClient::connect(client_config(&server.endpoint()))
        .await
        .unwrap();
    let resilient = ResilientClient::new(client, resilient_config(5));

    let _ = resilient
        .run_forecast(forecast_request())
        .await
        .unwrap_err();
    assert_eq!(invocations.load(Ordering::SeqCst), 1);

    server.shutdown().await;
}
