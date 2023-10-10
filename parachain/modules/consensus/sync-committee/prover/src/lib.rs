#[warn(unused_imports)]
#[warn(unused_variables)]
mod responses;
mod routes;
#[cfg(test)]
mod test;

use anyhow::anyhow;
use bls_on_arkworks::{point_to_pubkey, types::G1ProjectivePoint};
use log::debug;
use reqwest::Client;
use sync_committee_primitives::{
    consensus_types::{
        BeaconBlock, BeaconBlockHeader, BeaconState, Checkpoint, SyncCommittee, Validator,
    },
    constants::EPOCHS_PER_SYNC_COMMITTEE_PERIOD,
    types::VerifierState,
};

use crate::{
    responses::{
        finality_checkpoint_response::FinalityCheckpoint,
        sync_committee_response::NodeSyncCommittee,
    },
    routes::*,
};
use primitive_types::H256;
use ssz_rs::{List, Merkleized, Node, Vector};
use sync_committee_primitives::{
    constants::{
        BlsPublicKey, Root, ValidatorIndex, BLOCK_ROOTS_INDEX, BYTES_PER_LOGS_BLOOM,
        EPOCHS_PER_HISTORICAL_VECTOR, EPOCHS_PER_SLASHINGS_VECTOR, ETH1_DATA_VOTES_BOUND,
        EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX, EXECUTION_PAYLOAD_INDEX,
        EXECUTION_PAYLOAD_STATE_ROOT_INDEX, EXECUTION_PAYLOAD_TIMESTAMP_INDEX,
        FINALIZED_ROOT_INDEX, HISTORICAL_ROOTS_LIMIT, MAX_ATTESTATIONS, MAX_ATTESTER_SLASHINGS,
        MAX_BLS_TO_EXECUTION_CHANGES, MAX_BYTES_PER_TRANSACTION, MAX_DEPOSITS,
        MAX_EXTRA_DATA_BYTES, MAX_PROPOSER_SLASHINGS, MAX_TRANSACTIONS_PER_PAYLOAD,
        MAX_VALIDATORS_PER_COMMITTEE, MAX_VOLUNTARY_EXITS, MAX_WITHDRAWALS_PER_PAYLOAD,
        NEXT_SYNC_COMMITTEE_INDEX, SLOTS_PER_EPOCH, SLOTS_PER_HISTORICAL_ROOT, SYNC_COMMITTEE_SIZE,
        VALIDATOR_REGISTRY_LIMIT,
    },
    types::{
        AncestryProof, BlockRootsProof, ExecutionPayloadProof, FinalityProof, SyncCommitteeUpdate,
        VerifierStateUpdate,
    },
    util::{
        compute_epoch_at_slot, compute_sync_committee_period_at_slot,
        should_have_sync_committee_update,
    },
};

use sync_committee_verifier::crypto::pubkey_to_projective;

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

#[derive(Clone)]
pub struct SyncCommitteeProver {
    pub node_url: String,
    pub client: Client,
}

impl SyncCommitteeProver {
    pub fn new(node_url: String) -> Self {
        let client = Client::new();

        SyncCommitteeProver { node_url, client }
    }

    pub async fn fetch_finalized_checkpoint(
        &self,
        state_id: Option<&str>,
    ) -> Result<FinalityCheckpoint, anyhow::Error> {
        let full_url = self.generate_route(&finality_checkpoints(state_id.unwrap_or("head")));
        let response = self.client.get(full_url).send().await?;

        let response_data =
            response.json::<responses::finality_checkpoint_response::Response>().await?;
        Ok(response_data.data)
    }

