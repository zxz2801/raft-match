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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().unwrap_or_default();
    config::RuntimeConfig::from_toml("config.toml").expect("Config is missing");
    {
        server::instance().lock().await.start().await;
    }
    shutdown_signal().await;
    {
        server::instance().lock().await.stop();
    }
    Ok(())
}
