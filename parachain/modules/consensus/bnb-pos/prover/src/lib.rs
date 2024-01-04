#[cfg(test)]
mod test;

use anyhow::anyhow;
use bnb_pos_verifier::{
    primitives::{compute_epoch, parse_extra, BnbClientUpdate, EPOCH_LENGTH},
    NextValidators,
};
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
            withdrawals_hash: None,
        };

        Ok(header)
    }

    pub async fn latest_header(&self) -> Result<CodecHeader, anyhow::Error> {
        let block_number = self.client.get_block_number().await?;
        let header = self.fetch_header(block_number.as_u64()).await?;
        Ok(header)
    }

    pub async fn fetch_bnb_update<I: Keccak256>(
        &self,
        attested_header: CodecHeader,
    ) -> Result<BnbClientUpdate, anyhow::Error> {
        let parse_extra_data = parse_extra::<I>(&attested_header.extra_data)
            .map_err(|_| anyhow!("Extra data set not found in header"))?;

        let source_hash = H256::from_slice(&parse_extra_data.vote_data.source_hash.0);
        let target_hash = H256::from_slice(&parse_extra_data.vote_data.target_hash.0);

        let source_header = self.fetch_header(source_hash).await?;
        let target_header = self.fetch_header(target_hash).await?;

        let bnb_client_update = BnbClientUpdate { source_header, target_header, attested_header };

        Ok(bnb_client_update)
    }

    pub async fn fetch_finalized_state<I: Keccak256>(
        &self,
    ) -> Result<(CodecHeader, Vec<BlsPublicKey>, Option<NextValidators>), anyhow::Error> {
        let latest_header = self.latest_header().await?;

        let current_epoch = compute_epoch(latest_header.number.low_u64());
        let current_epoch_block_number = current_epoch * EPOCH_LENGTH;

        let current_epoch_header = self.fetch_header(current_epoch_block_number).await?;
        let current_epoch_extra_data = parse_extra::<I>(&current_epoch_header.extra_data)
            .map_err(|_| anyhow!("Extra data set not found in header"))?;

        let next_rotation_block_number =
            current_epoch_block_number + (current_epoch_extra_data.validator_size as u64 / 2);

        let current_validators;
        let next_validators;
        if latest_header.number.low_u64() >= next_rotation_block_number {
            current_validators = current_epoch_extra_data
                .validators
                .into_iter()
                .map(|val| val.bls_public_key.as_slice().try_into().expect("Infallible"))
                .collect::<Vec<BlsPublicKey>>();
            next_validators = None;
        } else {
            let previous_epoch_block_number = (current_epoch - 1) * EPOCH_LENGTH;

            let previous_epoch_header = self.fetch_header(previous_epoch_block_number).await?;

            let previous_epoch_extra_data = parse_extra::<I>(&previous_epoch_header.extra_data)
                .map_err(|_| anyhow!("Extra data set not found in header"))?;

            current_validators = previous_epoch_extra_data
                .validators
                .into_iter()
                .map(|val| val.bls_public_key.as_slice().try_into().expect("Infallible"))
                .collect::<Vec<BlsPublicKey>>();
            next_validators = Some(NextValidators {
                validators: current_epoch_extra_data
                    .validators
                    .into_iter()
                    .map(|val| val.bls_public_key.as_slice().try_into().expect("Infallible"))
                    .collect::<Vec<BlsPublicKey>>(),
                rotation_block: next_rotation_block_number,
            })
        }

        Ok((latest_header, current_validators, next_validators))
    }
}
