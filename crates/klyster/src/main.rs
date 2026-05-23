//! Klyster binary entry point.

mod bootstrap;
mod cli;

use bootstrap::Components;
use clap::Parser;
use cli::Cli;
use domain::{logging, Config};
use std::process;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let config = match Config::load(Some(&cli.config)) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {e}");
            process::exit(1);
        }
    };

    let log_level = cli.log_level.as_ref().map(|l| match l {
        cli::LogLevel::Trace => "trace",
        cli::LogLevel::Debug => "debug",
        cli::LogLevel::Info => "info",
        cli::LogLevel::Warn => "warn",
        cli::LogLevel::Error => "error",
    });

    if let Err(e) = logging::init(&config, log_level) {
        eprintln!("Failed to initialize logging: {e}");
        process::exit(1);
    }

    let components = cli.components();

    info!("Klyster starting");
    info!(
        components = ?components,
        "Components enabled"
    );

    let bootstrap_components = Components {
        web: components.web,
        agent: components.agent,
        analytics: components.analytics,
        ui: components.ui,
    };

    if let Err(e) = bootstrap::bootstrap(config, bootstrap_components).await {
        error!("Application failed: {}", e);
        process::exit(1);
    }
}
