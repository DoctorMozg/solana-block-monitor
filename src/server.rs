use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::cache::BlockCache;
use crate::state::AppState;
use crate::syndica_client::ApiClient;

pub async fn is_slot_confirmed(
    Path(slot): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    debug!(slot, "Checking if slot is confirmed");

    if state.cache.contains(slot) {
        debug!(slot, "Slot found in cache");
        return Ok(StatusCode::OK);
    }

    debug!(slot, "Slot not in cache, checking via RPC");

    match state.client.get_blocks(slot, slot).await {
        Ok(blocks) => {
            if blocks.contains(&slot) {
                debug!(slot, "Slot confirmed via RPC, adding to cache");
                state.cache.insert(slot);
                Ok(StatusCode::OK)
            } else {
                debug!(slot, "Slot not confirmed");
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            error!(slot, error = %e, "Failed to check slot via RPC");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/isSlotConfirmed/:slot", get(is_slot_confirmed))
        .with_state(state)
}

pub async fn start_server(
    port: u16,
    cache: Arc<BlockCache>,
    client: Arc<dyn ApiClient + Send + Sync>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState::new(cache, client));
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!(port, "Server starting");

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Path, State};
    use axum::http::StatusCode;
    use mockall::mock;
    use std::sync::Arc;

    mock! {
        TestClient {}

        #[async_trait::async_trait]
        impl ApiClient for TestClient {
            async fn get_block(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
            async fn get_blocks(&self, start_slot: u64, end_slot: u64) -> Result<Vec<u64>, Box<dyn std::error::Error + Send + Sync>>;
        }
    }

    #[tokio::test]
    async fn test_slot_confirmed_cache_hit() {
        let cache = Arc::new(BlockCache::new(10));
        cache.insert(12345);

        let mut mock_client = MockTestClient::new();
        mock_client.expect_get_blocks().never();

        let state = Arc::new(AppState::new(cache, Arc::new(mock_client)));

        let result = is_slot_confirmed(Path(12345), State(state)).await;

        assert_eq!(result, Ok(StatusCode::OK));
    }

    #[tokio::test]
    async fn test_slot_confirmed_cache_miss_confirmed() {
        let cache = Arc::new(BlockCache::new(10));

        let mut mock_client = MockTestClient::new();
        mock_client
            .expect_get_blocks()
            .with(mockall::predicate::eq(12345), mockall::predicate::eq(12345))
            .times(1)
            .returning(|_, _| Ok(vec![12345]));

        let state = Arc::new(AppState::new(cache.clone(), Arc::new(mock_client)));

        let result = is_slot_confirmed(Path(12345), State(state)).await;

        assert_eq!(result, Ok(StatusCode::OK));
        assert!(cache.contains(12345));
    }

    #[tokio::test]
    async fn test_slot_not_confirmed() {
        let cache = Arc::new(BlockCache::new(10));

        let mut mock_client = MockTestClient::new();
        mock_client
            .expect_get_blocks()
            .with(mockall::predicate::eq(12345), mockall::predicate::eq(12345))
            .times(1)
            .returning(|_, _| Ok(vec![]));

        let state = Arc::new(AppState::new(cache.clone(), Arc::new(mock_client)));

        let result = is_slot_confirmed(Path(12345), State(state)).await;

        assert_eq!(result, Err(StatusCode::NOT_FOUND));
        assert!(!cache.contains(12345));
    }

    #[tokio::test]
    async fn test_rpc_error() {
        let cache = Arc::new(BlockCache::new(10));

        let mut mock_client = MockTestClient::new();
        mock_client
            .expect_get_blocks()
            .with(mockall::predicate::eq(12345), mockall::predicate::eq(12345))
            .times(1)
            .returning(|_, _| Err("RPC error".into()));

        let state = Arc::new(AppState::new(cache, Arc::new(mock_client)));

        let result = is_slot_confirmed(Path(12345), State(state)).await;

        assert_eq!(result, Err(StatusCode::INTERNAL_SERVER_ERROR));
    }
}
