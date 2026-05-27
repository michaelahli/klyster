//! End-to-end integration tests against the live Seer sidecar.
//!
//! These spawn the actual Python process (`python -m seer`) via [`Supervisor`]
//! and run the Rust client against it. They require:
//!
//! * A Python interpreter with the seer requirements installed.
//! * `PYTHONPATH` pointing at the `svcs/` directory so `import seer` works.
//!
//! Both are opt-in via the `KLYSTER_E2E_PYTHON` and `KLYSTER_E2E_PYTHONPATH`
//! environment variables. When unset, the tests are skipped via `eprintln!`
//! and an early return so CI without Python still builds and tests cleanly.
//! Run locally with:
//!
//! ```bash
//! KLYSTER_E2E_PYTHON=svcs/seer/venv/bin/python \
//! KLYSTER_E2E_PYTHONPATH=svcs \
//!     cargo test -p analytics --test end_to_end_test -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::time::Duration;

use analytics::proto::{ForecastRequest, MetricDataPoint};
use analytics::{
    AnalyticsClient, AnalyticsEndpoint, ClientConfig, ProcessConfig, ResilientClient,
    ResilientConfig, RetryConfig, Supervisor, SupervisorConfig, SupervisorEvent,
};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

/// Read the opt-in environment knobs; returns `None` if either is missing.
fn opt_in() -> Option<(PathBuf, PathBuf)> {
    let python = std::env::var("KLYSTER_E2E_PYTHON").ok()?;
    let path = std::env::var("KLYSTER_E2E_PYTHONPATH").ok()?;
    Some((PathBuf::from(python), PathBuf::from(path)))
}

/// Reserve an ephemeral port by binding and immediately dropping the listener.
async fn pick_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn supervisor_config(python: PathBuf, pythonpath: PathBuf, port: u16) -> SupervisorConfig {
    let process = ProcessConfig {
        python_executable: python,
        module: "seer".to_string(),
        python_path: Some(pythonpath),
        host: "127.0.0.1".to_string(),
        port,
        socket: None,
        log_level: "info".to_string(),
        graceful_shutdown: Duration::from_secs(2),
    };
    let endpoint = AnalyticsEndpoint::parse(&format!("http://127.0.0.1:{port}")).unwrap();
    let client = ClientConfig {
        endpoint,
        forecast_timeout: Duration::from_secs(15),
        health_timeout: Duration::from_secs(5),
        connect_timeout: Duration::from_secs(5),
        max_retries: 20,
        initial_backoff: Duration::from_millis(200),
    };
    SupervisorConfig {
        process,
        client,
        health_interval: Duration::from_millis(500),
        restart_window: Duration::from_secs(30),
        max_restarts: 2,
        startup_grace: Duration::from_secs(3),
    }
}

async fn wait_for_started(rx: &mut mpsc::Receiver<SupervisorEvent>) -> u32 {
    while let Some(event) = rx.recv().await {
        if let SupervisorEvent::Started { pid } = event {
            return pid;
        }
    }
    panic!("supervisor closed before emitting Started");
}

fn linear_data(n: usize) -> Vec<MetricDataPoint> {
    (0..n)
        .map(|i| MetricDataPoint {
            #[allow(clippy::cast_possible_wrap)]
            timestamp: i as i64,
            #[allow(clippy::cast_precision_loss)]
            value: (i as f64) * 2.0 + 1.0,
            labels: std::collections::HashMap::new(),
        })
        .collect()
}

#[tokio::test]
#[ignore = "requires KLYSTER_E2E_PYTHON + KLYSTER_E2E_PYTHONPATH; spawns real Seer sidecar"]
async fn end_to_end_linear_regression_through_supervisor() {
    let Some((python, pythonpath)) = opt_in() else {
        eprintln!("skipping: KLYSTER_E2E_PYTHON / KLYSTER_E2E_PYTHONPATH not set");
        return;
    };

    let port = pick_port().await;
    let (events_tx, mut events_rx) = mpsc::channel(16);
    let supervisor =
        Supervisor::new(supervisor_config(python, pythonpath, port)).with_events(events_tx);
    let handle = supervisor.start();

    let _pid = wait_for_started(&mut events_rx).await;

    // Allow the gRPC server to bind before we connect.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let endpoint = AnalyticsEndpoint::parse(&format!("http://127.0.0.1:{port}")).unwrap();
    let client_config = ClientConfig {
        endpoint,
        forecast_timeout: Duration::from_secs(10),
        health_timeout: Duration::from_secs(5),
        connect_timeout: Duration::from_secs(5),
        max_retries: 5,
        initial_backoff: Duration::from_millis(200),
    };
    let client = AnalyticsClient::connect(client_config).await.unwrap();
    let resilient = ResilientClient::new(client, ResilientConfig::default());

    let response = resilient
        .run_forecast(ForecastRequest {
            function_name: "linear_regression".to_string(),
            data: linear_data(20),
            horizon: 5,
            parameters: "{}".to_string(),
            custom_code: None,
        })
        .await
        .unwrap();
    assert_eq!(response.points.len(), 5);
    let metadata = response.metadata.expect("metadata present");
    assert_eq!(metadata.function_name, "linear_regression");

    handle.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore = "requires KLYSTER_E2E_PYTHON + KLYSTER_E2E_PYTHONPATH; spawns real Seer sidecar"]
