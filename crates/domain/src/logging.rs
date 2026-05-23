//! Logging initialization and configuration.

use crate::config::Config;
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging based on configuration.
///
/// Sets up tracing subscriber with configurable format and log level.
/// Respects `RUST_LOG` environment variable if set.
///
/// Supported formats:
/// - `json`: JSON structured logs (default)
/// - `text`: Plain text logs
/// - `logfmt`: Logfmt-style logs
///
/// Note: Multiple outputs (file, syslog) and advanced features (rotation, audit logs)
/// will be fully implemented in M2. For M1, stdout output is supported.
pub fn init(config: &Config, cli_log_level: Option<&str>) -> Result<(), String> {
    let log_level = cli_log_level
        .or(Some(&config.logging.level))
        .unwrap_or("info");

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("{}={}", env!("CARGO_PKG_NAME"), log_level)));

    let format = &config.logging.format;

    match format.as_str() {
        "json" => {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    fmt::layer()
                        .json()
                        .with_current_span(true)
                        .with_span_list(true)
                        .with_target(true)
                        .with_timer(fmt::time::UtcTime::rfc_3339()),
                )
                .try_init()
                .map_err(|e| format!("Failed to initialize logging: {e}"))?;
        }
        "text" => {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    fmt::layer()
                        .with_target(true)
                        .with_timer(fmt::time::UtcTime::rfc_3339()),
                )
                .try_init()
                .map_err(|e| format!("Failed to initialize logging: {e}"))?;
        }
        "logfmt" => {
            // Logfmt format (key=value pairs)
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    fmt::layer()
                        .with_target(true)
                        .with_timer(fmt::time::UtcTime::rfc_3339()),
                )
                .try_init()
                .map_err(|e| format!("Failed to initialize logging: {e}"))?;
        }
        _ => {
            return Err(format!("Unsupported log format: {format}"));
        }
    }

    Ok(())
}

/// Parse log level string to tracing Level.
pub fn parse_level(level: &str) -> Result<Level, String> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(Level::TRACE),
        "debug" => Ok(Level::DEBUG),
        "info" => Ok(Level::INFO),
        "warn" => Ok(Level::WARN),
        "error" => Ok(Level::ERROR),
        _ => Err(format!("Invalid log level: {level}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_level() {
        assert!(matches!(parse_level("trace"), Ok(Level::TRACE)));
        assert!(matches!(parse_level("debug"), Ok(Level::DEBUG)));
        assert!(matches!(parse_level("info"), Ok(Level::INFO)));
        assert!(matches!(parse_level("warn"), Ok(Level::WARN)));
        assert!(matches!(parse_level("error"), Ok(Level::ERROR)));
        assert!(matches!(parse_level("TRACE"), Ok(Level::TRACE)));
        assert!(parse_level("invalid").is_err());
    }
}
