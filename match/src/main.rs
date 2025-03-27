mod config;
mod engine;
mod match_service;
mod metrics;
mod raft;
mod raft_client;
mod raft_service;
mod server;
mod state_match;

use tokio::signal;
use clap::Parser;



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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'c', long = "config", default_value_t = String::from("./config/config.toml"))]
    config: String,
    #[arg(short = 's', long = "stage", default_value_t = false)]
    stage: bool,
}

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
