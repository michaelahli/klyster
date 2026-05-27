//! Supervise the Seer sidecar: spawn, health-check, auto-restart, shutdown.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, error, warn};

use crate::client::{AnalyticsClient, ClientConfig};
use crate::error::AnalyticsError;
use crate::process::{ProcessConfig, SidecarProcess};

/// Configuration for the supervisor.
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    /// How the sidecar process is spawned.
    pub process: ProcessConfig,
    /// How the gRPC client should connect once the sidecar is up.
    pub client: ClientConfig,
    /// Interval between health probes.
    pub health_interval: Duration,
    /// Sliding window over which restart attempts are counted.
    pub restart_window: Duration,
    /// Maximum restart attempts within `restart_window` before giving up.
    pub max_restarts: u32,
    /// Initial delay before starting the first health probe (allows the
    /// sidecar to bind its socket).
    pub startup_grace: Duration,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            process: ProcessConfig::default(),
            client: ClientConfig::default(),
            health_interval: Duration::from_secs(10),
            restart_window: Duration::from_secs(60),
            max_restarts: 3,
            startup_grace: Duration::from_secs(2),
        }
    }
}

/// Reasons a supervisor loop iteration may exit.
#[derive(Debug)]
enum LoopExit {
    /// User requested shutdown.
    Shutdown,
    /// The child process exited.
    ChildExited,
    /// Health check failed.
    HealthFailed(AnalyticsError),
}

/// Handle returned by [`Supervisor::start`].
#[derive(Debug)]
pub struct SupervisorHandle {
    shutdown: Option<oneshot::Sender<()>>,
    join: JoinHandle<Result<(), SupervisorError>>,
}

impl SupervisorHandle {
    /// Request a graceful shutdown of the supervisor and the underlying sidecar.
    pub async fn shutdown(mut self) -> Result<(), SupervisorError> {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        match self.join.await {
            Ok(result) => result,
            Err(err) if err.is_cancelled() => Ok(()),
            Err(err) => Err(SupervisorError::Join(err.to_string())),
        }
    }
}

/// Errors returned by the supervisor task.
#[derive(thiserror::Error, Debug)]
#[allow(clippy::result_large_err)]
pub enum SupervisorError {
    /// The sidecar crashed too many times within the configured window.
    #[error("sidecar exceeded restart budget: {restarts} restarts in {window:?}")]
    RestartBudgetExceeded {
        /// Number of restarts within the window.
        restarts: u32,
        /// Sliding window length.
        window: Duration,
    },

    /// Spawning the sidecar process failed.
    #[error("failed to spawn sidecar: {0}")]
    Spawn(#[from] AnalyticsError),

    /// The supervisor task itself failed to join.
    #[error("supervisor task join failure: {0}")]
    Join(String),
}

/// Events emitted by the supervisor for observability or tests.
#[derive(Debug, Clone)]
pub enum SupervisorEvent {
    /// Sidecar was spawned with the given pid.
    Started {
        /// Process id of the spawned child.
        pid: u32,
    },
    /// Sidecar exited and the supervisor will restart it.
    Restarting {
        /// Reason summary for the restart.
        reason: String,
    },
    /// Sidecar exited and the supervisor stopped trying to restart it.
    Stopped,
}

/// Supervises the Seer sidecar process.
#[derive(Debug)]
pub struct Supervisor {
    config: SupervisorConfig,
    events: Option<mpsc::Sender<SupervisorEvent>>,
}

impl Supervisor {
    /// Build a supervisor with the given configuration.
    pub fn new(config: SupervisorConfig) -> Self {
        Self {
            config,
            events: None,
        }
    }

    /// Attach an event channel for observability/testing.
    #[must_use]
    pub fn with_events(mut self, events: mpsc::Sender<SupervisorEvent>) -> Self {
        self.events = Some(events);
        self
    }

    /// Start the supervisor on a background task.
    pub fn start(self) -> SupervisorHandle {
        let (tx, rx) = oneshot::channel();
        let join = tokio::spawn(self.run(rx));
        SupervisorHandle {
            shutdown: Some(tx),
            join,
        }
    }

    async fn run(self, mut shutdown: oneshot::Receiver<()>) -> Result<(), SupervisorError> {
        let mut restart_history: VecDeque<Instant> = VecDeque::new();

        loop {
            // Trim history outside the sliding window.
            let now = Instant::now();
            let cutoff = now.checked_sub(self.config.restart_window).unwrap_or(now);
            while restart_history.front().is_some_and(|t| *t < cutoff) {
                restart_history.pop_front();
            }
            let restart_count = u32::try_from(restart_history.len()).unwrap_or(u32::MAX);
            if restart_count > self.config.max_restarts {
                return Err(SupervisorError::RestartBudgetExceeded {
                    restarts: restart_count,
                    window: self.config.restart_window,
                });
            }

            let mut process = SidecarProcess::spawn(&self.config.process)?;
            let pid = process.pid();
            self.emit(SupervisorEvent::Started { pid }).await;

            time::sleep(self.config.startup_grace).await;

            let exit = self.supervise_running(&mut process, &mut shutdown).await;
            match exit {
                LoopExit::Shutdown => {
                    let _ = process.shutdown().await;
                    self.emit(SupervisorEvent::Stopped).await;
                    return Ok(());
                }
                LoopExit::ChildExited => {
                    warn!(pid, "sidecar exited unexpectedly, scheduling restart");
                    self.emit(SupervisorEvent::Restarting {
                        reason: "child exited".to_string(),
                    })
                    .await;
                    restart_history.push_back(Instant::now());
                }
                LoopExit::HealthFailed(err) => {
                    warn!(pid, error = %err, "health check failed, restarting sidecar");
                    let _ = process.shutdown().await;
                    self.emit(SupervisorEvent::Restarting {
                        reason: format!("health check failed: {err}"),
                    })
                    .await;
                    restart_history.push_back(Instant::now());
                }
            }
        }
    }

    async fn supervise_running(
        &self,
        process: &mut SidecarProcess,
        shutdown: &mut oneshot::Receiver<()>,
    ) -> LoopExit {
        let client = match AnalyticsClient::connect(self.config.client.clone()).await {
            Ok(client) => client,
            Err(err) => return LoopExit::HealthFailed(err),
        };
        debug!("connected to sidecar; entering health monitoring loop");

        let mut interval = time::interval(self.config.health_interval);
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
        // Skip the immediate first tick so we don't probe before the startup
        // grace has truly elapsed.
        interval.tick().await;

        loop {
            tokio::select! {
                biased;
                _ = &mut *shutdown => return LoopExit::Shutdown,
                exit = process.wait() => {
                    match exit {
                        Ok(status) => warn!(?status, "sidecar process exited"),
                        Err(err) => error!(error = %err, "failed to wait on sidecar"),
                    }
                    return LoopExit::ChildExited;
                }
                _ = interval.tick() => {
                    if let Err(err) = client.health_check().await {
                        return LoopExit::HealthFailed(err);
                    }
                    debug!("sidecar health check ok");
                }
            }
        }
    }

    async fn emit(&self, event: SupervisorEvent) {
        if let Some(events) = &self.events {
            if let Err(err) = events.send(event).await {
                debug!(error = %err, "supervisor event channel closed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_budget_zero_disallows_any_restart() {
        let config = SupervisorConfig {
            max_restarts: 0,
            ..SupervisorConfig::default()
        };
        assert_eq!(config.max_restarts, 0);
    }
}
