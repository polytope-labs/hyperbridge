use std::marker::PhantomData;

use anyhow::anyhow;
use beacon_kit_verifier::primitives::{BeaconKitUpdate, Config, ValidatorSetProof};
use primitive_types::H256;
use reqwest::{Client, Url};
use ssz_rs::Merkleized;
use sync_committee_primitives::{
    consensus_types::{BeaconBlockHeader, BeaconState},
    constants::{
        BlsPublicKey, BlsSignature, BYTES_PER_LOGS_BLOOM, EPOCHS_PER_HISTORICAL_VECTOR,
        EPOCHS_PER_SLASHINGS_VECTOR, HISTORICAL_ROOTS_LIMIT, MAX_EXTRA_DATA_BYTES,
        PENDING_CONSOLIDATIONS_LIMIT, PENDING_DEPOSITS_LIMIT, PENDING_PARTIAL_WITHDRAWALS_LIMIT,
        SLOTS_PER_HISTORICAL_ROOT, SYNC_COMMITTEE_SIZE, VALIDATOR_REGISTRY_LIMIT,
    },
};
use tracing::{instrument, trace};

use crate::routes::*;

pub mod responses;
pub mod routes;

#[cfg(test)]
mod test;

const SLOTS_PER_EPOCH: u64 = 32;

pub type BeaconStateType<const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize> =
BeaconState<
    SLOTS_PER_HISTORICAL_ROOT,
    HISTORICAL_ROOTS_LIMIT,
    ETH1_DATA_VOTES_BOUND,
    VALIDATOR_REGISTRY_LIMIT,
    EPOCHS_PER_HISTORICAL_VECTOR,
    EPOCHS_PER_SLASHINGS_VECTOR,
    SYNC_COMMITTEE_SIZE,
    BYTES_PER_LOGS_BLOOM,
    MAX_EXTRA_DATA_BYTES,
    PENDING_DEPOSITS_LIMIT,
    PENDING_CONSOLIDATIONS_LIMIT,
    PENDING_PARTIAL_WITHDRAWALS_LIMIT,
    PROPOSER_LOOK_AHEAD_LIMIT,
>;

pub struct BeaconKitProver<C: Config> {
    pub primary_url: String,
    pub client: Client,
    pub phantom: PhantomData<C>,
}

impl<C: Config> Clone for BeaconKitProver<C> {
    fn clone(&self) -> Self {
        Self {
            primary_url: self.primary_url.clone(),
            client: self.client.clone(),
            phantom: PhantomData,
        }
    }
}

impl<C: Config> BeaconKitProver<C> {
    pub fn new(primary_url: String) -> Self {
        let client = Client::new();

        Self {
            primary_url,
            client,
            phantom: PhantomData,
        }
    }

    fn generate_route(&self, path: &str) -> Result<Url, anyhow::Error> {
        Url::parse(&format!("{}{}", self.primary_url, path)).map_err(Into::into)
    }

    #[instrument(skip(self))]
    pub async fn fetch_header(&self, block_id: &str) -> Result<BeaconBlockHeader, anyhow::Error> {
        let path = header_route(block_id);
        let url = self.generate_route(&path)?;

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch header: {:?}", e))?;

        let data = response
            .json::<responses::beacon_block_header_response::Response>()
            .await
            .map_err(|e| anyhow!("Failed to parse header response: {:?}", e))?;

        Ok(data.data.header.message)
    }

    #[instrument(skip(self))]
    pub async fn fetch_beacon_state<const ETH1_VOTES: usize, const PROPOSER_LIMIT: usize>(
        &self,
        state_id: &str,
    ) -> Result<BeaconStateType<ETH1_VOTES, PROPOSER_LIMIT>, anyhow::Error> {
        let path = beacon_state_route(state_id);
        let url = self.generate_route(&path)?;

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch state: {:?}", e))?;

        let data = response
            .json::<responses::beacon_state_response::Response<ETH1_VOTES, PROPOSER_LIMIT>>()
            .await
            .map_err(|e| anyhow!("Failed to parse state response: {:?}", e))?;

        Ok(data.data)
    }

    #[instrument(skip(self))]
    pub async fn fetch_initial_state<const ETH1_VOTES: usize, const PROPOSER_LIMIT: usize>(
        &self,
        block_id: &str,
    ) -> Result<(BeaconBlockHeader, Vec<BlsPublicKey>), anyhow::Error> {
        let header = self.fetch_header(block_id).await?;

        let state = self
            .fetch_beacon_state::<ETH1_VOTES, PROPOSER_LIMIT>(&header.state_root.to_string())
            .await?;

        let current_validators: Vec<BlsPublicKey> = state
            .validators
            .iter()
            .map(|v| v.public_key.clone())
            .collect();

        Ok((header, current_validators))
    }

    #[instrument(skip(self))]
    pub async fn fetch_light_client_update<const ETH1_VOTES: usize, const PROPOSER_LIMIT: usize>(
        &self,
        block_id: &str,
        trusted_header: BeaconBlockHeader,
    ) -> Result<BeaconKitUpdate, anyhow::Error> {
        let header = self.fetch_header(block_id).await?;
        let current_epoch = header.slot / SLOTS_PER_EPOCH;
        let last_finalized_epoch = trusted_header.slot / SLOTS_PER_EPOCH;

        let mut state = self
            .fetch_beacon_state::<ETH1_VOTES, PROPOSER_LIMIT>(&header.state_root.to_string())
            .await?;

        let execution_payload_proof =
            ssz_rs::generate_proof(&mut state, &[C::EXECUTION_PAYLOAD_INDEX])?;

        let validator_set_proof = if current_epoch > last_finalized_epoch {
            trace!("Epoch changed ({} > {}), fetching validator proof", current_epoch, last_finalized_epoch);
            let proof = ssz_rs::generate_proof(&mut state, &[C::VALIDATOR_REGISTRY_INDEX])?;
            let validators = state
                .validators
                .iter()
                .map(|v| v.public_key.clone())
                .collect();

            Some(ValidatorSetProof {
                validators,
                proof: proof
                    .into_iter()
                    .map(|n| H256::from_slice(n.as_ref()))
                    .collect(),
            })
        } else {
            None
        };

        let (signature, signers) = self.fetch_consensus_data(block_id).await?;

        Ok(BeaconKitUpdate {
            beacon_header: header.clone(),
            signature,
            signers,
            execution_payload: state.latest_execution_payload_header,
            execution_payload_proof: execution_payload_proof
                .into_iter()
                .map(|n| H256::from_slice(n.as_ref()))
                .collect(),
            validator_set_proof,
        })
    }

    async fn fetch_consensus_data(
        &self,
        block_id: &str,
    ) -> Result<(BlsSignature, Vec<BlsPublicKey>), anyhow::Error> {
        todo!("fetch from cometbft")
    }
}