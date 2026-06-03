//! System metrics collector using sysinfo.

use super::MetricCollector;
use anyhow::Result;
use chrono::Utc;
use domain::models::Metric;
use sysinfo::System;

/// System metrics collector.
pub struct SystemCollector {
    #[allow(dead_code)]
    hostname: String,
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCollector {
    /// Create a new system metrics collector.
    #[must_use]
    pub fn new() -> Self {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());

        Self { hostname }
    }
}

impl MetricCollector for SystemCollector {
    fn collect(&self) -> Result<Vec<Metric>> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let mut metrics = Vec::new();
        let timestamp = Utc::now();

        // CPU metrics
        let total_cpu = f64::from(sys.global_cpu_info().cpu_usage());
        metrics.push(Metric {
            id: 0,
            source_id: 0,
            name: "cpu_usage".to_string(),
            value: total_cpu,
            timestamp,
            created_at: timestamp,
        });

        // Memory metrics
        #[allow(clippy::cast_precision_loss)]
        let memory_used = sys.used_memory() as f64;
        #[allow(clippy::cast_precision_loss)]
        let memory_total = sys.total_memory() as f64;
        #[allow(clippy::cast_precision_loss)]
        let memory_available = sys.available_memory() as f64;

        metrics.push(Metric {
            id: 0,
            source_id: 0,
            name: "memory_used_bytes".to_string(),
            value: memory_used,
            timestamp,
            created_at: timestamp,
        });

        metrics.push(Metric {
            id: 0,
            source_id: 0,
            name: "memory_total_bytes".to_string(),
            value: memory_total,
            timestamp,
            created_at: timestamp,
        });

        metrics.push(Metric {
            id: 0,
            source_id: 0,
            name: "memory_available_bytes".to_string(),
            value: memory_available,
            timestamp,
            created_at: timestamp,
        });

        Ok(metrics)
    }

    fn name(&self) -> &'static str {
        "system"
    }
}
