//! Spawn and manage the Seer (Python analytics) sidecar process.

// `AnalyticsError` carries `tonic::transport::Error`, which is intrinsically large.
#![allow(clippy::result_large_err)]

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, error, info, warn};

use crate::error::AnalyticsError;

/// Default name of the Python module to run as the sidecar.
const DEFAULT_MODULE: &str = "seer";

/// Configuration for spawning the sidecar process.
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Python executable to invoke.
    pub python_executable: PathBuf,
    /// Python module to run with `-m` (defaults to `seer`).
    pub module: String,
    /// `PYTHONPATH` directory; should point to a folder containing the module.
    pub python_path: Option<PathBuf>,
    /// Bind address: TCP host (e.g. `0.0.0.0`) or a Unix socket path via `socket`.
    pub host: String,
    /// TCP port (used when no socket path is set).
    pub port: u16,
    /// Optional Unix socket path (overrides host/port when set).
    pub socket: Option<PathBuf>,
    /// Log level forwarded to the sidecar (`debug`, `info`, ...).
    pub log_level: String,
    /// Time to wait for graceful shutdown after SIGTERM before SIGKILL.
    pub graceful_shutdown: Duration,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            python_executable: PathBuf::from("python3"),
            module: DEFAULT_MODULE.to_string(),
            python_path: None,
            host: "127.0.0.1".to_string(),
            port: 50051,
            socket: None,
            log_level: "info".to_string(),
            graceful_shutdown: Duration::from_secs(5),
        }
    }
}

impl ProcessConfig {
    fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.python_executable);
        cmd.arg("-m").arg(&self.module);

        if let Some(socket) = &self.socket {
            cmd.arg("--socket").arg(socket);
        } else {
            cmd.arg("--host").arg(&self.host);
            cmd.arg("--port").arg(self.port.to_string());
        }

        cmd.arg("--log-level").arg(&self.log_level);

        if let Some(path) = &self.python_path {
            cmd.env("PYTHONPATH", path);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd.kill_on_drop(true);
        cmd
    }
}

/// Handle to a running sidecar process.
#[derive(Debug)]
pub struct SidecarProcess {
    child: Child,
    pid: u32,
    stdout_handle: Option<JoinHandle<()>>,
    stderr_handle: Option<JoinHandle<()>>,
    graceful_shutdown: Duration,
}

impl SidecarProcess {
    /// Spawn the sidecar process and start log forwarding tasks.
    pub fn spawn(config: &ProcessConfig) -> Result<Self, AnalyticsError> {
        let mut command = config.build_command();
        let mut child = command.spawn()?;
        let pid = child
            .id()
            .ok_or_else(|| AnalyticsError::Io(std::io::Error::other("spawned child has no pid")))?;

        info!(pid, module = %config.module, "spawned analytics sidecar");

        let stdout_handle = child
            .stdout
            .take()
            .map(|stdout| tokio::spawn(forward_lines(stdout, pid, false)));
        let stderr_handle = child
            .stderr
            .take()
            .map(|stderr| tokio::spawn(forward_lines(stderr, pid, true)));

        Ok(Self {
            child,
            pid,
            stdout_handle,
            stderr_handle,
            graceful_shutdown: config.graceful_shutdown,
        })
    }

    /// Process id of the running sidecar.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Wait for the process to exit and return its status.
    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }

    /// Send SIGTERM, wait up to `graceful_shutdown`, then SIGKILL if needed.
    pub async fn shutdown(mut self) -> Result<std::process::ExitStatus, AnalyticsError> {
        let pid = self.pid;
        let pid_i32 = i32::try_from(pid).map_err(|_| {
            AnalyticsError::Io(std::io::Error::other(format!(
                "pid {pid} does not fit in i32"
            )))
        })?;
        if let Err(err) = signal::kill(Pid::from_raw(pid_i32), Signal::SIGTERM) {
            warn!(pid, error = %err, "failed to send SIGTERM, falling back to kill");
        } else {
            debug!(pid, "sent SIGTERM to sidecar");
        }

        let status =
            if let Ok(result) = time::timeout(self.graceful_shutdown, self.child.wait()).await {
                result?
            } else {
                warn!(
                    pid,
                    "sidecar did not exit within grace period, sending SIGKILL"
                );
                self.child.start_kill()?;
                self.child.wait().await?
            };

        info!(pid, ?status, "sidecar exited");
        if let Some(handle) = self.stdout_handle.take() {
            let _ = handle.await;
        }
        if let Some(handle) = self.stderr_handle.take() {
            let _ = handle.await;
        }
        Ok(status)
    }
}

async fn forward_lines<R>(reader: R, pid: u32, is_stderr: bool)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if is_stderr {
                    warn!(pid, line = %line, "seer stderr");
                } else {
                    info!(pid, line = %line, "seer stdout");
                }
            }
            Ok(None) => break,
            Err(err) => {
                error!(pid, error = %err, "failed reading sidecar output");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_shell(script: &str, graceful: Duration) -> SidecarProcess {
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c").arg(script);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd.kill_on_drop(true);
        let mut child = cmd.spawn().expect("spawn shell");
        let pid = child.id().expect("pid present");
        let stdout_handle = child
            .stdout
            .take()
            .map(|stdout| tokio::spawn(forward_lines(stdout, pid, false)));
        let stderr_handle = child
            .stderr
            .take()
            .map(|stderr| tokio::spawn(forward_lines(stderr, pid, true)));
        SidecarProcess {
            child,
            pid,
            stdout_handle,
            stderr_handle,
            graceful_shutdown: graceful,
        }
    }

    #[tokio::test]
    async fn shutdown_kills_unresponsive_process() {
        let proc = spawn_shell(
            "trap '' TERM; while true; do sleep 0.1; done",
            Duration::from_millis(200),
        );
        let pid = proc.pid();
        let status = proc.shutdown().await.unwrap();
        assert!(!status.success());
        assert!(pid > 0);
    }

    #[tokio::test]
    async fn shutdown_returns_quickly_when_process_exits_on_sigterm() {
        let proc = spawn_shell(
            "trap 'exit 0' TERM; while true; do sleep 0.1; done",
            Duration::from_secs(2),
        );
        let started = std::time::Instant::now();
        let _ = proc.shutdown().await.unwrap();
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "shutdown should complete promptly when child honors SIGTERM"
        );
    }

    #[test]
    fn build_command_uses_socket_when_set() {
        let config = ProcessConfig {
            socket: Some(PathBuf::from("/tmp/seer.sock")),
            ..ProcessConfig::default()
        };
        let cmd = config.build_command();
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(args.iter().any(|a| a == "--socket"));
        assert!(!args.iter().any(|a| a == "--port"));
    }

    #[test]
    fn build_command_uses_host_port_by_default() {
        let cmd = ProcessConfig::default().build_command();
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(args.iter().any(|a| a == "--host"));
        assert!(args.iter().any(|a| a == "--port"));
        assert!(!args.iter().any(|a| a == "--socket"));
    }
}
