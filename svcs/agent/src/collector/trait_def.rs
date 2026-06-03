//! Metric collector trait and registry.

use anyhow::Result;
use domain::models::Metric;

/// Trait for metric collectors.
///
/// Collectors implement this trait to provide metrics from various sources
/// (system metrics, custom collectors, etc.).
pub trait MetricCollector: Send + Sync {
    /// Collect metrics from this source.
    ///
    /// Returns a vector of metrics or an error if collection fails.
    /// Collector errors should not crash the agent.
    fn collect(&self) -> Result<Vec<Metric>>;

    /// Name of this collector (for logging and identification).
    fn name(&self) -> &str;
}
