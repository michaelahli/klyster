//! Configuration management for Klyster.

use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Database configuration
    pub database: DatabaseConfig,
    /// Web server configuration
    pub web: WebConfig,
    /// Agent configuration
    pub agent: AgentConfig,
    /// Analytics configuration
    pub analytics: AnalyticsConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Telemetry configuration
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    /// Metrics configuration
    #[serde(default)]
    pub metrics: MetricsConfig,
    /// Data retention configuration
    #[serde(default)]
    pub retention: RetentionConfig,
}

/// Database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type: "sqlite" or "postgres"
    #[serde(default = "default_db_type")]
    pub db_type: String,
    /// `SQLite` database file path
    #[serde(default = "default_sqlite_path")]
    pub sqlite_path: String,
    /// `PostgreSQL` connection URL (optional)
    #[serde(default)]
    pub postgres_url: Option<String>,
    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    /// Enable `SQLite` WAL mode
    #[serde(default = "default_wal_mode")]
    pub wal_mode: bool,
}

/// Web server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Server bind address
    #[serde(default = "default_web_host")]
    pub host: String,
    /// Server port
    #[serde(default = "default_web_port")]
    pub port: u16,
    /// Number of worker threads
    #[serde(default = "default_workers")]
    pub workers: usize,
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Enable agent
    #[serde(default = "default_agent_enabled")]
    pub enabled: bool,
    /// Collection interval in seconds
    #[serde(default = "default_agent_interval")]
    pub collection_interval_secs: u64,
}

/// Analytics configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Enable analytics
    #[serde(default = "default_analytics_enabled")]
    pub enabled: bool,
    /// gRPC endpoint for Python analytics sidecar
    #[serde(default = "default_analytics_endpoint")]
    pub grpc_endpoint: String,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log format: "json", "logfmt", or "text"
    #[serde(default = "default_log_format")]
    pub format: String,
    /// Log level: "trace", "debug", "info", "warn", or "error"
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log outputs
    #[serde(default)]
    pub outputs: Vec<LogOutput>,
}

/// Log output configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LogOutput {
    /// Standard output
    Stdout,
    /// Standard error
    Stderr,
    /// File output with rotation
    File {
        /// File path
        path: String,
        /// Rotation strategy: "daily", "hourly", or "size"
        #[serde(default = "default_rotation")]
        rotation: String,
        /// Retention period in days
        #[serde(default = "default_retention_days")]
        retention_days: u32,
        /// Optional filter (e.g., "audit" for audit logs only)
        #[serde(default)]
        filter: Option<String>,
    },
    /// Syslog output
    Syslog {
        /// Syslog server address
        address: String,
    },
}

/// Telemetry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Enable telemetry
    #[serde(default = "default_telemetry_enabled")]
    pub enabled: bool,
    /// Exporter type: "otlp", "jaeger", "zipkin", or "stdout"
    #[serde(default = "default_telemetry_exporter")]
    pub exporter: String,
    /// Exporter endpoint
    #[serde(default = "default_telemetry_endpoint")]
    pub endpoint: String,
    /// Service name for traces
    #[serde(default = "default_service_name")]
    pub service_name: String,
    /// Sampling rate (0.0 to 1.0)
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

/// Metrics configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable Prometheus metrics endpoint
    #[serde(default = "default_metrics_enabled")]
    pub enabled: bool,
    /// Metrics endpoint path
    #[serde(default = "default_metrics_endpoint")]
    pub endpoint: String,
}

/// Data retention configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Metrics retention period in days
    #[serde(default = "default_metrics_days")]
    pub metrics_days: u32,
    /// Forecasts retention period in days
    #[serde(default = "default_forecasts_days")]
    pub forecasts_days: u32,
    /// Recommendations retention period in days
    #[serde(default = "default_recommendations_days")]
    pub recommendations_days: u32,
    /// Audit logs retention period in days
    #[serde(default = "default_audit_logs_days")]
    pub audit_logs_days: u32,
    /// Cleanup job schedule (cron format)
    #[serde(default = "default_cleanup_schedule")]
    pub cleanup_schedule: String,
}

fn default_db_type() -> String {
    "sqlite".to_string()
}

fn default_sqlite_path() -> String {
    "./data/klyster.db".to_string()
}

fn default_pool_size() -> u32 {
    10
}

fn default_wal_mode() -> bool {
    true
}

fn default_web_host() -> String {
    "127.0.0.1".to_string()
}

fn default_web_port() -> u16 {
    8080
}

fn default_workers() -> usize {
    num_cpus::get()
}

fn default_agent_enabled() -> bool {
    false
}

fn default_agent_interval() -> u64 {
    60
}

fn default_analytics_enabled() -> bool {
    false
}

