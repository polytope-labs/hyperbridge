use anyhow::anyhow;
use ethers::{
    prelude::{Provider, Ws},
    providers::Middleware,
    types::BlockId,
};
use geth_primitives::CodecHeader;
use polygon_pos_verifier::primitives::{parse_validators, SPAN_LENGTH};
use primitive_types::H160;
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

#[cfg(test)]
mod test;

#[derive(Clone)]
pub struct PolygonPosProver {
    /// Execution Rpc client
    pub client: Arc<Provider<Ws>>,
}

impl PolygonPosProver {
    pub fn new(client: Provider<Ws>) -> Self {
        Self { client: Arc::new(client) }
    }

    pub async fn fetch_header<T: Into<BlockId> + Send + Sync + Debug + Copy>(
        &self,
        block: T,
    ) -> Result<Option<CodecHeader>, anyhow::Error> {
        let block = self.client.get_block(block).await?.map(|header| header.into());

        Ok(block)
    }

    pub async fn latest_header(&self) -> Result<CodecHeader, anyhow::Error> {
        let block_number = self.client.get_block_number().await?;
        let header = self
            .fetch_header(block_number.as_u64())
            .await?
            .ok_or_else(|| anyhow!("Header not found for {block_number:?}"))?;
        Ok(header)
    }

    pub async fn fetch_finalized_state(
        &self,
    ) -> Result<(CodecHeader, BTreeSet<H160>), anyhow::Error> {
        let latest_header = self.latest_header().await?;
        let finalized_block = latest_header.number.low_u64() - 250;
        let span = finalized_block / SPAN_LENGTH;
        let span_start = span * SPAN_LENGTH;
        let span_begin_header = self
            .fetch_header(span_start - 1)
            .await?
            .ok_or_else(|| anyhow!("Header not found for {:?}", span_start - 1))?;
        let validators = parse_validators(&span_begin_header.extra_data)?
            .ok_or_else(|| anyhow!("Validator set not found in span header"))?;
        let finalized_header = self
            .fetch_header(finalized_block)
            .await?
            .ok_or_else(|| anyhow!("Header not found for {finalized_block:?}"))?;
        Ok((finalized_header, validators))
    }
}

pub fn is_span_start(block_number: u64) -> bool {
    block_number % SPAN_LENGTH == 0
}
