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

//! Error types for the Arc verifier.

use alloc::string::String;
use primitive_types::{H160, H256};
use thiserror::Error;

/// Errors that can occur during Arc consensus verification.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
	/// The update does not advance the finalized height
	#[error("Stale update: trusted height {current}, update height {update}")]
	StaleUpdate {
		/// The trusted finalized height
		current: u64,
		/// The update's height
		update: u64,
	},

	/// The certificate height does not match the header
	#[error("Certificate height {certificate} does not match header number {header}")]
	HeightMismatch {
		/// Height in the commit certificate
		certificate: u64,
		/// The header's block number
		header: u64,
	},

	/// The certificate's block hash does not match the supplied header
	#[error("Certificate block hash {certificate} does not match computed header hash {computed}")]
	BlockHashMismatch {
		/// Block hash committed to by the certificate
		certificate: H256,
		/// keccak256 of the RLP-encoded header
		computed: H256,
	},

	/// A validator signed the certificate more than once
	#[error("Duplicate commit signature from {address}")]
	DuplicateVote {
		/// The offending validator address
		address: H160,
	},

	/// A commit signature is from an address outside the trusted validator set
	#[error("Commit signature from unknown validator {address}")]
	UnknownValidator {
		/// The unknown signer address
		address: H160,
	},

	/// A commit signature failed ed25519 verification
	#[error("Invalid signature from validator {address}")]
	InvalidSignature {
		/// The validator whose signature failed
		address: H160,
	},

	/// The signatures do not amount to more than 2/3 of the voting power
	#[error("Insufficient voting power: signed {signed}, total {total}")]
	InsufficientVotingPower {
		/// Voting power that validly signed
		signed: u64,
		/// Total voting power of the trusted set
		total: u64,
	},

	/// The validator set storage proof is invalid or incomplete
	#[error("Validator set proof error: {0}")]
	ValidatorSetProof(String),

	/// The proven active validator set is unusable (empty or power overflow)
	#[error("Invalid validator set: {0}")]
	InvalidValidatorSet(String),
}