fn default_analytics_endpoint() -> String {
    "http://127.0.0.1:50051".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_rotation() -> String {
    "daily".to_string()
}

fn default_retention_days() -> u32 {
    30
}

fn default_telemetry_enabled() -> bool {
    false
}

fn default_telemetry_exporter() -> String {
    "otlp".to_string()
}

fn default_telemetry_endpoint() -> String {
    "http://localhost:4317".to_string()
}

fn default_service_name() -> String {
    "klyster".to_string()
}

fn default_sample_rate() -> f64 {
    1.0
}

fn default_metrics_enabled() -> bool {
    true
}

fn default_metrics_endpoint() -> String {
    "/metrics".to_string()
}

fn default_metrics_days() -> u32 {
    90
}

fn default_forecasts_days() -> u32 {
    180
}

fn default_recommendations_days() -> u32 {
    365
}

fn default_audit_logs_days() -> u32 {
    365
}

fn default_cleanup_schedule() -> String {
    "0 2 * * *".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            format: default_log_format(),
            level: default_log_level(),
            outputs: vec![LogOutput::Stdout],
        }
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: default_telemetry_enabled(),
            exporter: default_telemetry_exporter(),
            endpoint: default_telemetry_endpoint(),
            service_name: default_service_name(),
            sample_rate: default_sample_rate(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_metrics_enabled(),
            endpoint: default_metrics_endpoint(),
        }
    }
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            metrics_days: default_metrics_days(),
            forecasts_days: default_forecasts_days(),
            recommendations_days: default_recommendations_days(),
            audit_logs_days: default_audit_logs_days(),
            cleanup_schedule: default_cleanup_schedule(),
        }
    }
}

