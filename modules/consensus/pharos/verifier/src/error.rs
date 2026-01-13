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

use alloc::string::String;
use primitive_types::U256;
use sync_committee_primitives::constants::BlsPublicKey;
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

	/// Invalid state proof for validator set
	#[error("Invalid state proof: {0}")]
	InvalidStateProof(String),

	/// Invalid merkle proof
	#[error("Invalid merkle proof: {0}")]
	InvalidMerkleProof(String),

	/// Account proof verification failed
	#[error("Account proof verification failed: {0}")]
	AccountProofFailed(String),

	/// Storage proof verification failed
	#[error("Storage proof verification failed: {0}")]
	StorageProofFailed(String),

	/// General verification error
	#[error("Verification failed: {0}")]
	VerificationFailed(String),

	/// Header hash mismatch
	#[error("Header hash mismatch: expected {expected:?}, got {actual:?}")]
	HeaderHashMismatch {
		/// Expected hash
		expected: primitive_types::H256,
		/// Actual hash
		actual: primitive_types::H256,
	},

	/// Invalid header data
	#[error("Invalid header: {0}")]
	InvalidHeader(String),
}
