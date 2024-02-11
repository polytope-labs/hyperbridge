#[cfg(test)]
mod test;

use anyhow::anyhow;
use bnb_pos_verifier::primitives::{compute_epoch, parse_extra, BnbClientUpdate, EPOCH_LENGTH};
use ethers::{
    prelude::{Provider, Ws},
    providers::Middleware,
    types::BlockId,
};
use geth_primitives::CodecHeader;
use ismp::util::Keccak256;
use sp_core::H256;
use std::{fmt::Debug, sync::Arc};
use sync_committee_primitives::constants::BlsPublicKey;

#[derive(Clone)]
pub struct BnbPosProver {
    /// Execution Rpc client
    pub client: Arc<Provider<Ws>>,
}

impl BnbPosProver {
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
            .ok_or_else(|| anyhow!("Latest header block could not be fetched {block_number}"))?;
        Ok(header)
    }

    pub async fn fetch_bnb_update<I: Keccak256>(
        &self,
        attested_header: CodecHeader,
    ) -> Result<Option<BnbClientUpdate>, anyhow::Error> {
        let parse_extra_data = parse_extra::<I>(&attested_header.extra_data)
            .map_err(|_| anyhow!("Extra data not found in header {:?}", attested_header.number))?;
        let source_hash = H256::from_slice(&parse_extra_data.vote_data.source_hash.0);
        let target_hash = H256::from_slice(&parse_extra_data.vote_data.target_hash.0);

        if source_hash == Default::default() || target_hash == Default::default() {
            return Ok(None)
        }

        let source_header = self
            .fetch_header(source_hash)
            .await?
            .ok_or_else(|| anyhow!("header block could not be fetched {source_hash}"))?;
        let target_header = self
            .fetch_header(target_hash)
            .await?
            .ok_or_else(|| anyhow!("header block could not be fetched {target_hash}"))?;

        let source_header_epoch = compute_epoch(source_header.number.low_u64());
        let epoch_header_number = source_header_epoch * EPOCH_LENGTH;

        let mut epoch_header_ancestry = vec![];

        // If we are still in authority rotation period get the epoch header ancestry alongside
        // update
        let diff = source_header.number.low_u64().saturating_sub(epoch_header_number);
        // The maximum difference between the epoch header block number and the source header
        // number is 9 since authority set rotation happens after the first 12 blocks in an
        // epoch, we want to show that the epoch header is in the ancestry of our finalized
        // header
        if (1..=9).contains(&diff) {
            let mut header =
                self.fetch_header(source_header.parent_hash).await?.ok_or_else(|| {
                    anyhow!("header block could not be fetched {}", source_header.parent_hash)
                })?;
            epoch_header_ancestry.insert(0, header.clone());
            while header.number.low_u64() > epoch_header_number {
                header = self.fetch_header(header.parent_hash).await?.ok_or_else(|| {
                    anyhow!("header block could not be fetched {}", header.parent_hash)
                })?;
                epoch_header_ancestry.insert(0, header.clone());
            }
        }

        let bnb_client_update = BnbClientUpdate {
            source_header,
            target_header,
            attested_header,
            epoch_header_ancestry: epoch_header_ancestry.try_into().expect("Infallible: Qed"),
        };

        Ok(Some(bnb_client_update))
    }

    pub async fn fetch_finalized_state<I: Keccak256>(
        &self,
    ) -> Result<(CodecHeader, Vec<BlsPublicKey>), anyhow::Error> {
        let latest_header = self.latest_header().await?;

        let current_epoch = compute_epoch(latest_header.number.low_u64());
        let current_epoch_block_number = current_epoch * EPOCH_LENGTH;

        let current_epoch_header =
            self.fetch_header(current_epoch_block_number).await?.ok_or_else(|| {
                anyhow!("header block could not be fetched {current_epoch_block_number}")
            })?;
        let current_epoch_extra_data = parse_extra::<I>(&current_epoch_header.extra_data)
            .map_err(|_| anyhow!("Extra data set not found in header"))?;

        let current_validators = current_epoch_extra_data
            .validators
            .into_iter()
            .map(|val| val.bls_public_key.as_slice().try_into().expect("Infallible"))
            .collect::<Vec<BlsPublicKey>>();
        Ok((current_epoch_header, current_validators))
    }
}
