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

//! BeaconKit consensus client implementation for ISMP.
//!
//! This module provides a consensus client for BeaconKit that verifies Tendermint light client
//! updates with BLS aggregated signatures, along with transaction proofs to verify the
//! embedded signed beacon block.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{
	boxed::Box,
	collections::BTreeMap,
	format,
	string::ToString,
	vec,
	vec::Vec,
};
use base64::Engine;
use base64::engine::general_purpose;
use codec::{Decode, Encode};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId, VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};

use evm_state_machine::EvmStateMachine;
use pallet_ismp_host_executive::Config as HostExecutiveConfig;
use primitive_types::H256;
use sha2::{Digest, Sha256};
use ssz_rs::prelude::*;
use sync_committee_primitives::consensus_types::{
	BeaconBlock, BeaconBlockBody, ExecutionPayload,
};
use sync_committee_primitives::constants::BlsSignature;
use tendermint_primitives::{CodecConsensusProof, CodecTrustedState, TrustedState};
use tendermint_verifier::verify_header_update;

pub const BEACON_KIT_CONSENSUS_CLIENT_ID: ConsensusClientId = *b"BKIT";
pub const BERACHAIN_MAINNET_CHAIN_ID: u32 = 80084;
pub const BERACHAIN_BEPOLIA_CHAIN_ID: u32 = 80069;
pub const MAX_PROPOSER_SLASHINGS: usize = 16;
pub const MAX_VALIDATORS_PER_COMMITTEE: usize = 2048;
pub const MAX_ATTESTER_SLASHINGS: usize = 2;
pub const MAX_ATTESTATIONS: usize = 128;
pub const MAX_DEPOSITS: usize = 16;
pub const MAX_VOLUNTARY_EXITS: usize = 16;
pub const SYNC_COMMITTEE_SIZE: usize = 512;
pub const BYTES_PER_LOGS_BLOOM: usize = 256;
pub const MAX_EXTRA_DATA_BYTES: usize = 32;
pub const MAX_BYTES_PER_TRANSACTION: usize = 1073741824;
pub const MAX_TRANSACTIONS_PER_PAYLOAD: usize = 1048576;
pub const MAX_WITHDRAWALS_PER_PAYLOAD: usize = 16;
pub const MAX_BLS_TO_EXECUTION_CHANGES: usize = 16;
pub const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize = 4096;
pub const MAX_COMMITTEES_PER_SLOT: usize = 64;
pub const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize = 8192;
pub const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize = 16;
pub const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize = 1;

pub type BeaconKitBlock = BeaconBlock<
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
	MAX_BLOB_COMMITMENTS_PER_BLOCK,
	MAX_COMMITTEES_PER_SLOT,
	MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
	MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
	MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
>;

pub type BeaconKitBlockBody = BeaconBlockBody<
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
	MAX_BLOB_COMMITMENTS_PER_BLOCK,
	MAX_COMMITTEES_PER_SLOT,
	MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
	MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
	MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
>;

pub type BeaconKitExecutionPayload = ExecutionPayload<
	BYTES_PER_LOGS_BLOOM,
	MAX_EXTRA_DATA_BYTES,
	MAX_BYTES_PER_TRANSACTION,
	MAX_TRANSACTIONS_PER_PAYLOAD,
	MAX_WITHDRAWALS_PER_PAYLOAD,
>;

/// Signed beacon block - wraps a BeaconBlock with its BLS signature
#[derive(Default, Debug, Clone, PartialEq, Eq, SimpleSerialize, Encode, Decode)]
pub struct SignedBeaconBlock {
	/// The beacon block message
	pub message: BeaconKitBlock,
	/// BLS signature over the block (96 bytes)
	pub signature: BlsSignature,
}

/// The consensus update/proof for BeaconKit
#[derive(Debug, Clone, Encode, Decode)]
pub struct BeaconKitUpdate {
	/// Tendermint consensus proof (signed header + validators) with BLS aggregation
	pub tendermint_update: CodecConsensusProof,
	/// SSZ-encoded SignedBeaconBlock (first transaction in CometBFT block)
	pub ssz_encoded_beacon_block: Vec<u8>,
	/// Merkle proof of the first transaction against DataHash
	/// Each H256 is a sibling hash in the merkle path
	pub tx_proof: Vec<H256>,
	/// Total number of transactions in the block
	pub tx_total: u64,
}

