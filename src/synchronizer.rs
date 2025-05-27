use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval};
use tracing::{debug, error, info, warn};

use crate::cache::BlockCache;
use crate::syndica_client::ApiClient;

pub struct Synchronizer {
    cache: Arc<BlockCache>,
    client: Arc<dyn ApiClient + Send + Sync>,
    monitor_interval: Duration,
    last_processed_slot: Option<u64>,
}

impl Synchronizer {
    pub fn new(
        cache: Arc<BlockCache>,
        client: Arc<dyn ApiClient + Send + Sync>,
        monitor_interval_ms: u64,
    ) -> Self {
        Self {
            cache,
            client,
            monitor_interval: Duration::from_millis(monitor_interval_ms),
            last_processed_slot: None,
        }
    }

    pub async fn start(&mut self) {
        info!("Starting block synchronizer");

        let mut interval_timer = interval(self.monitor_interval);

        loop {
            interval_timer.tick().await;

            if self.last_processed_slot.is_none() {
                if let Err(e) = self.initialize_starting_slot().await {
                    error!("Failed to initialize starting slot: {}", e);
                }
            }

            if let Err(e) = self.sync_blocks().await {
                error!("Error during block synchronization: {}", e);
            }
        }
    }

    async fn initialize_starting_slot(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let current_slot = self.client.get_block().await?;

        let start_slot = if current_slot >= 100 {
            current_slot - 100
        } else {
            0
        };

        self.last_processed_slot = Some(start_slot);
        info!(
            current_slot,
            start_slot, "Initialized synchronizer starting from slot"
        );

        Ok(())
    }

    async fn sync_blocks(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.last_processed_slot.is_none() {
            return Ok(());
        }

        let current_slot = self.client.get_block().await?;
        let last_slot = self.last_processed_slot.unwrap_or(0);

        if current_slot <= last_slot {
            debug!(current_slot, last_slot, "No new slots to process");
            return Ok(());
        }

        let batch_size = 100;
        let mut start_slot = last_slot + 1;

        while start_slot <= current_slot {
            let end_slot = std::cmp::min(start_slot + batch_size - 1, current_slot);

            match self.process_slot_range(start_slot, end_slot).await {
                Ok(confirmed_count) => {
                    debug!(
                        start_slot,
                        end_slot, confirmed_count, "Processed slot range"
                    );
                    self.last_processed_slot = Some(end_slot);
                    start_slot = end_slot + 1;
                }
                Err(e) => {
                    warn!(
                        start_slot,
                        end_slot,
                        error = %e,
                        "Failed to process slot range, will retry"
                    );
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    async fn process_slot_range(
        &self,
        start_slot: u64,
        end_slot: u64,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let confirmed_blocks = self.client.get_blocks(start_slot, end_slot).await?;

        let mut inserted_count = 0;
        for block_slot in confirmed_blocks {
            if !self.cache.contains(block_slot) &&
                self.cache.insert(block_slot) {
                inserted_count += 1;
            }
        }

        if inserted_count > 0 {
            info!(
                start_slot,
                end_slot,
                inserted_count,
                cache_size = self.cache.len(),
                "Added new confirmed blocks to cache"
            );
        }

        Ok(inserted_count)
    }

    pub fn get_last_processed_slot(&self) -> Option<u64> {
        self.last_processed_slot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::BlockCache;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockClient {
        current_slot: Arc<Mutex<u64>>,
        confirmed_blocks: Arc<Mutex<HashMap<(u64, u64), Vec<u64>>>>,
    }

    impl MockClient {
        fn new() -> Self {
            Self {
                current_slot: Arc::new(Mutex::new(1000)),
                confirmed_blocks: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn set_current_slot(&self, slot: u64) {
            *self.current_slot.lock().unwrap() = slot;
        }

        fn set_confirmed_blocks(&self, start: u64, end: u64, blocks: Vec<u64>) {
            self.confirmed_blocks
                .lock()
                .unwrap()
                .insert((start, end), blocks);
        }
    }

    #[async_trait]
    impl ApiClient for MockClient {
        async fn get_block(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
            Ok(*self.current_slot.lock().unwrap())
        }

        async fn get_blocks(
            &self,
            start_slot: u64,
            end_slot: u64,
        ) -> Result<Vec<u64>, Box<dyn std::error::Error + Send + Sync>> {
            let blocks = self.confirmed_blocks.lock().unwrap();
            Ok(blocks
                .get(&(start_slot, end_slot))
                .cloned()
                .unwrap_or_default())
        }
    }

    #[tokio::test]
    async fn test_synchronizer_initialization() {
        let cache = Arc::new(BlockCache::new(100));
        let client = Arc::new(MockClient::new());
        client.set_current_slot(1000);

        let mut sync = Synchronizer::new(cache, client, 100);
        sync.initialize_starting_slot().await.unwrap();

        assert_eq!(sync.get_last_processed_slot(), Some(900));
    }

    #[tokio::test]
    async fn test_process_slot_range() {
        let cache = Arc::new(BlockCache::new(100));
        let client = Arc::new(MockClient::new());

        client.set_confirmed_blocks(100, 105, vec![100, 102, 104]);

        let sync = Synchronizer::new(cache.clone(), client, 100);
        let result = sync.process_slot_range(100, 105).await.unwrap();

        assert_eq!(result, 3);
        assert!(cache.contains(100));
        assert!(cache.contains(102));
        assert!(cache.contains(104));
        assert!(!cache.contains(101));
        assert!(!cache.contains(103));
    }
}
