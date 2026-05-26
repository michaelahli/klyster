//! CLI argument parsing for Klyster.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Command-line interface for Klyster.
#[derive(Parser, Debug)]
#[command(
    name = "klyster",
    version,
    about = "Capacity planning application for Kubernetes and VM workloads",
    long_about = None
)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE", default_value = "klyster.toml")]
    pub config: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, value_name = "LEVEL")]
    pub log_level: Option<LogLevel>,

    /// Run web server component
    #[arg(long)]
    pub web: bool,

    /// Run agent component
    #[arg(long)]
    pub agent: bool,

    /// Run analytics component
    #[arg(long)]
    pub analytics: bool,

    /// Run UI component (implies --web)
    #[arg(long)]
    pub ui: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Test Prometheus connection
    TestPrometheus {
        /// Prometheus server URL
        #[arg(long, default_value = "http://localhost:9090")]
        url: String,

        /// Request timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u64,

        /// Optional authentication token
        #[arg(long)]
        auth_token: Option<String>,
    },
}

/// Log level options.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LogLevel {
    /// Trace level logging
    Trace,
    /// Debug level logging
    Debug,
    /// Info level logging
    Info,
    /// Warn level logging
    Warn,
    /// Error level logging
    Error,
}

impl Cli {
    /// Determine which components should run based on flags.
    /// If no component flags are set, all components run.
    pub fn components(&self) -> ComponentFlags {
        let any_flag = self.web || self.agent || self.analytics || self.ui;

        ComponentFlags {
            web: !any_flag || self.web || self.ui,
            agent: !any_flag || self.agent,
            analytics: !any_flag || self.analytics,
            ui: !any_flag || self.ui,
        }
    }
}

/// Flags indicating which components should run.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub struct ComponentFlags {
    /// Run web server
    pub web: bool,
    /// Run agent
    pub agent: bool,
    /// Run analytics
    pub analytics: bool,
    /// Run UI
    pub ui: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_flags_runs_all() {
        let cli = Cli::parse_from(["klyster"]);
        let components = cli.components();
        assert!(components.web);
        assert!(components.agent);
        assert!(components.analytics);
        assert!(components.ui);
    }

    #[test]
    fn test_web_flag_only() {
        let cli = Cli::parse_from(["klyster", "--web"]);
        let components = cli.components();
        assert!(components.web);
        assert!(!components.agent);
        assert!(!components.analytics);
        assert!(!components.ui);
    }

    #[test]
    fn test_agent_flag_only() {
        let cli = Cli::parse_from(["klyster", "--agent"]);
        let components = cli.components();
        assert!(!components.web);
        assert!(components.agent);
        assert!(!components.analytics);
        assert!(!components.ui);
    }

    #[test]
    fn test_analytics_flag_only() {
        let cli = Cli::parse_from(["klyster", "--analytics"]);
        let components = cli.components();
        assert!(!components.web);
        assert!(!components.agent);
        assert!(components.analytics);
        assert!(!components.ui);
    }

    #[test]
    fn test_ui_flag_implies_web() {
        let cli = Cli::parse_from(["klyster", "--ui"]);
        let components = cli.components();
        assert!(components.web);
        assert!(!components.agent);
        assert!(!components.analytics);
        assert!(components.ui);
    }

    #[test]
    fn test_multiple_flags() {
        let cli = Cli::parse_from(["klyster", "--web", "--agent"]);
        let components = cli.components();
        assert!(components.web);
        assert!(components.agent);
        assert!(!components.analytics);
        assert!(!components.ui);
    }

    #[test]
    fn test_config_path_default() {
        let cli = Cli::parse_from(["klyster"]);
        assert_eq!(cli.config, PathBuf::from("klyster.toml"));
    }

    #[test]
    fn test_config_path_custom() {
        let cli = Cli::parse_from(["klyster", "--config", "/etc/klyster/config.toml"]);
        assert_eq!(cli.config, PathBuf::from("/etc/klyster/config.toml"));
    }

    #[test]
    fn test_log_level() {
        let cli = Cli::parse_from(["klyster", "--log-level", "debug"]);
        assert!(matches!(cli.log_level, Some(LogLevel::Debug)));
    }

    #[test]
    fn test_version_flag() {
        let result = Cli::try_parse_from(["klyster", "--version"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn test_help_flag() {
        let result = Cli::try_parse_from(["klyster", "--help"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn test_test_prometheus_subcommand() {
        let cli = Cli::parse_from(["klyster", "test-prometheus", "--url", "http://prom:9090"]);
        assert!(matches!(cli.command, Some(Commands::TestPrometheus { .. })));

        if let Some(Commands::TestPrometheus { url, .. }) = cli.command {
            assert_eq!(url, "http://prom:9090");
        }
    }

    #[test]
    fn test_test_prometheus_with_auth() {
        let cli = Cli::parse_from([
            "klyster",
            "test-prometheus",
            "--url",
            "http://prom:9090",
            "--auth-token",
            "secret123",
        ]);

        if let Some(Commands::TestPrometheus { auth_token, .. }) = cli.command {
            assert_eq!(auth_token, Some("secret123".to_string()));
        }
    }
}