async fn end_to_end_health_check_with_retry_config() {
    let Some((python, pythonpath)) = opt_in() else {
        eprintln!("skipping: KLYSTER_E2E_PYTHON / KLYSTER_E2E_PYTHONPATH not set");
        return;
    };

    let port = pick_port().await;
    let (events_tx, mut events_rx) = mpsc::channel(16);
    let supervisor =
        Supervisor::new(supervisor_config(python, pythonpath, port)).with_events(events_tx);
    let handle = supervisor.start();
    let _pid = wait_for_started(&mut events_rx).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let endpoint = AnalyticsEndpoint::parse(&format!("http://127.0.0.1:{port}")).unwrap();
    let client = AnalyticsClient::connect(ClientConfig {
        endpoint,
        forecast_timeout: Duration::from_secs(10),
        health_timeout: Duration::from_secs(5),
        connect_timeout: Duration::from_secs(5),
        max_retries: 5,
        initial_backoff: Duration::from_millis(200),
    })
    .await
    .unwrap();
    let resilient = ResilientClient::new(
        client,
        ResilientConfig {
            retry: RetryConfig {
                max_attempts: 3,
                initial_backoff: Duration::from_millis(50),
                max_backoff: Duration::from_secs(1),
                backoff_factor: 2.0,
            },
            ..ResilientConfig::default()
        },
    );

    let status = resilient.health_check().await.unwrap();
    assert_eq!(status.status, analytics::proto::HealthState::Healthy as i32);
    assert!(!status.python_version.is_empty());
    for pkg in ["numpy", "pandas", "scikit-learn", "statsmodels"] {
        assert!(
            status.packages.contains_key(pkg),
            "expected package {pkg} in health response, got {:?}",
            status.packages.keys().collect::<Vec<_>>(),
        );
    }

    handle.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore = "requires KLYSTER_E2E_PYTHON + KLYSTER_E2E_PYTHONPATH; spawns real Seer sidecar"]
async fn end_to_end_validate_custom_function() {
    let Some((python, pythonpath)) = opt_in() else {
        eprintln!("skipping: KLYSTER_E2E_PYTHON / KLYSTER_E2E_PYTHONPATH not set");
        return;
    };

    let port = pick_port().await;
    let (events_tx, mut events_rx) = mpsc::channel(16);
    let supervisor =
        Supervisor::new(supervisor_config(python, pythonpath, port)).with_events(events_tx);
    let handle = supervisor.start();
    let _pid = wait_for_started(&mut events_rx).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let client = AnalyticsClient::connect(ClientConfig {
        endpoint: AnalyticsEndpoint::parse(&format!("http://127.0.0.1:{port}")).unwrap(),
        forecast_timeout: Duration::from_secs(10),
        health_timeout: Duration::from_secs(5),
        connect_timeout: Duration::from_secs(5),
        max_retries: 5,
        initial_backoff: Duration::from_millis(200),
    })
    .await
    .unwrap();

    let safe = client
        .validate_function(analytics::proto::FunctionCode {
            name: "passthrough".to_string(),
            code: "def forecast(data, horizon, params):\n    return []\n".to_string(),
        })
        .await
        .unwrap();
    assert!(safe.valid);

    let bad = client
        .validate_function(analytics::proto::FunctionCode {
            name: "danger".to_string(),
            code: "import os\ndef forecast(data, horizon, params):\n    return []\n".to_string(),
        })
        .await
        .unwrap();
    assert!(!bad.valid);

    handle.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore = "requires KLYSTER_E2E_PYTHON + KLYSTER_E2E_PYTHONPATH; spawns real Seer sidecar"]
async fn end_to_end_large_dataset() {
    let Some((python, pythonpath)) = opt_in() else {
        eprintln!("skipping: KLYSTER_E2E_PYTHON / KLYSTER_E2E_PYTHONPATH not set");
        return;
    };

    let port = pick_port().await;
    let (events_tx, mut events_rx) = mpsc::channel(16);
    let supervisor =
        Supervisor::new(supervisor_config(python, pythonpath, port)).with_events(events_tx);
    let handle = supervisor.start();
    let _pid = wait_for_started(&mut events_rx).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let client = AnalyticsClient::connect(ClientConfig {
        endpoint: AnalyticsEndpoint::parse(&format!("http://127.0.0.1:{port}")).unwrap(),
        forecast_timeout: Duration::from_secs(30),
        health_timeout: Duration::from_secs(5),
        connect_timeout: Duration::from_secs(5),
        max_retries: 5,
        initial_backoff: Duration::from_millis(200),
    })
    .await
    .unwrap();

    let started = std::time::Instant::now();
    let response = client
        .run_forecast(ForecastRequest {
            function_name: "linear_regression".to_string(),
            data: linear_data(10_000),
            horizon: 10,
            parameters: "{}".to_string(),
            custom_code: None,
        })
        .await
        .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(response.points.len(), 10);
    assert!(
        elapsed < Duration::from_secs(10),
        "10k-point forecast took {elapsed:?}",
    );

    handle.shutdown().await.unwrap();
}
