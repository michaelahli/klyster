//! Resilient wrapper around [`AnalyticsClient`].
//!
//! Adds three things on top of the bare gRPC client:
//!
//! 1. **Retry**: transient errors (transport/connection/timeout/`Unavailable`,
//!    `DeadlineExceeded`, `ResourceExhausted`, `Aborted`, `Internal`) are
//!    retried with exponential backoff up to a configurable budget.
//! 2. **Circuit breaker**: after a run of consecutive failures the breaker
//!    opens and rejects calls until a cooldown elapses.
//! 3. **Tracing/metrics**: each call records structured span fields suitable
//!    for downstream Prometheus counters (label `metric_label()` covers all
//!    error variants).

// `AnalyticsError` carries `tonic::transport::Error`, which is intrinsically large.
#![allow(clippy::result_large_err)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::{debug, warn};

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::client::AnalyticsClient;
use crate::error::AnalyticsError;
use crate::proto::{
    ForecastRequest, ForecastResponse, FunctionCode, FunctionList, HealthStatus, ValidationResult,
};

/// Tunables for the resilient wrapper.
#[derive(Debug, Clone, Copy)]
pub struct RetryConfig {
    /// Maximum number of attempts (initial call + retries).
    pub max_attempts: u32,
    /// Initial backoff between attempts.
    pub initial_backoff: Duration,
    /// Cap on the backoff to prevent runaway growth.
    pub max_backoff: Duration,
    /// Multiplier applied to the backoff after each attempt (>= 1.0).
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(2),
            backoff_factor: 2.0,
        }
    }
}

/// Composite configuration for the resilient client.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResilientConfig {
    /// Retry policy applied per call.
    pub retry: RetryConfig,
    /// Circuit breaker thresholds.
    pub circuit: CircuitBreakerConfig,
}

/// Wrapper that adds retry + circuit breaking around an `AnalyticsClient`.
///
/// # Panics
///
/// Does not panic in practice: the retry loop runs at least one attempt before
/// inspecting `last_err`.
#[derive(Debug, Clone)]
pub struct ResilientClient {
    inner: AnalyticsClient,
    config: ResilientConfig,
    breaker: Arc<CircuitBreaker>,
}

impl ResilientClient {
    /// Wrap an existing client.
    #[must_use]
    pub fn new(inner: AnalyticsClient, config: ResilientConfig) -> Self {
        Self {
            inner,
            config,
            breaker: Arc::new(CircuitBreaker::new(config.circuit)),
        }
    }

    /// Access the underlying client (e.g. for one-off non-resilient calls).
    #[must_use]
    pub fn inner(&self) -> &AnalyticsClient {
        &self.inner
    }

    /// Run a forecast with retries and circuit breaking.
    pub async fn run_forecast(
        &self,
        request: ForecastRequest,
    ) -> Result<ForecastResponse, AnalyticsError> {
        self.call("run_forecast", || {
            let req = request.clone();
            let inner = self.inner.clone();
            async move { inner.run_forecast(req).await }
        })
        .await
    }

    /// Validate a custom function with retries and circuit breaking.
    pub async fn validate_function(
        &self,
        request: FunctionCode,
    ) -> Result<ValidationResult, AnalyticsError> {
        self.call("validate_function", || {
            let req = request.clone();
            let inner = self.inner.clone();
            async move { inner.validate_function(req).await }
        })
        .await
    }

    /// List predefined functions.
    pub async fn list_predefined_functions(&self) -> Result<FunctionList, AnalyticsError> {
        self.call("list_predefined_functions", || {
            let inner = self.inner.clone();
            async move { inner.list_predefined_functions().await }
        })
        .await
    }

    /// Probe sidecar health.
    pub async fn health_check(&self) -> Result<HealthStatus, AnalyticsError> {
        self.call("health_check", || {
            let inner = self.inner.clone();
            async move { inner.health_check().await }
        })
        .await
    }

    async fn call<F, Fut, T>(
        &self,
        operation: &'static str,
        mut make: F,
    ) -> Result<T, AnalyticsError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, AnalyticsError>>,
    {
        if let Err(retry_after) = self.breaker.admit() {
            warn!(
                operation,
                ?retry_after,
                "circuit breaker open, rejecting call"
            );
            return Err(AnalyticsError::CircuitOpen { retry_after });
        }

        let max = self.config.retry.max_attempts.max(1);
        let mut backoff = self.config.retry.initial_backoff;
        let mut last_err: Option<AnalyticsError> = None;
        let started = Instant::now();

        for attempt in 1..=max {
            match make().await {
                Ok(value) => {
                    self.breaker.record_success();
                    debug!(operation, attempt, "rpc succeeded");
                    return Ok(value);
                }
                Err(err) => {
                    let label = err.metric_label();
                    let transient = err.is_transient();
                    warn!(
                        operation,
                        attempt,
                        error = %err,
                        error_label = label,
                        transient,
                        "rpc attempt failed"
                    );
                    self.breaker.record_failure();
                    if !transient || attempt == max {
                        last_err = Some(err);
                        break;
                    }
                    last_err = Some(err);
                    tokio::time::sleep(backoff).await;
                    backoff = next_backoff(backoff, &self.config.retry);
                }
            }
        }

        let err = last_err.expect("retry loop runs at least once");
        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        warn!(operation, elapsed_ms, error = %err, "giving up after retries");
        Err(err)
    }

    /// Snapshot of the breaker state, useful for tests/metrics.
    #[must_use]
    pub fn circuit_state(&self) -> crate::circuit_breaker::CircuitState {
        self.breaker.state()
    }
}

fn next_backoff(current: Duration, config: &RetryConfig) -> Duration {
    let factor = config.backoff_factor.max(1.0);
    // Use seconds (f64) for the computation; precision loss only matters at
    // ~285 years, far beyond any reasonable backoff.
    let scaled = current.as_secs_f64() * factor;
    if !scaled.is_finite() || scaled <= 0.0 {
        return config.max_backoff;
    }
    let cap = config.max_backoff.as_secs_f64();
    let bumped = if scaled >= cap {
        config.max_backoff
    } else {
        Duration::from_secs_f64(scaled)
    };
    bumped.min(config.max_backoff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_backoff_caps_at_max() {
        let cfg = RetryConfig {
            max_attempts: 5,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(3),
            backoff_factor: 10.0,
        };
        let d = next_backoff(Duration::from_secs(1), &cfg);
        assert_eq!(d, Duration::from_secs(3));
    }

    #[test]
    fn next_backoff_grows_under_cap() {
        let cfg = RetryConfig {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_factor: 2.0,
        };
        let d = next_backoff(Duration::from_millis(100), &cfg);
        assert_eq!(d, Duration::from_millis(200));
    }
}
