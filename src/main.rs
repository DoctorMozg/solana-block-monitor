use solana_block_monitor::{
    cache::BlockCache, config::Config, server::start_server, synchronizer::Synchronizer,
    syndica_client::SyndicaClient,
};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().await?;

    tracing_subscriber::fmt()
        .with_max_level(config.get_tracing_level())
        .init();

    info!("Loaded configuration from .env file:");
    info!("  Solana RPC URL: {}", config.solana_rpc_url);
    info!("  Server Port: {}", config.server_port);
    info!("  Log Level: {}", config.log_level);
    info!("  Monitor Interval: {}ms", config.monitor_interval_ms);

    let cache = Arc::new(BlockCache::default());
    let client = Arc::new(SyndicaClient::new(config.solana_rpc_url.clone()));

    let mut synchronizer =
        Synchronizer::new(cache.clone(), client.clone(), config.monitor_interval_ms);

    let sync_handle = tokio::spawn(async move {
        synchronizer.start().await;
    });

    info!("Starting server on port {}", config.server_port);
    let server_handle = tokio::spawn(async move {
        if let Err(e) = start_server(config.server_port, cache, client).await {
            tracing::error!("Server error: {}", e);
        }
    });

    tokio::select! {
        _ = sync_handle => {
            tracing::error!("Synchronizer task ended unexpectedly");
        }
        _ = server_handle => {
            tracing::error!("Server task ended unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    Ok(())
}
