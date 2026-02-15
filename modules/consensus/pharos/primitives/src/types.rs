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

//! Type definitions for Pharos consensus.

use crate::constants::BlsPublicKey;
use alloc::{collections::BTreeSet, vec::Vec};
use codec::{Decode, Encode};
use core::cmp::Ordering;
use geth_primitives::CodecHeader;
use primitive_types::{H256, U256};

/// Unique identifier for a validator pool in the staking contract
pub type PoolId = H256;

/// Information about a single validator.
///
/// Each validator has a BLS public key for signing blocks and a stake amount
/// that determines their voting power in consensus.
///
/// Validators are ordered by their BLS public key to enable use in BTreeSet,
/// which automatically prevents duplicates.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidatorInfo {
	/// The validator's BLS public key (48 bytes compressed)
	pub bls_public_key: BlsPublicKey,
	/// The validator's pool ID in the staking contract
	pub pool_id: PoolId,
	/// The stake amount
	pub stake: U256,
}

impl PartialOrd for ValidatorInfo {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for ValidatorInfo {
	fn cmp(&self, other: &Self) -> Ordering {
		self.bls_public_key.cmp(&other.bls_public_key)
	}
}

/// The complete validator set for a given epoch.
///
/// This represents the set of validators that are eligible to sign blocks
/// during a specific epoch. The validator set is updated at epoch boundaries(last block of an
/// epoch).
///
/// Uses `BTreeSet` to automatically prevent duplicate validators (by BLS public key).
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidatorSet {
	/// Set of all validators
	pub validators: BTreeSet<ValidatorInfo>,
	/// Total stake across all validators
	pub total_stake: U256,
	/// The epoch this validator set is valid for
	pub epoch: u64,
}

impl ValidatorSet {
	/// Create a new empty validator set
	pub fn new(epoch: u64) -> Self {
		Self { validators: BTreeSet::new(), total_stake: U256::zero(), epoch }
	}

	/// Add a validator to the set.
	/// Returns true if the validator was added, false if it was a duplicate.
	pub fn add_validator(&mut self, validator: ValidatorInfo) -> bool {
		let stake = validator.stake;
		if self.validators.insert(validator) {
			self.total_stake = self.total_stake.saturating_add(stake);
			true
		} else {
			false
		}
	}

	/// Check if a validator is in the set by their BLS public key
	pub fn contains(&self, bls_key: &BlsPublicKey) -> bool {
		self.validators.iter().any(|v| &v.bls_public_key == bls_key)
	}

	/// Get a validator by their BLS public key
	pub fn get_validator(&self, bls_key: &BlsPublicKey) -> Option<&ValidatorInfo> {
		self.validators.iter().find(|v| &v.bls_public_key == bls_key)
	}

	/// Calculate the stake of participating validators
	pub fn participating_stake(&self, participants: &[BlsPublicKey]) -> U256 {
		participants
			.iter()
			.filter_map(|key| self.get_validator(key))
			.fold(U256::zero(), |acc, v| acc.saturating_add(v.stake))
	}

	/// Check if participating stake meets the 2/3 + 1 threshold
	pub fn has_supermajority(&self, participants: &[BlsPublicKey]) -> bool {
		let participating = self.participating_stake(participants);
		let required = (self.total_stake * 2 / 3) + 1;
		participating >= required
	}

	/// Get the number of validators in the set
	pub fn len(&self) -> usize {
		self.validators.len()
	}

	/// Check if the validator set is empty
	pub fn is_empty(&self) -> bool {
		self.validators.is_empty()
	}
}

/// Block proof containing the BLS signature data.
///
/// This contains the aggregated BLS signature for a block and the list
/// of participating validators who signed it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockProof {
	/// The aggregated BLS signature from participating validators (96 bytes)
	pub aggregate_signature: Vec<u8>,
	/// List of BLS public keys of validators who participated in signing
	pub participant_keys: Vec<BlsPublicKey>,
}

impl BlockProof {
	/// Get the number of participants who signed this block
	pub fn participant_count(&self) -> usize {
		self.participant_keys.len()
	}
}

/// Single node in a Pharos hexary hash tree proof path.
///
/// Each proof node contains the raw node bytes and offsets indicating where
/// the child hash appears within this node (used for bottom-up verification).
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PharosProofNode {
	/// Raw bytes of this proof node
	pub proof_node: Vec<u8>,
	/// Start offset within this node where the next (child) hash begins
	pub next_begin_offset: u32,
	/// End offset within this node where the next (child) hash ends
	pub next_end_offset: u32,
}

/// State proof for validator set stored in the staking contract.
///
/// This proof is required when the validator set changes at epoch boundaries.
/// The validator set is decoded directly from the proof.
///
/// Uses Pharos hexary hash tree proofs (SHA-256) instead of Ethereum MPT (Keccak-256).
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidatorSetProof {
	/// Account proof nodes (verified against state_root from header)
	pub account_proof: Vec<PharosProofNode>,
	/// Storage proof nodes (verified against storage_hash)
	pub storage_proof: Vec<PharosProofNode>,
	/// Storage trie root (from eth_getProof response)
	pub storage_hash: H256,
	/// RLP-encoded account value (rawValue from eth_getProof response)
	/// Format: RLP([nonce, balance, "", code_hash])
	pub raw_account_value: Vec<u8>,
	/// Raw storage values in order: [totalStake, activePoolIds length,
	/// pool_id_0..pool_id_n, validator_0_bls_header, validator_0_bls_data_0..2,
	/// validator_0_stake, ...]
	pub storage_values: Vec<Vec<u8>>,
}

/// The trusted state maintained by the Pharos consensus client.
///
/// This state is updated as new blocks are verified and represents
/// the current view of the chain from the light client's perspective.
#[derive(Debug, Clone, Encode, Decode)]
pub struct VerifierState {
	/// The current (active) validator set
	pub current_validator_set: ValidatorSet,
	/// The latest finalized block number
	pub finalized_block_number: u64,
	/// The hash of the finalized header
	pub finalized_hash: H256,
	/// The current epoch number
	pub current_epoch: u64,
}

impl VerifierState {
	/// Create a new verifier state with initial trusted state
	pub fn new(
		initial_validator_set: ValidatorSet,
		initial_block_number: u64,
		initial_hash: H256,
	) -> Self {
		let epoch = initial_validator_set.epoch;
		Self {
			current_validator_set: initial_validator_set,
			finalized_block_number: initial_block_number,
			finalized_hash: initial_hash,
			current_epoch: epoch,
		}
	}
}

/// Data required to update the verifier state.
///
/// This is what the prover submits to advance the light client's state.
#[derive(Debug, Clone, Encode, Decode)]
pub struct VerifierStateUpdate {
	/// The header being attested to
	pub header: CodecHeader,
	/// Block proof from debug_getBlockProof containing the BLS signature
	pub block_proof: BlockProof,
	/// Optional validator set update proof (required at epoch boundaries)
	pub validator_set_proof: Option<ValidatorSetProof>,
}

impl VerifierStateUpdate {
	/// Get the block number from the header
	pub fn block_number(&self) -> u64 {
		self.header.number.low_u64()
	}

	/// Check if this update includes a validator set rotation
	pub fn has_validator_set_update(&self) -> bool {
		self.validator_set_proof.is_some()
	}
}

/// Result of successful verification
#[derive(Debug, Clone, Encode, Decode)]
pub struct VerificationResult {
	/// The verified block hash
	pub block_hash: H256,
	/// The verified header
	pub header: CodecHeader,
	/// The new validator set if this was an epoch boundary block
	pub new_validator_set: Option<ValidatorSet>,
}
