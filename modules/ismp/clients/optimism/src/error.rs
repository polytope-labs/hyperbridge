// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Typed errors for the Optimism (Op Stack) consensus support — both
//! the `op-verifier` proof functions and the `ismp-optimism` client
//! wrapper.
//!
//! The same enum spans both layers so the ismp client doesn't have to
//! redefine variants; downstream callers map to `ismp::error::Error`
//! via the `From` impl below.

use alloc::string::{String, ToString};
use ismp::host::StateMachine;
use thiserror::Error;

/// Failure modes for Op Stack consensus proof verification and the
/// ismp client wrapper.
#[derive(Debug, Error)]
pub enum Error {
	// -- verifier-level --
	/// The L2 output root slot couldn't be RLP-decoded from the proof.
	#[error("Error decoding output root from {0}")]
	DecodeOutputRoot(String),
	/// The block-number-and-timestamp slot couldn't be RLP-decoded.
	#[error("Error decoding block and timestamp from {0}")]
	DecodeBlockTimestamp(String),
	/// The dispute-game id slot couldn't be RLP-decoded.
	#[error("Error decoding dispute game id from {0}")]
	DecodeDisputeGameId(String),
	/// A generic storage-trie leaf couldn't be RLP-decoded.
	#[error("Error decoding storage value {0}")]
	DecodeStorageValue(String),
	/// A storage-trie leaf is longer than 32 bytes — not a valid uint256/address.
	#[error("Storage value longer than 32 bytes")]
	StorageValueTooLong,
	/// The `FaultDisputeGame.claimData[0]` storage word couldn't be RLP-decoded.
	#[error("Error decoding claimData[0] value {0}")]
	DecodeClaimData(String),
	/// The `claimData[0]` storage word is longer than 32 bytes.
	#[error("claimData[0] storage value longer than 32 bytes")]
	ClaimDataTooLong,
	/// The `AggregateVerifier.counteredByIntermediateRootIndexPlusOne` storage value
	/// couldn't be RLP-decoded.
	#[error("Error decoding counteredByIntermediateRootIndexPlusOne value {0}")]
	DecodeCounteredBy(String),
	/// The `counteredByIntermediateRootIndexPlusOne` storage value is longer than 32 bytes.
	#[error("counteredByIntermediateRootIndexPlusOne value longer than 32 bytes")]
	CounteredByTooLong,

	// -- ismp-optimism client wrapper --
	/// The submitted consensus proof failed to SCALE-decode into an `OptimismUpdate`.
	#[error("Cannot decode optimism update")]
	DecodeOptimismUpdate,
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
