mod error;
mod responses;
mod routes;
#[cfg(test)]
mod test;

use ethereum_consensus::altair::Validator;
use ethereum_consensus::bellatrix::{
    BeaconBlock, BeaconBlockBody, BeaconBlockHeader, BeaconState, Checkpoint, SignedBeaconBlock,
    SignedBeaconBlockHeader, SyncCommittee,
};
use reqwest::{Client, StatusCode};

use crate::error::Error;
use crate::responses::sync_committee_response::NodeSyncCommittee;
use crate::routes::*;
use ethereum_consensus::bellatrix::mainnet::{
    BYTES_PER_LOGS_BLOOM, MAX_BYTES_PER_TRANSACTION, MAX_EXTRA_DATA_BYTES,
    MAX_TRANSACTIONS_PER_PAYLOAD, SYNC_COMMITTEE_SIZE,
};
use ethereum_consensus::crypto::{aggregate, eth_aggregate_public_keys, PublicKey};
use ethereum_consensus::phase0::mainnet::{
    EPOCHS_PER_HISTORICAL_VECTOR, EPOCHS_PER_SLASHINGS_VECTOR, ETH1_DATA_VOTES_BOUND,
    HISTORICAL_ROOTS_LIMIT, MAX_ATTESTATIONS, MAX_ATTESTER_SLASHINGS, MAX_DEPOSITS,
    MAX_PROPOSER_SLASHINGS, MAX_VALIDATORS_PER_COMMITTEE, MAX_VOLUNTARY_EXITS,
    SLOTS_PER_HISTORICAL_ROOT, VALIDATOR_REGISTRY_LIMIT,
};
use ethereum_consensus::primitives::{BlsPublicKey, Bytes32, Hash32, Slot, ValidatorIndex};
use light_client_primitives::types::{
    BlockRootsProof, ExecutionPayloadProof, EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX,
    EXECUTION_PAYLOAD_INDEX, EXECUTION_PAYLOAD_STATE_ROOT_INDEX, FINALIZED_ROOT_INDEX,
    NEXT_SYNC_COMMITTEE_INDEX,
};
use light_client_primitives::util::get_subtree_index;
use ssz_rs::{Bitlist, List, Node, Vector};

type BeaconBlockType = BeaconBlock<
    MAX_PROPOSER_SLASHINGS,
    MAX_VALIDATORS_PER_COMMITTEE,
    MAX_ATTESTER_SLASHINGS,
    MAX_ATTESTATIONS,
    MAX_DEPOSITS,
    MAX_VOLUNTARY_EXITS,
    SYNC_COMMITTEE_SIZE,
    BYTES_PER_LOGS_BLOOM,
    MAX_EXTRA_DATA_BYTES,
    MAX_BYTES_PER_TRANSACTION,
    MAX_TRANSACTIONS_PER_PAYLOAD,
>;

type SignedBeaconBlockType = SignedBeaconBlock<
    MAX_PROPOSER_SLASHINGS,
    MAX_VALIDATORS_PER_COMMITTEE,
    MAX_ATTESTER_SLASHINGS,
    MAX_ATTESTATIONS,
    MAX_DEPOSITS,
    MAX_VOLUNTARY_EXITS,
    SYNC_COMMITTEE_SIZE,
    BYTES_PER_LOGS_BLOOM,
    MAX_EXTRA_DATA_BYTES,
    MAX_BYTES_PER_TRANSACTION,
    MAX_TRANSACTIONS_PER_PAYLOAD,
>;

pub type BeaconStateType = BeaconState<
    SLOTS_PER_HISTORICAL_ROOT,
    HISTORICAL_ROOTS_LIMIT,
    ETH1_DATA_VOTES_BOUND,
    VALIDATOR_REGISTRY_LIMIT,
    EPOCHS_PER_HISTORICAL_VECTOR,
    EPOCHS_PER_SLASHINGS_VECTOR,
    MAX_VALIDATORS_PER_COMMITTEE,
    SYNC_COMMITTEE_SIZE,
    BYTES_PER_LOGS_BLOOM,
    MAX_EXTRA_DATA_BYTES,
    MAX_BYTES_PER_TRANSACTION,
    MAX_TRANSACTIONS_PER_PAYLOAD,
