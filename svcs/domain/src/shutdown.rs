//! Graceful shutdown orchestration.

use tokio::sync::broadcast;
use tracing::{info, warn};

/// Shutdown signal broadcaster.
#[derive(Clone)]
pub struct ShutdownSignal {
    sender: broadcast::Sender<()>,
}

impl ShutdownSignal {
    /// Create a new shutdown signal broadcaster.
    #[must_use] 
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1);
        Self { sender }
    }

    /// Subscribe to shutdown signals.
    #[must_use] 
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.sender.subscribe()
    }

    /// Broadcast shutdown signal to all subscribers.
    pub fn shutdown(&self) {
        info!("Broadcasting shutdown signal");
        let _ = self.sender.send(());
    }

    /// Get the number of active subscribers.
    #[must_use] 
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// Wait for shutdown signal (SIGTERM or SIGINT).
///
/// # Panics
///
/// Panics if signal handlers cannot be installed.
pub async fn wait_for_signal() {
    use tokio::signal;

    #[cfg(unix)]
    {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
            .expect("Failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT");
            }
        }
    }

    #[cfg(not(unix))]
    {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C");
    }
}

/// Shutdown coordinator that manages graceful shutdown with timeout.
pub struct ShutdownCoordinator {
    signal: ShutdownSignal,
    timeout_secs: u64,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator with specified timeout.
    #[must_use] 
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            signal: ShutdownSignal::new(),
            timeout_secs,
        }
    }

    /// Get the shutdown signal broadcaster.
    #[must_use] 
    pub fn signal(&self) -> ShutdownSignal {
        self.signal.clone()
    }

    /// Wait for OS signal and initiate graceful shutdown.
    pub async fn wait_and_shutdown(self) {
        wait_for_signal().await;

        info!("Initiating graceful shutdown");
        self.signal.shutdown();

        // Wait for components to shut down with timeout
        let timeout = tokio::time::Duration::from_secs(self.timeout_secs);
        tokio::select! {
            () = tokio::time::sleep(timeout) => {
                warn!(
                    timeout_secs = self.timeout_secs,
                    "Shutdown timeout reached, forcing exit"
                );
            }
            () = self.wait_for_completion() => {
                info!("All components shut down gracefully");
            }
        }
    }

    /// Wait for all subscribers to drop their receivers.
    async fn wait_for_completion(&self) {
        loop {
            if self.signal.receiver_count() == 0 {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new(30)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_shutdown_signal_broadcast() {
        let signal = ShutdownSignal::new();
        let mut rx1 = signal.subscribe();
        let mut rx2 = signal.subscribe();

        assert_eq!(signal.receiver_count(), 2);

        signal.shutdown();

        // Both receivers should get the signal
        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_timeout() {
        let coordinator = ShutdownCoordinator::new(1);
        let signal = coordinator.signal();
        let mut rx = signal.subscribe();

        // Spawn a task that holds the receiver
        let handle = tokio::spawn(async move {
            // Simulate a component that takes too long to shut down
            sleep(Duration::from_secs(5)).await;
            let _ = rx.recv().await;
        });

        // Trigger shutdown
        signal.shutdown();

        // Wait a bit to ensure timeout is triggered
        sleep(Duration::from_millis(1500)).await;

        // The handle should still be running (not completed)
        assert!(!handle.is_finished());

        // Clean up
        handle.abort();
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_graceful() {
        let coordinator = ShutdownCoordinator::new(5);
        let signal = coordinator.signal();
        let mut rx = signal.subscribe();

        // Spawn a task that shuts down quickly
        let handle = tokio::spawn(async move {
            let _ = rx.recv().await;
            // Simulate quick cleanup
            sleep(Duration::from_millis(100)).await;
        });

        // Trigger shutdown
        signal.shutdown();

        // Wait for the task to complete
        handle.await.unwrap();

        // Receiver count should be 0 now
        assert_eq!(signal.receiver_count(), 0);
    }

    #[test]
    fn test_shutdown_signal_default() {
        let signal = ShutdownSignal::default();
        assert_eq!(signal.receiver_count(), 0);
    }

    #[test]
    fn test_shutdown_coordinator_default() {
        let coordinator = ShutdownCoordinator::default();
        assert_eq!(coordinator.timeout_secs, 30);
    }
}
