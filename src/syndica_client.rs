use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;

use crate::types::BoxError;

pub struct SyndicaClient {
    rpc_client: RpcClient,
}

impl SyndicaClient {
    pub fn new(rpc_url: String, key: String) -> Self {
        let connection_url = format!("{}/{}", rpc_url, key);
        let rpc_client =
            RpcClient::new_with_commitment(connection_url, CommitmentConfig::confirmed());
        Self { rpc_client }
    }
}

impl SyndicaClient {
    pub async fn get_slot(&self) -> Result<u64, BoxError> {
        let slot = self.rpc_client.get_slot().await?;
        Ok(slot)
    }

    pub async fn get_blocks(&self, start_slot: u64, end_slot: u64) -> Result<Vec<u64>, BoxError> {
        let blocks = self
            .rpc_client
            .get_blocks(start_slot, Some(end_slot))
            .await?;
        Ok(blocks)
    }
}
