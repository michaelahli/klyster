//! Klyster binary entry point.

mod cli;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();
    let components = cli.components();

    println!("Klyster starting with components:");
    if components.web {
        println!("  - Web");
    }
    if components.agent {
        println!("  - Agent");
    }
    if components.analytics {
        println!("  - Analytics");
    }
    if components.ui {
        println!("  - UI");
    }
}
