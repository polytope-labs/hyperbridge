use anyhow::anyhow;
use ethers::{
    prelude::{Provider, Ws},
    providers::Middleware,
    types::BlockId,
};
use polygon_pos_verifier::primitives::CodecHeader;
use std::{fmt::Debug, sync::Arc};

#[cfg(test)]
mod test;

#[derive(Clone)]
pub struct PolygonPosProver {
    /// Execution Rpc client
    pub client: Arc<Provider<Ws>>,
}
const SPAN_LENGTH: u64 = 400 * 16;

impl PolygonPosProver {
    pub fn new(client: Provider<Ws>) -> Self {
        Self { client: Arc::new(client) }
    }

    pub async fn fetch_header<T: Into<BlockId> + Send + Sync + Debug + Copy>(
        &self,
        block: T,
    ) -> Result<CodecHeader, anyhow::Error> {
        let block = self
            .client
            .get_block(block)
            .await?
            .ok_or_else(|| anyhow!("Header not found for {:?}", block))?;
        let header = CodecHeader {
            parent_hash: block.parent_hash,
            uncle_hash: block.uncles_hash,
            coinbase: block.author.unwrap_or_default(),
            state_root: block.state_root,
            transactions_root: block.transactions_root,
            receipts_root: block.receipts_root,
            logs_bloom: block.logs_bloom.unwrap_or_default(),
            difficulty: block.difficulty,
            number: block.number.unwrap_or_default().as_u64().into(),
            gas_limit: block.gas_limit.low_u64(),
            gas_used: block.gas_used.low_u64(),
            timestamp: block.timestamp.low_u64(),
            extra_data: block.extra_data.0.into(),
            mix_hash: block.mix_hash.unwrap_or_default(),
            nonce: block.nonce.unwrap_or_default(),
            base_fee_per_gas: block.base_fee_per_gas,
            withdrawals_hash: block.withdrawals_root,
            excess_data_gas: block.excess_blob_gas,
        };

        Ok(header)
    }

    pub async fn latest_header(&self) -> Result<CodecHeader, anyhow::Error> {
        let block_number = self.client.get_block_number().await?;
        let header = self.fetch_header(block_number.as_u64()).await?;
        Ok(header)
    }

    // async fn create_verifier_state() -> Result<VerifierState, anyhow::Error> {

    // }
}

/// Returns a vector of mandatory block numbers
pub fn should_sync(previous_finalized: u64, latest_header: u64) -> Vec<u64> {
    let current_span = previous_finalized / SPAN_LENGTH;
    let next_span = latest_header / SPAN_LENGTH;

    ((current_span + 1)..=next_span)
        .into_iter()
        .map(|span: u64| span * SPAN_LENGTH)
        .collect()
}

pub fn is_span_start(block_number: u64) -> bool {
    block_number % SPAN_LENGTH == 0
}
