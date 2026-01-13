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

//! Error types for Pharos verifier.

use pharos_primitives::BlsPublicKey;
use primitive_types::U256;
use thiserror::Error;

/// Errors that can occur during Pharos block verification.
#[derive(Debug, Error)]
pub enum Error {
	/// The update is for a block that has already been finalized
	#[error("Stale update: current finalized block {current} >= update block {update}")]
	StaleUpdate {
		/// Current finalized block number
		current: u64,
		/// Update block number
		update: u64,
	},

	/// The update block is not in the expected epoch
	#[error("Epoch mismatch: update block is in epoch {update_epoch}, expected {expected_epoch}")]
	EpochMismatch {
		/// The epoch of the update block
		update_epoch: u64,
		/// The expected epoch (current verifier state epoch)
		expected_epoch: u64,
	},

	/// A participating validator is not in the trusted validator set
	#[error("Unknown validator with BLS key: {}", hex::encode(&key.as_ref()[..8]))]
	UnknownValidator {
		/// The unknown validator's BLS public key
		key: BlsPublicKey,
	},

	/// Not enough stake participated in signing the block
	#[error(
		"Insufficient stake: {participating} participated, {required} required (total: {total})"
	)]
	InsufficientStake {
		/// Stake that participated
		participating: U256,
		/// Required stake (2/3 + 1 of total)
		required: U256,
		/// Total network stake
		total: U256,
	},

	/// No validators participated in signing
	#[error("No validators participated in signing")]
	NoParticipants,

	/// BLS signature verification failed
	#[error("BLS signature verification failed")]
	InvalidSignature,

	/// BLS cryptography error
	#[error("BLS error: {0:?}")]
	BlsError(#[from] bls::errors::BLSError),

	/// Missing validator set proof for epoch boundary block
	#[error("Missing validator set proof for epoch boundary block {block_number}")]
	MissingValidatorSetProof {
		/// Block number that requires a validator set proof
		block_number: u64,
	},

	/// Unexpected validator set proof for non-epoch-boundary block
	#[error("Unexpected validator set proof for non-epoch-boundary block {block_number}")]
	UnexpectedValidatorSetProof {
		/// Block number that should not have a validator set proof
		block_number: u64,
	},

	/// Header hash mismatch
	#[error("Header hash mismatch: expected {expected:?}, got {actual:?}")]
	HeaderHashMismatch {
		/// Expected hash
		expected: primitive_types::H256,
		/// Actual hash
		actual: primitive_types::H256,
	},

	/// Trie lookup failed during account proof verification
	#[error("Account proof trie lookup failed")]
	AccountTrieLookupFailed,

	/// Staking contract account not found in the state trie
	#[error("Staking contract account not found in proof")]
	StakingContractNotFound,

	/// Failed to decode RLP-encoded account data
	#[error("Failed to decode account RLP data")]
	AccountRlpDecodeFailed,

	/// Storage proof lookup failed
	#[error("Storage proof lookup failed")]
	StorageProofLookupFailed,

	/// Storage value exceeds maximum size for U256
	#[error("Storage value too large for U256")]
	StorageValueTooLarge,

	/// Mismatch between number of storage slots and values
	#[error("Slots and values length mismatch: {slots} slots, {values} values")]
	SlotValueLengthMismatch { slots: usize, values: usize },

	/// Not enough storage values provided for validator set
	#[error("Insufficient storage values: expected at least {expected}, got {got}")]
	InsufficientStorageValues { expected: usize, got: usize },

	/// Not enough pool IDs provided for validators
	#[error("Insufficient pool IDs: expected {expected} for {validators} validators, got {got}")]
	InsufficientPoolIds { expected: usize, validators: usize, got: usize },

	/// BLS public key slot value is missing
	#[error("Missing BLS public key slot value")]
	MissingBlsKeySlot,

	/// BLS key slot is empty
	#[error("Empty BLS key slot")]
	EmptyBlsKeySlot,

	/// Invalid short string length in BLS key slot
	#[error("Invalid short string length in BLS key")]
	InvalidBlsStringLength,

	/// BLS key string contains invalid UTF-8
	#[error("Invalid UTF-8 in BLS key string")]
	InvalidBlsKeyUtf8,

	/// Long string BLS keys require additional data slots
	#[error("Long string BLS key detected - string data slots required in proof")]
	LongStringBlsKeyUnsupported,

	/// BLS key hex string is invalid
	#[error("Invalid hex encoding in BLS key string")]
	InvalidBlsKeyHex,

	/// BLS key has incorrect byte length
	#[error("Invalid BLS key length: expected {expected}, got {got}")]
	InvalidBlsKeyLength { expected: usize, got: usize },

	/// Failed to convert BLS key bytes to the expected type
	#[error("Failed to convert BLS key bytes")]
	BlsKeyConversionFailed,

	/// Epoch in claimed validator set doesn't match decoded value
	#[error("Validator set epoch mismatch: claimed {claimed}, decoded {decoded}")]
	ValidatorSetEpochMismatch { claimed: u64, decoded: u64 },

	/// Validator count doesn't match between claimed and decoded sets
	#[error("Validator count mismatch: claimed {claimed}, decoded {decoded}")]
	ValidatorCountMismatch { claimed: usize, decoded: usize },

	/// Total stake doesn't match between claimed and decoded sets
	#[error("Total stake mismatch: claimed {claimed}, decoded {decoded}")]
	TotalStakeMismatch { claimed: U256, decoded: U256 },

	/// Validator address doesn't match at given index
	#[error("Validator {index} address mismatch")]
	ValidatorAddressMismatch { index: usize },

	/// Validator BLS key doesn't match at given index
	#[error("Validator {index} BLS key mismatch")]
	ValidatorBlsKeyMismatch { index: usize },

	/// Validator pool ID doesn't match at given index
	#[error("Validator {index} pool ID mismatch")]
	ValidatorPoolIdMismatch { index: usize },

	/// Validator stake doesn't match at given index
	#[error("Validator {index} stake mismatch: claimed {claimed}, decoded {decoded}")]
	ValidatorStakeMismatch { index: usize, claimed: U256, decoded: U256 },

	/// Validator set contains no validators
	#[error("Validator set is empty")]
	EmptyValidatorSet,

	/// Computed total stake doesn't match claimed total
	#[error("Total stake mismatch: computed {computed}, claimed {claimed}")]
	ComputedStakeMismatch { computed: U256, claimed: U256 },

	/// Duplicate validator detected in the set
	#[error("Duplicate validator in set")]
	DuplicateValidator,

	/// Validator has zero stake
	#[error("Validator has zero stake")]
	ZeroStakeValidator,
}
