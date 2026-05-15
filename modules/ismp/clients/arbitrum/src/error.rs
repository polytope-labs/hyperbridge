// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Typed errors for the Arbitrum consensus support — both the
//! `arbitrum-verifier` proof functions and the `ismp-arbitrum` client
//! wrapper.
//!
//! The same enum spans both layers so the ismp client doesn't have to
//! redefine variants; downstream callers map to `ismp::error::Error`
//! via the `From` impl below.

use alloc::string::{String, ToString};
use ismp::host::StateMachine;
use thiserror::Error;

/// Failure modes for Arbitrum consensus proof verification and the
/// ismp client wrapper.
#[derive(Debug, Error)]
pub enum Error {
	// -- verifier-level (produced by `verify_arbitrum_payload`, `verify_arbitrum_bold`) --
	/// The Arbitrum header's `extra_data` doesn't match the send root
	/// recorded in the global state.
	#[error("Arbitrum header extra data does not match send root in global state")]
	HeaderExtraDataMismatch,
	/// The hash of the Arbitrum header doesn't match the block hash
	/// recorded in the global state.
	#[error("Arbitrum header hash does not match block hash in global state")]
	HeaderHashMismatch,
	/// The state-hash slot couldn't be RLP-decoded from the storage proof.
	#[error("Error decoding state hash from storage: {0}")]
	DecodeStateHash(String),
	/// The assertion hash isn't present in the rollup-core `_assertions` map.
	#[error("Assertion provided is invalid")]
	InvalidAssertion,
	/// The parent assertion isn't present in the rollup-core
	/// `_assertions` map, so we can't check challenge state.
	#[error("Parent assertion not found in proof — cannot check challenge state")]
	ParentAssertionNotFound,
	/// The parent `AssertionNode` storage word couldn't be RLP-decoded.
	#[error("Error decoding parent assertion word: {0}")]
	DecodeParentAssertionWord(String),
	/// The parent `AssertionNode` storage word is longer than 32 bytes.
	#[error("Parent AssertionNode storage word longer than 32 bytes")]
	ParentAssertionTooLong,
	/// The parent assertion has a non-zero `secondChildBlock` — i.e.,
	/// this branch is in challenge.
	#[error("Assertion has been challenged: parent.secondChildBlock != 0")]
	AssertionChallenged,

	// -- ismp-arbitrum client wrapper --
	/// The submitted consensus proof failed to SCALE-decode into an `ArbitrumUpdate`.
	#[error("Cannot decode arbitrum update")]
	DecodeArbitrumUpdate,
	/// The submitted trusted state failed to SCALE-decode into a `ConsensusState`.
	#[error("Cannot decode trusted consensus state")]
	DecodeConsensusState,
	/// Fraud-proof verification is not implemented for this client.
	#[error("Fraud proof verification unimplemented")]
	FraudProofUnimplemented,
	/// Asked to serve a state machine the client doesn't support.
	#[error("State machine not supported: {0:?}")]
	UnsupportedStateMachine(StateMachine),

	// -- forwarding for ismp client --
	/// An ISMP-level error surfaced from a nested call (kept as-is
	/// rather than stringified, so we can re-emit it unchanged). Carries
	/// e.g. `MembershipProofVerificationFailed` from the membership
	/// helpers in `evm-state-machine`.
	#[error("{0:?}")]
	Ismp(#[from] ismp::error::Error),
}

impl From<Error> for ismp::error::Error {
	fn from(value: Error) -> Self {
		match value {
			Error::Ismp(err) => err,
			other => ismp::error::Error::Custom(other.to_string()),
		}
	}
}
