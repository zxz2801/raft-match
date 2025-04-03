//! Main entry point for the Raft match service
//!
//! This module initializes the service, handles configuration, and manages the server lifecycle.

mod config;
mod engine;
mod match_service;
mod metrics;
mod raft;
mod raft_client;
mod raft_service;
mod server;
mod state_match;

use clap::Parser;
use tokio::signal;

/// Handles graceful shutdown signals
///
/// This function listens for Ctrl+C and SIGTERM signals on Unix systems,
/// allowing the service to shut down gracefully.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Command line arguments for the service
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short = 'c', long = "config", default_value_t = String::from("./config/config.toml"))]
    config: String,
    /// Whether to run in staging mode
    #[arg(short = 's', long = "stage", default_value_t = false)]
    stage: bool,
}

/// Main entry point of the application
///
/// This function:
/// 1. Initializes logging
/// 2. Parses command line arguments
/// 3. Loads configuration
/// 4. Starts the server
/// 5. Waits for shutdown signal
/// 6. Stops the server gracefully
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().unwrap_or_default();
    let args = Args::parse();
    config::RuntimeConfig::from_toml(&args.config).expect("Config is missing");
    {
        server::instance().lock().await.start().await;
    }
    shutdown_signal().await;
    {
        server::instance().lock().await.stop();
    }
    Ok(())
}
