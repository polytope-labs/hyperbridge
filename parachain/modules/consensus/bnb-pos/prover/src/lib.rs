#[cfg(test)]
mod test;

use anyhow::anyhow;
use bnb_pos_verifier::{
    primitives::{parse_extra, CodecHeader, CodecVoteData},
    ValidatorData,
};
use ethers::{
    prelude::{Provider, Ws},
    providers::Middleware,
    types::BlockId,
};
use ismp::util::Keccak256;
use primitive_types::H160;
use sp_core::H256;
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

#[derive(Clone)]
pub struct BnbPosProver {
    /// Execution Rpc client
    pub client: Arc<Provider<Ws>>,
}

#[derive(Clone)]
pub struct BnbProof {
    pub agg_signature: [u8; 96],
    pub vote_data_hash: H256,
    pub vote_data: CodecVoteData,
    pub validator_set_size: u8,
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
            withdrawals_root: block.withdrawals_root,
            blob_gas_used: block.blob_gas_used,
            excess_blob_gas: block.excess_blob_gas,
        };

        Ok(header)
    }

    pub async fn latest_header(&self) -> Result<CodecHeader, anyhow::Error> {
        let block_number = self.client.get_block_number().await?;
        let header = self.fetch_header(block_number.as_u64()).await?;
        Ok(header)
    }

    pub async fn fetch_proofs_and_validators<I: Keccak256>(
        &self,
        header: CodecHeader,
    ) -> Result<(BnbProof, Option<Vec<ValidatorData>>), anyhow::Error> {
        let parse_extra_data = parse_extra::<I>(&header.extra_data)
            .map_err(|_| anyhow!("Extra data set not found in header"))?;

        let validator_data_vec: Option<Vec<ValidatorData>> = {
            let mut iter =
                parse_extra_data.validators.iter().map(|data| ValidatorData::from(data.clone()));
            if let Some(first) = iter.next() {
                Some(iter.collect())
            } else {
                None
            }
        };

        let bnb_proof = BnbProof {
            agg_signature: parse_extra_data.agg_signature,
            vote_data_hash: parse_extra_data.vote_data_hash,
            vote_data: parse_extra_data.vote_data,
            validator_set_size: parse_extra_data.validator_size,
        };

        Ok((bnb_proof, validator_data_vec))
    }
}
