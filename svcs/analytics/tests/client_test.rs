//! Integration tests for `AnalyticsClient` against an in-process mock server.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use analytics::proto::analytics_service_server::{AnalyticsService, AnalyticsServiceServer};
use analytics::proto::{
    Empty, ForecastMetadata, ForecastPoint, ForecastRequest, ForecastResponse, FunctionCode,
    FunctionInfo, FunctionList, FunctionType, HealthState, HealthStatus, ValidationResult,
};
use analytics::{AnalyticsClient, AnalyticsEndpoint, ClientConfig};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Default)]
struct MockService;

#[tonic::async_trait]
impl AnalyticsService for MockService {
    async fn run_forecast(
        &self,
        request: Request<ForecastRequest>,
    ) -> Result<Response<ForecastResponse>, Status> {
        let req = request.into_inner();
        if req.function_name == "fail" {
            return Err(Status::invalid_argument("unknown function"));
        }
        let points = (0..req.horizon)
            .map(|i| ForecastPoint {
                timestamp: i64::from(i),
                predicted_value: f64::from(i),
                lower_bound: 0.0,
                upper_bound: f64::from(i) + 1.0,
            })
            .collect();
        Ok(Response::new(ForecastResponse {
            points,
            metadata: Some(ForecastMetadata {
                function_name: req.function_name,
                execution_time_ms: 5,
                parameters: req.parameters,
                quality_metrics: HashMap::new(),
            }),
        }))
    }

    async fn validate_function(
        &self,
        request: Request<FunctionCode>,
    ) -> Result<Response<ValidationResult>, Status> {
        let code = request.into_inner();
        Ok(Response::new(ValidationResult {
            valid: !code.code.is_empty(),
            error_message: if code.code.is_empty() {
                Some("empty code".to_string())
            } else {
                None
            },
            warnings: Vec::new(),
        }))
    }

    async fn list_predefined_functions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<FunctionList>, Status> {
        Ok(Response::new(FunctionList {
            functions: vec![FunctionInfo {
                name: "linear_regression".to_string(),
                r#type: FunctionType::Predefined as i32,
                description: "Linear regression forecast".to_string(),
                parameters_schema: "{}".to_string(),
            }],
        }))
    }

    async fn health_check(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<HealthStatus>, Status> {
        Ok(Response::new(HealthStatus {
            status: HealthState::Healthy as i32,
            python_version: "3.11.5".to_string(),
            packages: HashMap::new(),
            message: Some("ok".to_string()),
        }))
    }
}

struct TestServer {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

        let (tx, rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            Server::builder()
                .add_service(AnalyticsServiceServer::new(MockService))
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

fn config_for(endpoint: &str) -> ClientConfig {
    ClientConfig {
        endpoint: AnalyticsEndpoint::parse(endpoint).unwrap(),
        forecast_timeout: Duration::from_secs(2),
        health_timeout: Duration::from_secs(1),
        connect_timeout: Duration::from_millis(500),
        max_retries: 3,
        initial_backoff: Duration::from_millis(20),
    }
}

#[tokio::test]
async fn health_check_returns_status() {
    let server = TestServer::start().await;
    let client = AnalyticsClient::connect(config_for(&server.endpoint()))
        .await
        .unwrap();

    let status = client.health_check().await.unwrap();
    assert_eq!(status.status, HealthState::Healthy as i32);
    assert_eq!(status.python_version, "3.11.5");

    server.shutdown().await;
}

#[tokio::test]
async fn run_forecast_returns_points() {
    let server = TestServer::start().await;
    let client = AnalyticsClient::connect(config_for(&server.endpoint()))
        .await
        .unwrap();

    let response = client
        .run_forecast(ForecastRequest {
            function_name: "linear_regression".to_string(),
            data: Vec::new(),
            horizon: 4,
            parameters: "{}".to_string(),
            custom_code: None,
        })
        .await
        .unwrap();

    assert_eq!(response.points.len(), 4);
    let metadata = response.metadata.expect("metadata present");
    assert_eq!(metadata.function_name, "linear_regression");

    server.shutdown().await;
}

#[tokio::test]
async fn run_forecast_propagates_rpc_error() {
    let server = TestServer::start().await;
    let client = AnalyticsClient::connect(config_for(&server.endpoint()))
        .await
        .unwrap();

    let err = client
        .run_forecast(ForecastRequest {
            function_name: "fail".to_string(),
            data: Vec::new(),
            horizon: 1,
            parameters: String::new(),
            custom_code: None,
        })
        .await
        .unwrap_err();

    match err {
        analytics::AnalyticsError::Rpc(status) => {
            assert_eq!(status.code(), tonic::Code::InvalidArgument);
        }
        other => panic!("expected Rpc error, got {other:?}"),
    }

    server.shutdown().await;
}

#[tokio::test]
async fn validate_function_flags_empty_code() {
    let server = TestServer::start().await;
    let client = AnalyticsClient::connect(config_for(&server.endpoint()))
        .await
        .unwrap();

    let result = client
        .validate_function(FunctionCode {
            name: "noop".to_string(),
            code: String::new(),
        })
        .await
        .unwrap();
    assert!(!result.valid);

    server.shutdown().await;
}

#[tokio::test]
async fn list_predefined_functions_returns_catalog() {
    let server = TestServer::start().await;
    let client = AnalyticsClient::connect(config_for(&server.endpoint()))
        .await
        .unwrap();

    let list = client.list_predefined_functions().await.unwrap();
    assert_eq!(list.functions.len(), 1);
    assert_eq!(list.functions[0].name, "linear_regression");

    server.shutdown().await;
}

#[tokio::test]
async fn connect_fails_after_retry_budget_exhausted() {
    // Bind a port, drop the listener so the OS likely won't reuse it immediately.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let endpoint = format!("http://{addr}");
    let mut config = config_for(&endpoint);
    config.max_retries = 2;
    config.initial_backoff = Duration::from_millis(10);

    let err = AnalyticsClient::connect(config).await.unwrap_err();
    match err {
        analytics::AnalyticsError::ConnectionFailed { attempts, .. } => {
            assert_eq!(attempts, 2);
        }
        other => panic!("expected ConnectionFailed, got {other:?}"),
    }
}
