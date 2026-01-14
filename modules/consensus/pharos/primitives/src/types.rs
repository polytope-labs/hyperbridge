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
use alloc::vec::Vec;
use codec::{Decode, Encode};
use geth_primitives::CodecHeader;
use primitive_types::{H160, H256, U256};

/// Unique identifier for a validator pool in the staking contract
pub type PoolId = H256;

/// Information about a single validator.
///
/// Each validator has a BLS public key for signing blocks and a stake amount
/// that determines their voting power in consensus.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidatorInfo {
	/// The validator's Ethereum address
	pub address: H160,
	/// The validator's BLS public key (48 bytes compressed)
	pub bls_public_key: BlsPublicKey,
	/// The validator's pool ID in the staking contract
	pub pool_id: PoolId,
	/// The stake amount
	pub stake: U256,
}

/// The complete validator set for a given epoch.
///
/// This represents the set of validators that are eligible to sign blocks
/// during a specific epoch. The validator set is updated at epoch boundaries(last block of an
/// epoch).
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidatorSet {
	/// List of all validators in the set
	pub validators: Vec<ValidatorInfo>,
	/// Total stake across all validators
	pub total_stake: U256,
	/// The epoch this validator set is valid for
	pub epoch: u64,
}

impl ValidatorSet {
	/// Create a new empty validator set
	pub fn new(epoch: u64) -> Self {
		Self { validators: Vec::new(), total_stake: U256::zero(), epoch }
	}

	/// Add a validator to the set
	pub fn add_validator(&mut self, validator: ValidatorInfo) {
		self.total_stake = self.total_stake.saturating_add(validator.stake);
		self.validators.push(validator);
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
}

/// Block proof as returned by `debug_getBlockProof` RPC.
///
/// This contains the aggregated BLS signature for a block and the list
/// of participating validators who signed it.
///
/// Based on Pharos block verification format:
/// ```go
/// type BlockProof struct {
///     BlockNumber            string   `json:"blockNumber"`
///     BlockProofHash         string   `json:"blockProofHash"`
///     BlsAggregatedSignature string   `json:"blsAggregatedSignature"`
///     SignedBlsKeys          []string `json:"signedBlsKeys"`
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockProof {
	/// The block number this proof is for (from JSON "blockNumber")
	pub block_number: u64,
	/// The block hash (derived from header)
	pub block_hash: H256,
	/// The block proof hash - this is the message that validators sign
	/// (from JSON "blockProofHash")
	///
	/// This is distinct from the block hash and is the actual message
	/// that the BLS signature is computed over.
	pub block_proof_hash: H256,
	/// The aggregated BLS signature from participating validators (96 bytes)
	/// (from JSON "blsAggregatedSignature")
	pub aggregate_signature: Vec<u8>,
	/// List of BLS public keys of validators who participated in signing
	/// (from JSON "signedBlsKeys")
	pub participant_keys: Vec<BlsPublicKey>,
}

impl BlockProof {
	/// Get the number of participants who signed this block
	pub fn participant_count(&self) -> usize {
		self.participant_keys.len()
	}
}

/// State proof for validator set stored in the staking contract.
///
/// This proof is required when the validator set changes at epoch boundaries.
/// It proves the new validator set against the state root of the epoch boundary block.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidatorSetProof {
	/// The new validator set
	pub validator_set: ValidatorSet,
	/// Merkle-Patricia trie proof nodes from the staking contract storage
	pub storage_proof: Vec<Vec<u8>>,
	/// The account proof for the staking contract
	pub account_proof: Vec<Vec<u8>>,
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
