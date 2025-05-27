use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

type ClientError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait::async_trait]
pub trait ApiClient {
    async fn get_block(&self) -> Result<u64, ClientError>;
    async fn get_blocks(&self, start_slot: u64, end_slot: u64) -> Result<Vec<u64>, ClientError>;
}

pub struct SyndicaClient {
    rpc_client: Arc<RpcClient>,
}

impl SyndicaClient {
    pub fn new(rpc_url: String) -> Self {
        let rpc_client = RpcClient::new(rpc_url);
        Self {
            rpc_client: Arc::new(rpc_client),
        }
    }
}

#[async_trait::async_trait]
impl ApiClient for SyndicaClient {
    async fn get_block(&self) -> Result<u64, ClientError> {
        let slot = self.rpc_client.get_slot().await?;
        Ok(slot)
    }

    async fn get_blocks(&self, start_slot: u64, end_slot: u64) -> Result<Vec<u64>, ClientError> {
        let blocks = self
            .rpc_client
            .get_blocks(start_slot, Some(end_slot))
            .await?;
        Ok(blocks)
    }
}