>;

pub struct SyncCommitteeProver {
    pub node_url: String,
    pub client: Client,
}

impl SyncCommitteeProver {
    pub fn new(node_url: String) -> Self {
        let client = reqwest::Client::new();

        SyncCommitteeProver { node_url, client }
    }

    pub async fn fetch_finalized_checkpoint(&self) -> Result<Checkpoint, reqwest::Error> {
        let full_url = self.generate_route(&finality_checkpoints("head"));
        let response = self.client.get(full_url).send().await?;

        let response_data = response
            .json::<responses::finality_checkpoint_response::Response>()
            .await?;
        Ok(response_data.data.finalized)
    }

    pub async fn fetch_header(&self, block_id: &str) -> Result<BeaconBlockHeader, reqwest::Error> {
        let path = header_route(block_id);
        let full_url = self.generate_route(&path);
        let response = self.client.get(full_url).send().await?;
        let status = response.status().as_u16();

        let response_data = response
            .json::<responses::beacon_block_header_response::Response>()
            .await?;

        let beacon_block_header = response_data.data.header.message;

        Ok(beacon_block_header)
    }
    pub async fn fetch_block(
        &self,
        block_id: &str,
    ) -> Result<
        BeaconBlock<
            MAX_PROPOSER_SLASHINGS,
            MAX_VALIDATORS_PER_COMMITTEE,
            MAX_ATTESTER_SLASHINGS,
            MAX_ATTESTATIONS,
            MAX_DEPOSITS,
            MAX_VOLUNTARY_EXITS,
            SYNC_COMMITTEE_SIZE,
            BYTES_PER_LOGS_BLOOM,
            MAX_EXTRA_DATA_BYTES,
            MAX_BYTES_PER_TRANSACTION,
            MAX_TRANSACTIONS_PER_PAYLOAD,
        >,
        reqwest::Error,
    > {
        let path = block_route(block_id);
        let full_url = self.generate_route(&path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response
            .json::<responses::beacon_block_response::Response>()
            .await?;

        let beacon_block = response_data.data.message;

        Ok(beacon_block)
    }
    pub async fn fetch_sync_committee(
        &self,
        state_id: &str,
    ) -> Result<NodeSyncCommittee, reqwest::Error> {
        let path = sync_committee_route(state_id);
        let full_url = self.generate_route(&path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response
            .json::<responses::sync_committee_response::Response>()
            .await?;

        let sync_committee = response_data.data;

        Ok(sync_committee)
    }
    pub async fn fetch_validator(
        &self,
        state_id: &str,
        validator_index: &str,
    ) -> Result<Validator, reqwest::Error> {
        let path = validator_route(state_id, validator_index);
        let full_url = self.generate_route(&path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response
            .json::<responses::validator_response::Response>()
            .await?;

        let validator = response_data.data.validator;

        Ok(validator)
    }

    pub async fn fetch_beacon_state(
        &self,
        state_id: &str,
    ) -> Result<BeaconStateType, reqwest::Error> {
        let path = beacon_state_route(state_id);
        let full_url = self.generate_route(&path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response
            .json::<responses::beacon_state_response::Response>()
            .await?;

        let beacon_state = response_data.data;

        Ok(beacon_state)
    }

    pub async fn fetch_processed_sync_committee(
        &self,
        state_id: &str,
    ) -> Result<SyncCommittee<SYNC_COMMITTEE_SIZE>, reqwest::Error> {
        // fetches sync committee from Node
        let node_sync_committee = self.fetch_sync_committee(state_id.clone()).await?;

        let mut validators: List<Validator, VALIDATOR_REGISTRY_LIMIT> = Default::default();
        for mut validator_index in node_sync_committee.validators.clone() {
            // fetches validator based on validator index
            let validator = self
                .fetch_validator(state_id.clone(), &validator_index)
                .await?;
            validators.push(validator);
        }

        let public_keys_vector = node_sync_committee
            .validators
            .into_iter()
            .map(|i| {
                let validator_index: ValidatorIndex = i.parse().unwrap();
                validators[validator_index].public_key.clone()
            })
            .collect::<Vector<_, SYNC_COMMITTEE_SIZE>>();

        let aggregate_public_key = eth_aggregate_public_keys(&public_keys_vector).unwrap();

        let sync_committee = SyncCommittee::<SYNC_COMMITTEE_SIZE> {
            public_keys: public_keys_vector,
            aggregate_public_key,
        };

        Ok(sync_committee)
    }

    async fn fetch_latest_finalized_block(
        &self,
    ) -> Result<(BeaconBlockHeader, BeaconBlockType), reqwest::Error> {
        let block_header = self.fetch_header("finalized").await?;
        let block = self.fetch_block("finalized").await?;

        Ok((block_header, block))
    }

    fn generate_route(&self, path: &str) -> String {
        format!("{}{}", self.node_url.clone(), path)
    }
}

fn get_attestation_slots_for_finalized_header(
    finalized_header: &BeaconBlockHeader,
    slots_per_epoch: u64,
) -> Slot {
    let finalized_header_slot = finalized_header.slot;

    // given that an epoch is 32 slots and blocks are finalized every 2 epochs
    // so the attested slot for a finalized block is 64 slots away
    let attested_slot = finalized_header_slot + (slots_per_epoch * 2);

    attested_slot
}

fn prove_beacon_state_values(
    data: BeaconStateType,
    indices: &[usize],
) -> anyhow::Result<Vec<Node>> {
    let proof = ssz_rs::generate_proof(data, indices)?;
    Ok(proof)
}

fn prove_beacon_block_values(
    data: BeaconBlockType,
    indices: &[usize],
) -> anyhow::Result<Vec<Node>> {
    let proof = ssz_rs::generate_proof(data, indices)?;
    Ok(proof)
}

fn prove_execution_payload(block: BeaconBlockType) -> anyhow::Result<ExecutionPayloadProof> {
    let indices = [
        EXECUTION_PAYLOAD_STATE_ROOT_INDEX as usize,
        EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX as usize,
    ];
    // generate multi proofs
    let multi_proof =
        ssz_rs::generate_proof(block.body.execution_payload.clone(), indices.as_slice())?;

    let execution_payload_index = [get_subtree_index(EXECUTION_PAYLOAD_INDEX) as usize];
    let execution_payload_branch =
        ssz_rs::generate_proof(block.body.clone(), execution_payload_index.as_slice())?;

    Ok(ExecutionPayloadProof {
        state_root: block.body.execution_payload.state_root,
        block_number: block.body.execution_payload.block_number,
        multi_proof: multi_proof
            .into_iter()
            .map(|node| Bytes32::try_from(node.as_bytes()).expect("Node is always 32 byte slice"))
            .collect(),
        execution_payload_branch: execution_payload_branch
            .into_iter()
            .map(|node| Bytes32::try_from(node.as_bytes()).expect("Node is always 32 byte slice"))
            .collect(),
    })
}

fn prove_sync_committee_update(state: BeaconStateType) -> anyhow::Result<Vec<Node>> {
    let indices = vec![get_subtree_index(NEXT_SYNC_COMMITTEE_INDEX) as usize];
    let proof = ssz_rs::generate_proof(state.clone(), indices.as_slice())?;

    Ok(proof)
}

fn prove_finalized_header(state: BeaconStateType) -> anyhow::Result<Vec<Node>> {
    let indices = vec![get_subtree_index(FINALIZED_ROOT_INDEX) as usize];
    let proof = ssz_rs::generate_proof(state.clone(), indices.as_slice())?;

    Ok(proof)
}

// function that generates block roots proof
// block number and convert to get_subtree_index
// beacon state has a block roots vec, pass the block root to generate proof
fn prove_block_roots_proof(
    state: BeaconStateType,
    block_number: u64,
) -> anyhow::Result<BlockRootsProof> {
    let indices = vec![get_subtree_index(block_number) as usize];
    let proof = ssz_rs::generate_proof(state.block_roots, indices.as_slice())?;

    Ok(BlockRootsProof {
        block_header_index: block_number,
        block_header_branch: proof
            .into_iter()
            .map(|node| Bytes32::try_from(node.as_bytes()).expect("Node is always 32 byte slice"))
            .collect(),
    })
}

//  prove the block roots vec inside the beacon state which will be the block roots branch

// when aggr sigs, create a new bit list
