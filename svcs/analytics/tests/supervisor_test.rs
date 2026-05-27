//! Integration tests for the sidecar supervisor lifecycle.

use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use analytics::proto::analytics_service_server::{AnalyticsService, AnalyticsServiceServer};
use analytics::proto::{
    Empty, ForecastRequest, ForecastResponse, FunctionCode, FunctionList, HealthState,
    HealthStatus, ValidationResult,
};
use analytics::{
    AnalyticsEndpoint, ClientConfig, ProcessConfig, Supervisor, SupervisorConfig, SupervisorError,
    SupervisorEvent,
};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Default)]
struct HealthOnlyService;

#[tonic::async_trait]
impl AnalyticsService for HealthOnlyService {
    async fn run_forecast(
        &self,
        _request: Request<ForecastRequest>,
    ) -> Result<Response<ForecastResponse>, Status> {
        Err(Status::unimplemented("not needed"))
    }

    async fn validate_function(
        &self,
        _request: Request<FunctionCode>,
    ) -> Result<Response<ValidationResult>, Status> {
        Err(Status::unimplemented("not needed"))
    }

    async fn list_predefined_functions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<FunctionList>, Status> {
        Err(Status::unimplemented("not needed"))
    }

    async fn health_check(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<HealthStatus>, Status> {
        Ok(Response::new(HealthStatus {
            status: HealthState::Healthy as i32,
            python_version: "3.11.0".to_string(),
            packages: HashMap::new(),
            message: Some("ok".to_string()),
        }))
    }
}

struct MockGrpcServer {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

impl MockGrpcServer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let incoming = TcpListenerStream::new(listener);
        let (tx, rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            Server::builder()
                .add_service(AnalyticsServiceServer::new(HealthOnlyService))
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

fn python_executable() -> String {
    std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string())
}

fn write_module(dir: &Path, name: &str, body: &str) {
    fs::write(dir.join(format!("{name}.py")), body).unwrap();
}

fn process_config(module_dir: &Path, module: &str) -> ProcessConfig {
    ProcessConfig {
        python_executable: python_executable().into(),
        module: module.to_string(),
        python_path: Some(module_dir.to_path_buf()),
        host: "127.0.0.1".to_string(),
        port: 0,
        socket: None,
        log_level: "info".to_string(),
        graceful_shutdown: Duration::from_millis(500),
    }
}

fn supervisor_config(process: ProcessConfig, endpoint: &str) -> SupervisorConfig {
    SupervisorConfig {
        process,
        client: ClientConfig {
            endpoint: AnalyticsEndpoint::parse(endpoint).unwrap(),
            forecast_timeout: Duration::from_millis(500),
            health_timeout: Duration::from_millis(500),
            connect_timeout: Duration::from_millis(200),
            max_retries: 2,
            initial_backoff: Duration::from_millis(10),
        },
        health_interval: Duration::from_millis(100),
        restart_window: Duration::from_secs(60),
        max_restarts: 2,
        startup_grace: Duration::from_millis(50),
    }
}

#[tokio::test]
async fn supervisor_starts_and_stops_sidecar_gracefully() {
    let server = MockGrpcServer::start().await;
    let temp = TempDir::new().unwrap();
    write_module(
        temp.path(),
        "sleepy_sidecar",
        "
import signal
import time
import sys

running = True

def stop(_sig, _frame):
    global running
    running = False

signal.signal(signal.SIGTERM, stop)
print('sleepy started', flush=True)
while running:
    time.sleep(0.05)
sys.exit(0)
",
    );

    let (events_tx, mut events_rx) = mpsc::channel(8);
    let supervisor = Supervisor::new(supervisor_config(
        process_config(temp.path(), "sleepy_sidecar"),
        &server.endpoint(),
    ))
    .with_events(events_tx);
    let handle = supervisor.start();

    let started = tokio::time::timeout(Duration::from_secs(2), events_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(started, SupervisorEvent::Started { .. }));

    handle.shutdown().await.unwrap();
    let stopped = tokio::time::timeout(Duration::from_secs(2), events_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(stopped, SupervisorEvent::Stopped));

    server.shutdown().await;
}

#[tokio::test]
async fn supervisor_restarts_crashed_sidecar_then_fails_on_budget() {
    let temp = TempDir::new().unwrap();
    write_module(
        temp.path(),
        "crashy_sidecar",
        "import sys\nprint('crashy started', flush=True)\nsys.exit(1)\n",
    );

    let (events_tx, mut events_rx) = mpsc::channel(16);
    let config = SupervisorConfig {
        max_restarts: 1,
        ..supervisor_config(
            process_config(temp.path(), "crashy_sidecar"),
            "http://127.0.0.1:1",
        )
    };
    let handle = Supervisor::new(config).with_events(events_tx).start();

    let first = tokio::time::timeout(Duration::from_secs(2), events_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(first, SupervisorEvent::Started { .. }));

    let restarting = tokio::time::timeout(Duration::from_secs(2), events_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(restarting, SupervisorEvent::Restarting { .. }));

    let err = handle.shutdown().await.unwrap_err();
    assert!(matches!(err, SupervisorError::RestartBudgetExceeded { .. }));
}
