use std::sync::Arc;

use crate::cache::BlockCache;
use crate::syndica_client::ApiClient;

pub struct AppState {
    pub cache: Arc<BlockCache>,
    pub client: Arc<dyn ApiClient + Send + Sync>,
}

impl AppState {
    pub fn new(cache: Arc<BlockCache>, client: Arc<dyn ApiClient + Send + Sync>) -> Self {
        Self { cache, client }
    }
}
