//! Circuit breaker for the analytics gRPC client.
//!
//! Tracks consecutive failures; after [`CircuitBreakerConfig::failure_threshold`]
//! failures in a row, transitions to the *open* state and rejects calls for
//! [`CircuitBreakerConfig::cooldown`]. After the cooldown elapses, the next call
//! is admitted in *half-open* state; success closes the circuit, failure
//! re-opens it.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Configurable thresholds for the breaker.
#[derive(Debug, Clone, Copy)]
pub struct CircuitBreakerConfig {
    /// Consecutive failures required to open the circuit.
    pub failure_threshold: u32,
    /// How long the circuit stays open before allowing a probe.
    pub cooldown: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            cooldown: Duration::from_secs(60),
        }
    }
}

/// Public state for observability and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation; calls flow through.
    Closed,
    /// Circuit is open; calls are rejected until the cooldown expires.
    Open,
    /// A single probe call is permitted; success closes, failure reopens.
    HalfOpen,
}

#[derive(Debug)]
struct State {
    consecutive_failures: u32,
    open_since: Option<Instant>,
    half_open: bool,
}

impl State {
    fn fresh() -> Self {
        Self {
            consecutive_failures: 0,
            open_since: None,
            half_open: false,
        }
    }
}

/// Thread-safe circuit breaker.
#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Mutex<State>,
}

impl CircuitBreaker {
    /// Create a new breaker with the given configuration.
    #[must_use]
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Mutex::new(State::fresh()),
        }
    }

    /// Check whether a call should be admitted.
    ///
    /// Returns `Ok(())` if calls are permitted (closed or half-open), or
    /// `Err` with the `Duration` until the next probe attempt is allowed.
    pub fn admit(&self) -> Result<(), Duration> {
        self.admit_at(Instant::now())
    }

    /// Record a successful call.
    ///
    /// # Panics
    ///
    /// Panics only if the internal mutex has been poisoned by a panic in
    /// another thread, which we treat as a programming error.
    pub fn record_success(&self) {
        let mut state = self.state.lock().expect("circuit breaker mutex poisoned");
        state.consecutive_failures = 0;
        state.open_since = None;
        state.half_open = false;
    }

    /// Record a failed call.
    pub fn record_failure(&self) {
        self.record_failure_at(Instant::now());
    }

    /// Current state of the circuit.
    #[must_use]
    pub fn state(&self) -> CircuitState {
        self.state_at(Instant::now())
    }

    fn admit_at(&self, now: Instant) -> Result<(), Duration> {
        let mut state = self.state.lock().expect("circuit breaker mutex poisoned");
        match state.open_since {
            None => Ok(()),
            Some(open_since) => {
                let elapsed = now.saturating_duration_since(open_since);
                if elapsed >= self.config.cooldown {
                    // Promote to half-open: a single probe is allowed.
                    state.half_open = true;
                    Ok(())
                } else {
                    Err(self.config.cooldown - elapsed)
                }
            }
        }
    }

    fn record_failure_at(&self, now: Instant) {
        let mut state = self.state.lock().expect("circuit breaker mutex poisoned");
        if state.half_open {
            // Probe failed; restart the cooldown timer from now.
            state.open_since = Some(now);
            state.half_open = false;
            return;
        }
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        if state.consecutive_failures >= self.config.failure_threshold && state.open_since.is_none()
        {
            state.open_since = Some(now);
        }
    }

    fn state_at(&self, now: Instant) -> CircuitState {
        let mut state = self.state.lock().expect("circuit breaker mutex poisoned");
        if let Some(open_since) = state.open_since {
            let elapsed = now.saturating_duration_since(open_since);
            if elapsed >= self.config.cooldown {
                state.half_open = true;
                CircuitState::HalfOpen
            } else {
                CircuitState::Open
            }
        } else {
            CircuitState::Closed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaker(threshold: u32, cooldown_ms: u64) -> CircuitBreaker {
        CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: threshold,
            cooldown: Duration::from_millis(cooldown_ms),
        })
    }

    #[test]
    fn closed_admits_until_threshold() {
        let cb = breaker(3, 100);
        assert!(cb.admit().is_ok());
        cb.record_failure();
        cb.record_failure();
        assert!(cb.admit().is_ok(), "still closed after 2 failures");
        cb.record_failure();
        assert!(cb.admit().is_err(), "open after 3 failures");
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn success_resets_failure_count() {
        let cb = breaker(3, 100);
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        cb.record_failure();
        cb.record_failure();
        assert!(cb.admit().is_ok(), "success should reset");
    }

    #[test]
    fn half_open_after_cooldown() {
        let cb = breaker(2, 50);
        cb.record_failure();
        cb.record_failure();
        assert!(cb.admit().is_err());
        std::thread::sleep(Duration::from_millis(60));
        assert!(cb.admit().is_ok(), "cooldown elapsed: half-open admits");
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn half_open_failure_restarts_cooldown() {
        let cb = breaker(2, 30);
        cb.record_failure();
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(40));
        assert!(cb.admit().is_ok());
        cb.record_failure();
        // Restart of cooldown means we should still be open immediately after.
        assert!(cb.admit().is_err());
    }

    #[test]
    fn half_open_success_closes_circuit() {
        let cb = breaker(2, 30);
        cb.record_failure();
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(40));
        assert!(cb.admit().is_ok());
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }
}
