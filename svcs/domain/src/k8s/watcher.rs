//! Kubernetes discovery sync timing helpers.

use std::time::Duration;

/// Default interval for background Kubernetes discovery sync.
pub const DEFAULT_DISCOVERY_SYNC_INTERVAL: Duration = Duration::from_secs(300);

/// Build the interval used by the background discovery sync task.
///
/// The first tick completes immediately, so callers can run an initial sync
/// before waiting for the next five-minute period.
pub fn discovery_sync_interval() -> tokio::time::Interval {
    let mut interval = tokio::time::interval(DEFAULT_DISCOVERY_SYNC_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    interval
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sync_interval_is_five_minutes() {
        assert_eq!(DEFAULT_DISCOVERY_SYNC_INTERVAL, Duration::from_secs(300));
    }
}