    pub async fn fetch_header(&self, block_id: &str) -> Result<BeaconBlockHeader, anyhow::Error> {
        let path = header_route(block_id);
        let full_url = self.generate_route(&path);
        let response = self.client.get(full_url).send().await?;

        let response_data =
            response.json::<responses::beacon_block_header_response::Response>().await?;

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
            MAX_WITHDRAWALS_PER_PAYLOAD,
            MAX_BLS_TO_EXECUTION_CHANGES,
        >,
        anyhow::Error,
    > {
        let path = block_route(block_id);
        let full_url = self.generate_route(&path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response.json::<responses::beacon_block_response::Response>().await?;

        let beacon_block = response_data.data.message;

        Ok(beacon_block)
    }

    pub async fn fetch_sync_committee(
        &self,
        state_id: &str,
    ) -> Result<NodeSyncCommittee, anyhow::Error> {
        let path = sync_committee_route(state_id);
        let full_url = self.generate_route(&path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response.json::<responses::sync_committee_response::Response>().await?;

        let sync_committee = response_data.data;

        Ok(sync_committee)
    }

    pub async fn fetch_validator(
        &self,
        state_id: &str,
        validator_index: &str,
    ) -> Result<Validator, anyhow::Error> {
        let path = validator_route(state_id, validator_index);
        let full_url = self.generate_route(&path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response.json::<responses::validator_response::Response>().await?;

        let validator = response_data.data.validator;

        Ok(validator)
    }

    pub async fn fetch_beacon_state(
        &self,
        state_id: &str,
    ) -> Result<BeaconStateType, anyhow::Error> {
        let path = beacon_state_route(state_id);
        let full_url = self.generate_route(&path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response.json::<responses::beacon_state_response::Response>().await?;

        let beacon_state = response_data.data;

        Ok(beacon_state)
    }

    pub async fn fetch_processed_sync_committee(
        &self,
        state_id: &str,
    ) -> Result<SyncCommittee<SYNC_COMMITTEE_SIZE>, anyhow::Error> {
        // fetches sync committee from Node
        let node_sync_committee = self.fetch_sync_committee(state_id).await?;

        let mut validators: List<Validator, VALIDATOR_REGISTRY_LIMIT> = Default::default();
        for validator_index in node_sync_committee.validators.iter() {
            // fetches validator based on validator index
            let validator = self.fetch_validator(state_id, validator_index).await?;
            validators.push(validator);
        }

        let public_keys_vector = node_sync_committee
            .validators
            .into_iter()
            .map(|i| {
                let validator_index: ValidatorIndex = i.parse()?;
                Ok(validators[validator_index as usize].public_key.clone())
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;

        let aggregate_public_key = eth_aggregate_public_keys(&public_keys_vector)?;

        let sync_committee = SyncCommittee::<SYNC_COMMITTEE_SIZE> {
            public_keys: Vector::<BlsPublicKey, SYNC_COMMITTEE_SIZE>::try_from(public_keys_vector)
                .map_err(|e| anyhow!("{:?}", e))?,
            aggregate_public_key,
        };

        Ok(sync_committee)
    }

    fn generate_route(&self, path: &str) -> String {
        format!("{}{}", self.node_url.clone(), path)
    }

    /// Fetches the latest finality update that can be verified by (state_period..=state_period+1)
    /// latest_block_id is an optional block id where to start the signature block search, if absent
    /// we use `head`
    pub async fn fetch_light_client_update(
        &self,
        client_state: VerifierState,
        finality_checkpoint: Checkpoint,
        latest_block_id: Option<&str>,
        debug_target: &str,
    ) -> Result<Option<VerifierStateUpdate>, anyhow::Error> {
        if finality_checkpoint.root == Node::default() ||
            client_state.latest_finalized_epoch >= finality_checkpoint.epoch
        {
            return Ok(None)
        }

        debug!(target: debug_target, "A new epoch has been finalized {}", finality_checkpoint.epoch);
        // Find the highest block with the a threshhold number of sync committee signatures
        let latest_header = self.fetch_header(latest_block_id.unwrap_or("head")).await?;
        let latest_root = latest_header.clone().hash_tree_root()?;
        let get_block_id = |root: Root| {
            let mut block_id = hex::encode(root.0.to_vec());
            block_id.insert_str(0, "0x");
            block_id
        };
        let mut block = self.fetch_block(&get_block_id(latest_root)).await?;
        let min_signatures = (2 * SYNC_COMMITTEE_SIZE) / 3;
        let state_period =
            compute_sync_committee_period_at_slot(client_state.finalized_header.slot);
        loop {
            // Some checks on the epoch finalized by the signature block
            let parent_root = block.parent_root;
            let parent_block_id = get_block_id(parent_root);
            let parent_block = self.fetch_block(&parent_block_id).await?;
            let parent_state_id = get_block_id(parent_block.state_root);
            let parent_block_finality_checkpoint =
                self.fetch_finalized_checkpoint(Some(&parent_state_id)).await?.finalized;
            if parent_block_finality_checkpoint.epoch <= client_state.latest_finalized_epoch {
                debug!(target: "prover", "Signature block search has reached an invalid epoch {} latest finalized_block_epoch {}", parent_block_finality_checkpoint.epoch, client_state.latest_finalized_epoch);
                return Ok(None)
            }

            let num_signatures = block.body.sync_aggregate.sync_committee_bits.count_ones();

            let signature_period = compute_sync_committee_period_at_slot(block.slot);

            if num_signatures >= min_signatures &&
                (state_period..=state_period + 1).contains(&signature_period) &&
                parent_block_finality_checkpoint.epoch > client_state.latest_finalized_epoch
            {
                break
            }
            block = parent_block;
        }

        let attested_block_id = get_block_id(block.parent_root);
        let attested_header = self.fetch_header(&attested_block_id).await?;
        let mut attested_state =
            self.fetch_beacon_state(&get_block_id(attested_header.state_root)).await?;
        if attested_state.finalized_checkpoint.root == Node::default() {
            return Ok(None)
        }
        let finalized_block_id = get_block_id(attested_state.finalized_checkpoint.root);
        let finalized_header = self.fetch_header(&finalized_block_id).await?;
        let mut finalized_state =
            self.fetch_beacon_state(&get_block_id(finalized_header.state_root)).await?;
        let finality_proof = FinalityProof {
            epoch: attested_state.finalized_checkpoint.epoch,
            finality_branch: prove_finalized_header(&mut attested_state)?,
        };

        let execution_payload_proof = prove_execution_payload(&mut finalized_state)?;

        let signature_period = compute_sync_committee_period_at_slot(block.slot);
        let sync_committee_update =
            if should_have_sync_committee_update(state_period, signature_period) {
                let sync_committee_proof = prove_sync_committee_update(&mut attested_state)?;
                Some(SyncCommitteeUpdate {
                    next_sync_committee: attested_state.next_sync_committee,
                    next_sync_committee_branch: sync_committee_proof,
                })
            } else {
                None
            };

        // construct light client
        let light_client_update = VerifierStateUpdate {
            attested_header,
            sync_committee_update,
            finalized_header,
            execution_payload: execution_payload_proof,
            finality_proof,
            sync_aggregate: block.body.sync_aggregate,
            signature_slot: block.slot,
        };

        Ok(Some(light_client_update))
    }

    pub async fn latest_update_for_period(
        &self,
        period: u64,
    ) -> Result<VerifierStateUpdate, anyhow::Error> {
        let mut higest_slot_in_epoch = ((period * EPOCHS_PER_SYNC_COMMITTEE_PERIOD) *
            SLOTS_PER_EPOCH) +
            (EPOCHS_PER_SYNC_COMMITTEE_PERIOD * SLOTS_PER_EPOCH) -
            1;
        let mut count = 0;
        // Some slots are empty so we'll use a loop to fetch the highest available slot in an epoch
        let mut block = loop {
            // Prevent an infinite loop
            if count == 100 {
                return Err(anyhow!("Error fetching blocks from selected epoch"))
            }

            if let Ok(block) = self.fetch_block(&higest_slot_in_epoch.to_string()).await {
                break block
            } else {
                higest_slot_in_epoch -= 1;
                count += 1;
            }
        };
        let min_signatures = (2 * SYNC_COMMITTEE_SIZE) / 3;
        let get_block_id = |root: Root| {
            let mut block_id = hex::encode(root.0.to_vec());
            block_id.insert_str(0, "0x");
            block_id
        };
        loop {
            let num_signatures = block.body.sync_aggregate.sync_committee_bits.count_ones();
            if num_signatures >= min_signatures {
                break
            }

            let parent_root = block.parent_root;
            let parent_block_id = get_block_id(parent_root);
            let parent_block = self.fetch_block(&parent_block_id).await?;

            block = parent_block;
        }

        let attested_block_id = get_block_id(block.parent_root);

        let attested_header = self.fetch_header(&attested_block_id).await?;
        let mut attested_state =
            self.fetch_beacon_state(&get_block_id(attested_header.state_root)).await?;
        let finalized_block_id = get_block_id(attested_state.finalized_checkpoint.root);
        let finalized_header = self.fetch_header(&finalized_block_id).await?;
        let mut finalized_state =
            self.fetch_beacon_state(&get_block_id(finalized_header.state_root)).await?;
        let finality_proof = FinalityProof {
            epoch: attested_state.finalized_checkpoint.epoch,
            finality_branch: prove_finalized_header(&mut attested_state)?,
        };

        let execution_payload_proof = prove_execution_payload(&mut finalized_state)?;

        let sync_committee_update = {
            let sync_committee_proof = prove_sync_committee_update(&mut attested_state)?;
            Some(SyncCommitteeUpdate {
                next_sync_committee: attested_state.next_sync_committee,
                next_sync_committee_branch: sync_committee_proof,
            })
        };

        // construct light client
        let light_client_update = VerifierStateUpdate {
            attested_header,
            sync_committee_update,
            finalized_header,
            execution_payload: execution_payload_proof,
            finality_proof,
            sync_aggregate: block.body.sync_aggregate,
            signature_slot: block.slot,
        };

        Ok(light_client_update)
    }
}

pub fn prove_execution_payload(
    beacon_state: &mut BeaconStateType,
) -> anyhow::Result<ExecutionPayloadProof> {
    let indices = [
        EXECUTION_PAYLOAD_STATE_ROOT_INDEX as usize,
        EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX as usize,
        EXECUTION_PAYLOAD_TIMESTAMP_INDEX as usize,
    ];
    // generate multi proofs
    let multi_proof = ssz_rs::generate_proof(
        &mut beacon_state.latest_execution_payload_header,
        indices.as_slice(),
    )?;

    Ok(ExecutionPayloadProof {
        state_root: H256::from_slice(
            beacon_state.latest_execution_payload_header.state_root.as_slice(),
        ),
        block_number: beacon_state.latest_execution_payload_header.block_number,
        timestamp: beacon_state.latest_execution_payload_header.timestamp,
        multi_proof,
        execution_payload_branch: ssz_rs::generate_proof(
            beacon_state,
            &[EXECUTION_PAYLOAD_INDEX as usize],
        )?,
    })
}

pub fn prove_sync_committee_update(state: &mut BeaconStateType) -> anyhow::Result<Vec<Node>> {
    let proof = ssz_rs::generate_proof(state, &[NEXT_SYNC_COMMITTEE_INDEX as usize])?;
    Ok(proof)
}

pub fn prove_finalized_header(state: &mut BeaconStateType) -> anyhow::Result<Vec<Node>> {
    let indices = [FINALIZED_ROOT_INDEX as usize];
    let proof = ssz_rs::generate_proof(state, indices.as_slice())?;

    Ok(proof)
}

pub fn prove_block_roots_proof(
    state: &mut BeaconStateType,
    mut header: BeaconBlockHeader,
) -> anyhow::Result<AncestryProof> {
    // Check if block root should still be part of the block roots vector on the beacon state
    let epoch_for_header = compute_epoch_at_slot(header.slot) as usize;
    let epoch_for_state = compute_epoch_at_slot(state.slot) as usize;

    if epoch_for_state.saturating_sub(epoch_for_header) >=
        SLOTS_PER_HISTORICAL_ROOT / SLOTS_PER_EPOCH as usize
    {
        // todo:  Historical root proofs
        unimplemented!()
    } else {
        // Get index of block root in the block roots
        let block_root = header.hash_tree_root().expect("hash tree root should be valid");
        let block_index = state
            .block_roots
            .as_ref()
            .into_iter()
            .position(|root| root == &block_root)
            .expect("Block root should exist in block_roots");

        let proof = ssz_rs::generate_proof(&mut state.block_roots, &[block_index])?;

        let block_roots_proof =
            BlockRootsProof { block_header_index: block_index as u64, block_header_branch: proof };

        let block_roots_branch = ssz_rs::generate_proof(state, &[BLOCK_ROOTS_INDEX as usize])?;
        Ok(AncestryProof::BlockRoots { block_roots_proof, block_roots_branch })
    }
}

pub fn eth_aggregate_public_keys(points: &[BlsPublicKey]) -> anyhow::Result<BlsPublicKey> {
    let points = points
        .iter()
        .map(|point| pubkey_to_projective(point))
        .collect::<Result<Vec<_>, _>>()?;
    let aggregate = points
        .into_iter()
        .fold(G1ProjectivePoint::default(), |acc, g1_point| acc + g1_point);
    let public_key = point_to_pubkey(aggregate.into());

    let bls_public_key =
        BlsPublicKey::try_from(public_key.as_slice()).map_err(|e| anyhow!("{:?}", e))?;

    Ok(bls_public_key)
}