/// The trusted consensus state for BeaconKit
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ConsensusState {
	/// Tendermint trusted state
	pub tendermint_state: CodecTrustedState,
	/// Chain ID for the BeaconKit network (EVM chain ID)
	pub chain_id: u32,
}

/// Verify a transaction merkle proof against the data_hash.
///
/// CometBFT uses a simple merkle tree where:
/// - Leaf hash = sha256(0x00 || tx_data)
/// - Inner hash = sha256(0x01 || left || right)
/// - Empty hash = sha256("")
///
/// The proof consists of sibling hashes (aunts) from leaf to root.
pub fn verify_tx_proof(
	tx_data: &[u8],
	tx_total: u64,
	aunts: &[H256],
	data_hash: H256,
) -> Result<(), Error> {
	if tx_total == 0 {
		return Err(Error::Custom("No transactions in block".to_string()));
	}

	// Compute leaf hash: sha256(0x00 || tx_data)
	let mut hasher = Sha256::new();
	hasher.update([0x00]);
	hasher.update(tx_data);
	let leaf_hash: [u8; 32] = hasher.finalize().into();

	// transaction index is 0 for beacon block
	let tx_index = 0;

	// walk up the tree using aunts
	let computed_root = compute_root_from_aunts(tx_index, tx_total, leaf_hash, aunts)?;

	if H256::from(computed_root) != data_hash {
		return Err(Error::Custom(format!(
			"Transaction proof verification failed: computed {:?} != expected {:?}",
			H256::from(computed_root),
			data_hash
		)));
	}

	Ok(())
}

/// Compute the merkle root from a leaf hash and its aunts (sibling hashes).
fn compute_root_from_aunts(
	index: u64,
	total: u64,
	leaf_hash: [u8; 32],
	aunts: &[H256],
) -> Result<[u8; 32], Error> {
	if total == 0 {
		return Err(Error::Custom("Cannot compute root with total=0".to_string()));
	}

	if total == 1 {
		if !aunts.is_empty() {
			return Err(Error::Custom("Expected no aunts for single item".to_string()));
		}
		return Ok(leaf_hash);
	}

	if aunts.is_empty() {
		return Err(Error::Custom("Expected aunts for multiple items".to_string()));
	}

	let split = get_split_point(total);

	let (new_index, new_total, aunt_hash) = if index < split {
		// left side/branch
		(index, split, aunts.last().ok_or_else(|| Error::Custom("Missing aunt".to_string()))?)
	} else {
		// right side/branch
		(index - split, total - split, aunts.last().ok_or_else(|| Error::Custom("Missing aunt".to_string()))?)
	};

	let subtree_hash = compute_root_from_aunts(
		new_index,
		new_total,
		leaf_hash,
		&aunts[..aunts.len() - 1],
	)?;

	let parent_hash = if index < split {
		// subtree is on left, aunt is right sibling
		inner_hash(&subtree_hash, aunt_hash.as_bytes())
	} else {
		// aunt is left sibling, subtree is on right
		inner_hash(aunt_hash.as_bytes(), &subtree_hash)
	};

	Ok(parent_hash)
}

/// Compute leaf hash: sha256(0x00 || data)
pub fn leaf_hash(data: &[u8]) -> [u8; 32] {
	let mut hasher = Sha256::new();
	hasher.update([0x00]);
	hasher.update(data);
	hasher.finalize().into()
}

/// Compute inner node hash: sha256(0x01 || left || right)
pub fn inner_hash(left: &[u8], right: &[u8]) -> [u8; 32] {
	let mut hasher = Sha256::new();
	hasher.update([0x01]);
	hasher.update(left);
	hasher.update(right);
	hasher.finalize().into()
}

/// Get the split point for merkle tree construction.
/// Returns the largest power of 2 less than n.
pub fn get_split_point(n: u64) -> u64 {
	if n < 1 {
		panic!("n must be >= 1");
	}
	let bit_len = 64 - n.leading_zeros();
	1u64 << (bit_len - 1)
}

