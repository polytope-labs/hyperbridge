use crate::consensus_clients::{
    beacon_consensus_client::{
        presets::L2_ORACLE_ADDRESS,
        state_machine_ids::OPTIMISM_ID,
        utils::{
            derive_array_item_key, get_contract_storage_root, get_value_from_proof, to_bytes_32,
        },
    },
    consensus_client_ids::ETHEREUM_CONSENSUS_CLIENT_ID,
};
use alloc::string::ToString;
use ethabi::ethereum_types::{H256, U128};
use ismp_rs::{
    consensus_client::{IntermediateState, StateCommitment, StateMachineHeight, StateMachineId},
    error::Error,
};
use sp_std::prelude::*;

#[derive(codec::Encode, codec::Decode)]
pub struct OptimismPayloadProof {
    /// Actual state root of the optimism execution layer
    pub state_root: [u8; 32],
    /// Storage root hash of the optimism withdrawal contracts
    pub withdrawal_storage_root: [u8; 32],
    /// Optimism Block hash at which the values aboved were fetched
    pub l2_block_hash: [u8; 32],
    /// L2Oracle contract version
    pub version: [u8; 32],
    /// Membership Proof for the L2Oracle contract account in the ethereum world trie
    pub l2_oracle_proof: Vec<Vec<u8>>,
    /// Membership proof for output root in l2Outputs array
    pub output_root_proof: Vec<Vec<u8>>,
    /// Membership proof Timestamp and block number in the l2Outputs array
    pub multi_proof: Vec<Vec<u8>>,
    /// Index of the output root that needs to be proved in the l2Outputs array
    pub output_root_index: u64,
    /// Block number
    pub block_number: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// Slot for the l2Outputs array in the L2Oracle contract
pub(super) const L2_OUTPUTS_SLOT: u8 = 3;

pub(super) fn verify_optimism_payload(
    payload: OptimismPayloadProof,
    root: &[u8],
) -> Result<IntermediateState, Error> {
    let root = to_bytes_32(root)?;
    let root = H256::from_slice(&root[..]);
    let storage_root =
        get_contract_storage_root(payload.l2_oracle_proof, &L2_ORACLE_ADDRESS, root)?;

    let mut buf = Vec::with_capacity(128);
    buf.extend_from_slice(&payload.version[..]);
    buf.extend_from_slice(&payload.state_root[..]);
    buf.extend_from_slice(&payload.withdrawal_storage_root[..]);
    buf.extend_from_slice(&payload.l2_block_hash[..]);

    let output_root = sp_io::hashing::keccak_256(&buf);

    let output_root_key = derive_array_item_key(L2_OUTPUTS_SLOT, payload.output_root_index);

    let proof_value =
        get_value_from_proof(output_root_key, storage_root, payload.output_root_proof)?
            .ok_or_else(|| {
                Error::MembershipProofVerificationFailed("Value not found in proof".to_string())
            })?;

    if &proof_value != &output_root[..] {
        return Err(Error::MembershipProofVerificationFailed(
            "Invalid optimism output root proof".to_string(),
        ))
    }

    // verify timestamp and block number
    let timestamp_block_number_key =
        derive_array_item_key(L2_OUTPUTS_SLOT, payload.output_root_index + 1);
    let block_and_timestamp =
        get_value_from_proof(timestamp_block_number_key, storage_root, payload.multi_proof)?
            .ok_or_else(|| {
                Error::MembershipProofVerificationFailed("Value not found in proof".to_string())
            })?;

    let mut timestamp = Vec::with_capacity(16);
    U128::from(payload.timestamp).to_big_endian(&mut timestamp);

    let mut block_number = Vec::with_capacity(16);
    U128::from(payload.block_number).to_big_endian(&mut block_number);

    let mut concat = Vec::with_capacity(32);
    concat.extend_from_slice(&timestamp);
    concat.extend_from_slice(&block_number);

    if block_and_timestamp != concat {
        return Err(Error::MembershipProofVerificationFailed(
            "Invalid optimism block and timestamp proof".to_string(),
        ))
    }

    Ok(IntermediateState {
        height: StateMachineHeight {
            id: StateMachineId {
                state_id: OPTIMISM_ID,
                consensus_client: ETHEREUM_CONSENSUS_CLIENT_ID,
            },
            height: payload.block_number,
        },
        commitment: StateCommitment {
            timestamp: payload.timestamp,
            ismp_root: [0u8; 32],
            state_root: payload.state_root,
        },
    })
}
