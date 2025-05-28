use solana_block_monitor::{
    cache::BlockCache, config::Config, logic::SyndicaAppLogic, metrics::TracingMetrics,
    server::start_server, state::AppState, synchronizer::Synchronizer,
    syndica_client::SyndicaClient,
};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().await?;

    tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_max_level(config.get_tracing_level())
        .init();

    info!("Loaded configuration from .env file:");
    info!("  Solana RPC URL: {}", config.solana_rpc_url);
    info!("  Server Port: {}", config.server_port);
    info!("  Log Level: {}", config.log_level);
    info!("  Monitor Interval: {}ms", config.monitor_interval_ms);

    let cache = Arc::new(BlockCache::new(config.monitoring_depth));
    let client = Arc::new(SyndicaClient::new(
        config.solana_rpc_url.clone(),
        config.solana_rpc_key.clone(),
    ));
    let metrics = Arc::new(TracingMetrics::new());
    let state = Arc::new(AppState::new(
        cache.clone(),
        client.clone(),
        metrics.clone(),
    ));
    let logic: Arc<SyndicaAppLogic> = Arc::new(SyndicaAppLogic::new(state));

    let mut synchronizer = Synchronizer::new(
        logic.clone(),
        config.monitor_interval_ms,
        config.monitoring_depth,
    );

    let sync_handle = tokio::spawn(async move {
        synchronizer.run().await;
    });

    info!("Starting server on port {}", config.server_port);
    let server_handle = tokio::spawn(async move {
        if let Err(e) = start_server(config.server_port, logic).await {
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