impl Config {
    /// Load configuration from TOML file with environment variable overrides.
    ///
    /// Environment variables use the prefix `KLYSTER_` with underscores as separators.
    /// For example: `KLYSTER_WEB_PORT=9090`
    pub fn load(config_path: Option<&Path>) -> Result<Self, ConfigError> {
        let mut builder = ConfigBuilder::builder();

        if let Some(path) = config_path {
            builder = builder.add_source(File::from(path).required(true));
        } else {
            builder = builder.add_source(File::with_name("klyster").required(false));
        }

        builder = builder.add_source(
            Environment::with_prefix("KLYSTER")
                .separator("_")
                .try_parsing(true),
        );

        let config = builder.build()?;
        let cfg: Config = config.try_deserialize()?;

        cfg.validate()?;

        Ok(cfg)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.web.port == 0 {
            return Err(ConfigError::Message(format!(
                "Invalid web port: {}",
                self.web.port
            )));
        }

        if self.database.pool_size == 0 {
            return Err(ConfigError::Message(
                "Database pool size must be greater than 0".to_string(),
            ));
        }

        if !["json", "logfmt", "text"].contains(&self.logging.format.as_str()) {
            return Err(ConfigError::Message(format!(
                "Invalid log format: {}. Must be one of: json, logfmt, text",
                self.logging.format
            )));
        }

        if ![
            "trace", "debug", "info", "warn", "error", "TRACE", "DEBUG", "INFO", "WARN", "ERROR",
        ]
        .contains(&self.logging.level.as_str())
        {
            return Err(ConfigError::Message(format!(
                "Invalid log level: {}. Must be one of: trace, debug, info, warn, error",
                self.logging.level
            )));
        }

        if self.telemetry.enabled
            && !["otlp", "jaeger", "zipkin", "stdout"].contains(&self.telemetry.exporter.as_str())
        {
            return Err(ConfigError::Message(format!(
                "Invalid telemetry exporter: {}. Must be one of: otlp, jaeger, zipkin, stdout",
                self.telemetry.exporter
            )));
        }

        if self.telemetry.sample_rate < 0.0 || self.telemetry.sample_rate > 1.0 {
            return Err(ConfigError::Message(format!(
                "Invalid telemetry sample_rate: {}. Must be between 0.0 and 1.0",
                self.telemetry.sample_rate
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::io::Write;

    #[test]
    #[serial]
    fn test_default_config() {
        env::remove_var("KLYSTER_WEB_PORT");
        env::remove_var("KLYSTER_LOGGING_LEVEL");

        let mut file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            file,
            r#"
[database]
db_type = "sqlite"

[web]
host = "127.0.0.1"
port = 8080

[agent]
enabled = false

[analytics]
enabled = false

[logging]
format = "json"
level = "info"
"#
        )
        .unwrap();

        let config = Config::load(Some(file.path())).unwrap();
        assert_eq!(config.database.db_type, "sqlite");
        assert_eq!(config.web.port, 8080);
        assert_eq!(config.logging.format, "json");
    }

    #[test]
    #[serial]
    fn test_env_override() {
        let mut file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            file,
            r#"
[database]
db_type = "sqlite"

[web]
host = "127.0.0.1"
port = 8080

[agent]
enabled = false

[analytics]
enabled = false

[logging]
format = "json"
level = "info"
"#
        )
        .unwrap();

        env::set_var("KLYSTER_WEB_PORT", "9090");
        env::set_var("KLYSTER_LOGGING_LEVEL", "debug");

        let config = Config::load(Some(file.path())).unwrap();
        assert_eq!(config.web.port, 9090);
        assert_eq!(config.logging.level, "debug");

        env::remove_var("KLYSTER_WEB_PORT");
        env::remove_var("KLYSTER_LOGGING_LEVEL");
    }

    #[test]
    #[serial]
    fn test_validation_invalid_port() {
        env::remove_var("KLYSTER_WEB_PORT");
        let mut file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            file,
            r#"
[database]
db_type = "sqlite"

[web]
host = "127.0.0.1"
port = 99999

[agent]
enabled = false

[analytics]
enabled = false

[logging]
format = "json"
level = "info"
"#
        )
        .unwrap();

        let result = Config::load(Some(file.path()));
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_log_format() {
        let mut file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            file,
            r#"
[database]
db_type = "sqlite"

[web]
host = "127.0.0.1"
port = 8080

[agent]
enabled = false

[analytics]
enabled = false

[logging]
format = "invalid"
level = "info"
"#
        )
        .unwrap();

        let result = Config::load(Some(file.path()));
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_validation_invalid_log_level() {
        let mut file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            file,
            r#"
[database]
db_type = "sqlite"

[web]
host = "127.0.0.1"
port = 8080

[agent]
enabled = false

[analytics]
enabled = false

[logging]
format = "json"
level = "invalid"
"#
        )
        .unwrap();

        let result = Config::load(Some(file.path()));
        assert!(result.is_err());
    }

    #[test]
    fn test_postgres_config() {
        let mut file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            file,
            r#"
[database]
db_type = "postgres"
postgres_url = "postgresql://user:pass@localhost/klyster"

[web]
host = "127.0.0.1"
port = 8080

[agent]
enabled = false

[analytics]
enabled = false

[logging]
format = "json"
level = "info"
"#
        )
        .unwrap();

        let config = Config::load(Some(file.path())).unwrap();
        assert_eq!(config.database.db_type, "postgres");
        assert_eq!(
            config.database.postgres_url,
            Some("postgresql://user:pass@localhost/klyster".to_string())
        );
    }

    #[test]
    fn test_multiple_log_outputs() {
        let mut file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            file,
            r#"
[database]
db_type = "sqlite"

[web]
host = "127.0.0.1"
port = 8080

[agent]
enabled = false

[analytics]
enabled = false

[logging]
format = "json"
level = "info"

[[logging.outputs]]
type = "stdout"

[[logging.outputs]]
type = "file"
path = "/var/log/klyster/app.log"
rotation = "daily"
retention_days = 30
"#
        )
        .unwrap();

        let config = Config::load(Some(file.path())).unwrap();
        assert_eq!(config.logging.outputs.len(), 2);
        assert!(matches!(config.logging.outputs[0], LogOutput::Stdout));
        if let LogOutput::File { path, .. } = &config.logging.outputs[1] {
            assert_eq!(path, "/var/log/klyster/app.log");
        } else {
            panic!("Expected File output");
        }
    }

    #[test]
    fn test_telemetry_config() {
        let mut file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            file,
            r#"
[database]
db_type = "sqlite"

[web]
host = "127.0.0.1"
port = 8080

[agent]
enabled = false

[analytics]
enabled = false

[logging]
format = "json"
level = "info"

[telemetry]
enabled = true
exporter = "otlp"
endpoint = "http://localhost:4317"
service_name = "klyster"
sample_rate = 0.5
"#
        )
        .unwrap();

        let config = Config::load(Some(file.path())).unwrap();
        assert!(config.telemetry.enabled);
        assert_eq!(config.telemetry.exporter, "otlp");
        assert!((config.telemetry.sample_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_retention_config() {
        let mut file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            file,
            r#"
[database]
db_type = "sqlite"

[web]
host = "127.0.0.1"
port = 8080

[agent]
enabled = false

[analytics]
enabled = false

[logging]
format = "json"
level = "info"

[retention]
metrics_days = 60
forecasts_days = 120
recommendations_days = 180
audit_logs_days = 730
cleanup_schedule = "0 3 * * *"
"#
        )
        .unwrap();

        let config = Config::load(Some(file.path())).unwrap();
        assert_eq!(config.retention.metrics_days, 60);
        assert_eq!(config.retention.forecasts_days, 120);
        assert_eq!(config.retention.recommendations_days, 180);
        assert_eq!(config.retention.audit_logs_days, 730);
        assert_eq!(config.retention.cleanup_schedule, "0 3 * * *");
    }
}
