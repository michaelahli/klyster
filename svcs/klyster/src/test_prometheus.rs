//! Test Prometheus connection command.

use agent::prometheus::{PrometheusClient, PrometheusConfig};
use std::time::Duration;

/// Runs the test-prometheus command.
///
/// # Errors
///
/// Returns error if the connection test fails.
pub async fn run(url: &str, timeout_secs: u64, auth_token: Option<String>) -> anyhow::Result<()> {
    println!("Testing Prometheus connection...");
    println!("URL: {url}");
    println!("Timeout: {timeout_secs}s");

    let config = PrometheusConfig {
        url: url.to_string(),
        timeout: Duration::from_secs(timeout_secs),
        auth_token,
    };

    let client = PrometheusClient::new(config)?;

    println!("\n1. Testing health endpoint...");
    match client.health_check().await {
        Ok(()) => {
            println!("   ✓ Health check passed");
        }
        Err(e) => {
            println!("   ✗ Health check failed: {e}");
            return Err(anyhow::anyhow!("Health check failed"));
        }
    }

    println!("\n2. Testing instant query (up)...");
    match client.query_instant("up", None).await {
        Ok(result) => {
            println!("   ✓ Query successful");
            println!("   Found {} time series", result.len());

            if !result.is_empty() {
                println!("\n   Sample results:");
                for (idx, series) in result.iter().take(3).enumerate() {
                    println!("   [{idx}] Metric: {:?}", series.metric_name());
                    println!("       Labels: {} labels", series.metric.len());
                    let points = series.data_points();
                    if let Some((ts, val)) = points.first() {
                        println!("       Value: {val} at {ts}");
                    }
                }

                if result.len() > 3 {
                    println!("   ... and {} more", result.len() - 3);
                }
            }
        }
        Err(e) => {
            println!("   ✗ Query failed: {e}");
            return Err(anyhow::anyhow!("Query failed"));
        }
    }

    println!("\n✓ All tests passed!");
    println!("\nPrometheus connection is working correctly.");

    Ok(())
}
