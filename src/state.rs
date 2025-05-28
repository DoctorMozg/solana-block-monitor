use std::sync::Arc;

use crate::cache::BlockCache;
use crate::metrics::Metrics;
use crate::syndica_client::SyndicaClient;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct AppState {
    cache: Arc<BlockCache>,
    client: Arc<SyndicaClient>,
    metrics: Arc<dyn Metrics + Send + Sync>,
    last_processed_slot: AtomicU64,
}

impl AppState {
    pub fn new(
        cache: Arc<BlockCache>,
        client: Arc<SyndicaClient>,
        metrics: Arc<dyn Metrics + Send + Sync>,
    ) -> Self {
        Self {
            cache,
            client,
            metrics,
            last_processed_slot: AtomicU64::new(0),
        }
    }

    pub fn cache(&self) -> &Arc<BlockCache> {
        &self.cache
    }

    pub fn client(&self) -> &Arc<SyndicaClient> {
        &self.client
    }

    pub fn metrics(&self) -> &Arc<dyn Metrics + Send + Sync> {
        &self.metrics
    }

    pub fn last_processed_slot(&self) -> u64 {
        self.last_processed_slot.load(Ordering::Relaxed)
    }

    pub fn set_last_processed_slot(&self, slot: u64) {
        self.last_processed_slot.store(slot, Ordering::Relaxed);
    }
}
