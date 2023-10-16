// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
    prelude::*,
    presets::L2_OUTPUTS_SLOT,
    utils::{derive_array_item_key, get_contract_storage_root, get_value_from_proof, to_bytes_32},
};
use alloc::{format, string::ToString};
use alloy_rlp::Decodable;
use ethabi::ethereum_types::{H160, H256, U128, U256};
use ismp::{
    consensus::{
        ConsensusStateId, IntermediateState, StateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error,
    host::{Ethereum, IsmpHost, StateMachine},
};

#[derive(codec::Encode, codec::Decode, Debug)]
pub struct OptimismPayloadProof {
    /// Actual state root of the optimism execution layer
    pub state_root: H256,
    /// Storage root hash of the optimism withdrawal contracts
    pub withdrawal_storage_root: H256,
    /// Optimism Block hash at which the values aboved were fetched
    pub l2_block_hash: H256,
    /// L2Oracle contract version
    pub version: H256,
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

pub fn verify_optimism_payload<H: IsmpHost + Send + Sync>(
    payload: OptimismPayloadProof,
    root: &[u8],
    l2_oracle_address: H160,
    consensus_state_id: ConsensusStateId,
) -> Result<IntermediateState, Error> {
    let root = to_bytes_32(root)?;
    let root = H256::from_slice(&root[..]);
    let storage_root =
        get_contract_storage_root::<H>(payload.l2_oracle_proof, l2_oracle_address, root)?;

    let mut buf = Vec::with_capacity(128);
    buf.extend_from_slice(&payload.version[..]);
    buf.extend_from_slice(&payload.state_root[..]);
    buf.extend_from_slice(&payload.withdrawal_storage_root[..]);
    buf.extend_from_slice(&payload.l2_block_hash[..]);

    let output_root = H::keccak256(&buf);

    let output_root_key = derive_array_item_key::<H>(L2_OUTPUTS_SLOT, payload.output_root_index, 0);

    let proof_value = match get_value_from_proof::<H>(
        output_root_key,
        storage_root,
        payload.output_root_proof,
    )? {
        Some(value) => value.clone(),
        _ => Err(Error::MembershipProofVerificationFailed("Value not found in proof".to_string()))?,
    };

    let proof_value = <alloy_primitives::U256 as Decodable>::decode(&mut &*proof_value)
        .map_err(|_| {
            Error::ImplementationSpecific(format!(
                "Error decoding output root from {:?}",
                &proof_value
            ))
        })?
        .to_be_bytes::<32>();

    if proof_value != output_root.0 {
        return Err(Error::MembershipProofVerificationFailed(
            "Invalid optimism output root proof".to_string(),
        ))
    }

    // verify timestamp and block number
    let timestamp_block_number_key =
        derive_array_item_key::<H>(L2_OUTPUTS_SLOT, payload.output_root_index, 1);
    let block_and_timestamp = match get_value_from_proof::<H>(
        timestamp_block_number_key,
        storage_root,
        payload.multi_proof,
    )? {
        Some(value) => value.clone(),
        _ => Err(Error::MembershipProofVerificationFailed("Value not found in proof".to_string()))?,
    };

    let block_and_timestamp =
        <alloy_primitives::U256 as Decodable>::decode(&mut &*block_and_timestamp)
            .map_err(|_| {
                Error::ImplementationSpecific(format!(
                    "Error decoding block and timestamp from{:?}",
                    &block_and_timestamp
                ))
            })?
            .to_be_bytes::<32>();

    let block_and_timestamp = U256::from_big_endian(&block_and_timestamp);
    // Timestamp is contained in the first two u64 values
    let timestamp = block_and_timestamp.low_u128() as u64;

    // Block number occupies the last two u64 values
    let mut block_number = [0u64; 2];
    block_number.copy_from_slice(&block_and_timestamp.0[2..]);
    let block_number = U128(block_number).as_u128() as u64;

    if payload.timestamp != timestamp && payload.block_number != block_number {
        return Err(Error::MembershipProofVerificationFailed(
            "Invalid optimism block and timestamp proof".to_string(),
        ))
    }

    Ok(IntermediateState {
        height: StateMachineHeight {
            id: StateMachineId {
                state_id: StateMachine::Ethereum(Ethereum::Optimism),
                consensus_state_id,
            },
            height: payload.block_number,
        },
        commitment: StateCommitment {
            timestamp: payload.timestamp,
            overlay_root: None,
            state_root: payload.state_root,
        },
    })
}
