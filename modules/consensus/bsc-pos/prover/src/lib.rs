#[cfg(test)]
mod test;

use anyhow::anyhow;
use bsc_pos_verifier::primitives::{compute_epoch, parse_extra, BscClientUpdate, EPOCH_LENGTH};
use ethers::{
    prelude::Provider,
    providers::{Http, Middleware},
    types::BlockId,
};
use geth_primitives::CodecHeader;
use ismp::util::Keccak256;
use sp_core::H256;
use std::{fmt::Debug, sync::Arc};
use sync_committee_primitives::constants::BlsPublicKey;
use tracing::{instrument, trace};

#[derive(Clone)]
pub struct BscPosProver {
    /// Execution Rpc client
    pub client: Arc<Provider<Http>>,
}

impl BscPosProver {
    pub fn new(client: Provider<Http>) -> Self {
        Self { client: Arc::new(client) }
    }

    pub async fn fetch_header<T: Into<BlockId> + Send + Sync + Debug + Copy>(
        &self,
        block: T,
    ) -> Result<Option<CodecHeader>, anyhow::Error> {
        let block = self.client.get_block(block).await?.map(|header| header.into());

        Ok(block)
    }

    #[instrument(level = "trace", target = "bsc-prover", skip(self))]
    pub async fn latest_header(&self) -> Result<CodecHeader, anyhow::Error> {
        trace!(target: "bsc-prover", "fetching latest header");
        let block_number = self.client.get_block_number().await?;
        let header = self
            .fetch_header(block_number.as_u64())
            .await?
            .ok_or_else(|| anyhow!("Latest header block could not be fetched {block_number}"))?;
        Ok(header)
    }

    #[instrument(level = "trace", target = "bsc-prover", skip_all)]
    pub async fn fetch_bsc_update<I: Keccak256>(
        &self,
        attested_header: CodecHeader,
        validator_size: u64,
    ) -> Result<Option<BscClientUpdate>, anyhow::Error> {
        trace!(target: "bsc-prover", "fetching bsc update for  {:?}", attested_header.number);
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
        // update only if the finalized header is not the epoch block
        let rotation_block = get_rotation_block(epoch_header_number, validator_size) - 1;
        if attested_header.number.low_u64() >= epoch_header_number + 2 &&
            attested_header.number.low_u64() <= rotation_block &&
            source_header.number.low_u64() > epoch_header_number
        {
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

        let bsc_client_update = BscClientUpdate {
            source_header,
            target_header,
            attested_header,
            epoch_header_ancestry: epoch_header_ancestry.try_into().expect("Infallible: Qed"),
        };

        Ok(Some(bsc_client_update))
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

// Get the maximum block that can be signed by previous validator set before authority set rotation
// occurs Validator set change happens at
// block%EPOCH_LENGTH == validator_size / 2
pub fn get_rotation_block(mut block: u64, validator_size: u64) -> u64 {
    loop {
        if block % EPOCH_LENGTH == (validator_size / 2) {
            break
        }
        block += 1
    }

    block
}
