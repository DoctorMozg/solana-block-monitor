use crate::state::AppState;
use crate::types::BoxError;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Business logic layer for the Syndica application.
///
/// This struct encapsulates the core business logic of the application, handling:
/// - Block and slot management
/// - Cache interactions
/// - Metrics recording
///
/// While web applications typically keep logic stateless, this application
/// maintains state for:
/// - Access to shared resources (client, cache, metrics)
/// - Performance optimization through caching
/// - Consistent metrics collection
///
/// The logic layer is the ideal place for metrics as it:
/// - Has direct access to operation results
/// - Can measure operation timing
/// - Can track cache effectiveness
/// - Provides a single point for all business metrics
pub struct SyndicaAppLogic {
    state: Arc<AppState>,
}

impl SyndicaAppLogic {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

impl SyndicaAppLogic {
    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }

    pub async fn get_latest_slot(&self) -> Result<u64, BoxError> {
        let result = self.state.client().get_slot().await;

        match &result {
            Ok(slot) => {
                self.state.metrics().record_latest_slot(*slot);
                debug!(slot = *slot, "Retrieved latest slot");
            }
            Err(e) => {
                warn!(error = %e, "Failed to get latest slot");
            }
        }
        result
    }

    pub async fn get_block(&self, slot: u64) -> Result<Option<u64>, BoxError> {
        if self.state.cache().contains(slot) {
            self.state.metrics().record_cache_hit(true);
            return Ok(Some(slot));
        }
        self.state.metrics().record_cache_hit(false);

        let start_time = Instant::now();
        let blocks = self.state.client().get_blocks(slot, slot).await?;
        self.state
            .metrics()
            .record_get_blocks_elapsed(start_time.elapsed());

        if blocks.contains(&slot) {
            self.state.cache().insert(slot);
            Ok(Some(slot))
        } else {
            Ok(None)
        }
    }

    pub async fn get_blocks(&self, start_slot: u64, end_slot: u64) -> Result<Vec<u64>, BoxError> {
        let start_time = Instant::now();
        let result = self.state.client().get_blocks(start_slot, end_slot).await;
        let elapsed = start_time.elapsed();

        self.state.metrics().record_get_blocks_elapsed(elapsed);

        match &result {
            Ok(blocks) => {
                debug!(
                    start_slot,
                    end_slot,
                    block_count = blocks.len(),
                    elapsed_ms = elapsed.as_millis(),
                    "Retrieved blocks range"
                );
            }
            Err(e) => {
                warn!(
                    start_slot,
                    end_slot,
                    elapsed_ms = elapsed.as_millis(),
                    error = %e,
                    "Failed to get blocks range"
                );
            }
        }

        result
    }

    pub async fn update_latest_slot(&self) -> Result<u64, BoxError> {
        let current_slot = self.get_latest_slot().await?;
        self.state.set_last_processed_slot(current_slot);

        info!(current_slot, "Initialized synchronizer starting from slot");

        Ok(current_slot)
    }

    pub async fn query_slot_range(
        &self,
        start_slot: u64,
        end_slot: u64,
    ) -> Result<usize, BoxError> {
        let confirmed_blocks = self.get_blocks(start_slot, end_slot).await?;

        let mut inserted_count = 0;
        for block_slot in confirmed_blocks {
            if !self.state.cache().contains(block_slot) && self.state.cache().insert(block_slot) {
                inserted_count += 1;
            }
        }

        if inserted_count > 0 {
            info!(
                start_slot,
                end_slot,
                inserted_count,
                cache_size = self.state.cache().len(),
                "Added confirmed blocks to cache"
            );
        }

        Ok(inserted_count)
    }
}
