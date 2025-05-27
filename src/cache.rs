use scc::HashCache;
use tracing::{debug, info, warn};

pub struct BlockCache {
    cache: HashCache<u64, ()>,
}

impl BlockCache {
    pub fn new(capacity: usize) -> Self {
        let cache = HashCache::with_capacity(capacity, capacity * 2);
        info!(capacity, "Created block cache");

        Self { cache }
    }

    pub fn contains(&self, block_number: u64) -> bool {
        let exists = self.cache.get(&block_number).is_some();
        debug!(block_number, exists, "Checked block in cache");
        exists
    }

    pub fn insert(&self, block_number: u64) -> bool {
        match self.cache.put(block_number, ()) {
            Ok(_) => {
                debug!(block_number, "Inserted block into cache");
                true
            }
            Err(_) => {
                warn!(block_number, "Failed to insert block into cache");
                false
            }
        }
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.cache.capacity()
    }

    pub fn clear(&self) {
        self.cache.clear();
        info!("Cleared block cache");
    }
}

impl Default for BlockCache {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache = BlockCache::new(3);

        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        assert!(cache.insert(1));
        assert!(cache.insert(2));
        assert!(cache.insert(3));

        assert!(cache.contains(1));
        assert!(cache.contains(2));
        assert!(cache.contains(3));
        assert!(!cache.contains(4));

        assert_eq!(cache.len(), 3);
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_cache_capacity() {
        let cache = BlockCache::new(1000);
        assert_eq!(cache.capacity(), 1024);

        cache.insert(1);
        cache.insert(2);
        cache.insert(3);

        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_cache_clear() {
        let cache = BlockCache::new(5);

        cache.insert(1);
        cache.insert(2);
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_lru_behavior() {
        let cache = BlockCache::new(2);

        cache.insert(1);
        cache.insert(2);

        cache.contains(1);

        cache.insert(3);

        assert!(cache.contains(1));
        assert!(cache.contains(3));
    }
}