/// Compute the merkle root from a list of leaf hashes
pub fn compute_merkle_root(hashes: &[[u8; 32]]) -> [u8; 32] {
	match hashes.len() {
		0 => {
			// empty tree
			let hasher = Sha256::new();
			hasher.finalize().into()
		},
		1 => hashes[0],
		n => {
			let split = get_split_point(n as u64) as usize;
			let left = compute_merkle_root(&hashes[..split]);
			let right = compute_merkle_root(&hashes[split..]);
			inner_hash(&left, &right)
		},
	}
}

/// Compute the merkle proof (aunts) for a leaf at a given index
fn compute_proof(hashes: &[[u8; 32]], index: usize) -> Vec<[u8; 32]> {
	if hashes.len() <= 1 {
		return vec![];
	}

	let split = get_split_point(hashes.len() as u64) as usize;

	if index < split {
		// left side/branch
		let mut proof = compute_proof(&hashes[..split], index);
		// Add right subtree root as aunt/sibling
		let right_root = compute_merkle_root(&hashes[split..]);
		proof.push(right_root);
		proof
	} else {
		// right side/branch
		let mut proof = compute_proof(&hashes[split..], index - split);
		// Add left subtree root as aunt/sibling
		let left_root = compute_merkle_root(&hashes[..split]);
		proof.push(left_root);
		proof
	}
}

/// Generate a merkle proof for a transaction at a given index.
///
/// CometBFT uses a simple merkle tree where:
/// - Leaf hash = sha256(0x00 || tx_data)
/// - Inner hash = sha256(0x01 || left || right)
///
/// Returns the aunt hashes (sibling hashes) from leaf to root.
pub fn generate_tx_merkle_proof(
	txs: &[Vec<u8>],
	tx_index: usize,
) -> Result<Vec<H256>, Error> {
	if txs.is_empty() {
		return Err(Error::Custom("No transactions in block".to_string()));
	}

	if tx_index >= txs.len() {
		return Err(Error::Custom(format!(
			"Transaction index {} out of bounds (total: {})",
			tx_index,
			txs.len()
		)));
	}

	// Compute leaf hashes for all transactions
	let leaf_hashes: Vec<[u8; 32]> = txs.iter().map(|tx| leaf_hash(tx)).collect();

	// Generate the proof (aunts/siblings)
	let aunts = compute_proof(&leaf_hashes, tx_index);

	Ok(aunts.into_iter().map(H256::from).collect())
}

/// BeaconKit consensus client implementation
pub struct BeaconKitClient<H: IsmpHost, T: HostExecutiveConfig>(
	core::marker::PhantomData<(H, T)>,
);

