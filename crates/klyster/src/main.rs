//! Klyster binary entry point.

mod cli;

use clap::Parser;
use cli::Cli;
use domain::{logging, Config};
use std::process;
use tracing::info;

fn main() {
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

    if components.web {
        info!("Starting web component");
    }
    if components.agent {
        info!("Starting agent component");
    }
    if components.analytics {
        info!("Starting analytics component");
    }
    if components.ui {
        info!("Starting UI component");
    }
}