impl<H: IsmpHost, T: HostExecutiveConfig> Default for BeaconKitClient<H, T> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<H: IsmpHost + Send + Sync + Default + 'static, T: HostExecutiveConfig> ConsensusClient
	for BeaconKitClient<H, T>
{
	fn verify_consensus(
		&self,
		host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		let beacon_kit_update: BeaconKitUpdate = Decode::decode(&mut &proof[..])
			.map_err(|e| Error::Custom(format!("Failed to decode BeaconKitUpdate: {}", e)))?;

		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|e| Error::Custom(format!("Failed to decode ConsensusState: {}", e)))?;

		let consensus_proof = beacon_kit_update
			.tendermint_update
			.to_consensus_proof()
			.map_err(|e| Error::Custom(format!("Failed to convert consensus proof: {}", e)))?;

		let trusted_state: TrustedState = consensus_state.tendermint_state.clone().into();
		let time = host.timestamp().as_secs();

		let updated_state = verify_header_update(trusted_state, consensus_proof.clone(), time)
			.map_err(|e| Error::Custom(format!("Tendermint header verification failed: {}", e)))?;

		let data_hash = consensus_proof
			.signed_header
			.header
			.data_hash
			.ok_or_else(|| Error::Custom("No data_hash in verified header".to_string()))?;

		let data_hash_h256 = H256::from_slice(data_hash.as_bytes());

		verify_tx_proof(
			&beacon_kit_update.ssz_encoded_beacon_block,
			beacon_kit_update.tx_total,
			&beacon_kit_update.tx_proof,
			data_hash_h256,
		)?;

		let beacon_block_bytes = general_purpose::STANDARD.decode(&beacon_kit_update.ssz_encoded_beacon_block).expect("Base64 Error");
		let ssz_beacon_block = &beacon_block_bytes[100..];

		let signed_beacon_block: BeaconBlock<MAX_PROPOSER_SLASHINGS, MAX_VALIDATORS_PER_COMMITTEE, MAX_ATTESTER_SLASHINGS, MAX_ATTESTATIONS, MAX_DEPOSITS, MAX_VOLUNTARY_EXITS, SYNC_COMMITTEE_SIZE, BYTES_PER_LOGS_BLOOM, MAX_EXTRA_DATA_BYTES, MAX_BYTES_PER_TRANSACTION, MAX_TRANSACTIONS_PER_PAYLOAD, MAX_WITHDRAWALS_PER_PAYLOAD, MAX_BLS_TO_EXECUTION_CHANGES, MAX_BLOB_COMMITMENTS_PER_BLOCK, MAX_COMMITTEES_PER_SLOT, MAX_DEPOSIT_REQUESTS_PER_PAYLOAD, MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD> =
			deserialize(&ssz_beacon_block)
				.map_err(|e| Error::Custom(format!("Failed to SSZ decode SignedBeaconBlock: {:?}", e)))?;

		let execution_payload = &signed_beacon_block.body.execution_payload;

		let state_root_bytes: [u8; 32] = execution_payload
			.state_root
			.as_ref()
			.try_into()
			.map_err(|_| Error::Custom("Invalid state root length".to_string()))?;

		let state_commitment = StateCommitmentHeight {
			commitment: StateCommitment {
				timestamp: execution_payload.timestamp,
				overlay_root: None,
				state_root: H256::from(state_root_bytes),
			},
			height: execution_payload.block_number,
		};

		let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
			BTreeMap::new();

		state_machine_map.insert(
			StateMachineId {
				state_id: StateMachine::Evm(consensus_state.chain_id),
				consensus_state_id,
			},
			vec![state_commitment],
		);

		let updated_consensus_state = ConsensusState {
			tendermint_state: CodecTrustedState::from(&updated_state.trusted_state),
			chain_id: consensus_state.chain_id,
		};

		Ok((updated_consensus_state.encode(), state_machine_map))
	}

	fn verify_fraud_proof(
		&self,
		host: &dyn IsmpHost,
		trusted_consensus_state: Vec<u8>,
		proof_1: Vec<u8>,
		proof_2: Vec<u8>,
	) -> Result<(), Error> {
		let update_1: BeaconKitUpdate =
			Decode::decode(&mut &proof_1[..]).map_err(|e| Error::Custom(e.to_string()))?;
		let update_2: BeaconKitUpdate =
			Decode::decode(&mut &proof_2[..]).map_err(|e| Error::Custom(e.to_string()))?;

		let consensus_state: ConsensusState = Decode::decode(&mut &trusted_consensus_state[..])
			.map_err(|e| Error::Custom(e.to_string()))?;


		let height_1 = update_1.tendermint_update.signed_header.header.height;
		let height_2 = update_2.tendermint_update.signed_header.header.height;
		if height_1 != height_2 {
			return Err(Error::Custom("Fraud proofs must be for the same block height".to_string()));
		}


		if proof_1 == proof_2 {
			return Err(Error::Custom("Fraud proofs are identical".to_string()));
		}

		let trusted_state: TrustedState = consensus_state.tendermint_state.into();
		let time = host.timestamp().as_secs();


		let consensus_proof_1 = update_1
			.tendermint_update
			.to_consensus_proof()
			.map_err(|e| Error::Custom(e.to_string()))?;

		let consensus_proof_2 = update_2
			.tendermint_update
			.to_consensus_proof()
			.map_err(|e| Error::Custom(e.to_string()))?;

		verify_header_update(trusted_state.clone(), consensus_proof_1, time)
			.map_err(|e| Error::Custom(e.to_string()))?;
		verify_header_update(trusted_state, consensus_proof_2, time)
			.map_err(|e| Error::Custom(e.to_string()))?;

		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		BEACON_KIT_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		match id {
			StateMachine::Evm(chain_id)
				if chain_id == BERACHAIN_MAINNET_CHAIN_ID
					|| chain_id == BERACHAIN_BEPOLIA_CHAIN_ID =>
			{
				Ok(Box::new(EvmStateMachine::<H, T>::default()))
			}
			_ => Err(Error::Custom("Unsupported state machine or chain ID".to_string())),
		}
	}
}
