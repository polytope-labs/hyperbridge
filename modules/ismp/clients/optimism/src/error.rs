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
use primitive_types::H160;
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
	/// The `FaultDisputeGame.claimData` length slot couldn't be RLP-decoded.
	#[error("Error decoding claimData length value {0}")]
	DecodeClaimData(String),
	/// The `claimData` length storage word is longer than 32 bytes.
	#[error("claimData length storage value longer than 32 bytes")]
	ClaimDataTooLong,
	/// The `AggregateVerifier.counteredByIntermediateRootIndexPlusOne` storage value
	/// couldn't be RLP-decoded.
	#[error("Error decoding counteredByIntermediateRootIndexPlusOne value {0}")]
	DecodeCounteredBy(String),
	/// The `counteredByIntermediateRootIndexPlusOne` storage value is longer than 32 bytes.
	#[error("counteredByIntermediateRootIndexPlusOne value longer than 32 bytes")]
	CounteredByTooLong,
	/// The L2 output root slot wasn't present in the storage proof.
	#[error("Output root slot not found in L2Oracle storage proof")]
	OutputRootSlotMissing,
	/// The recovered output root from the proof doesn't match the value
	/// computed from `(version, state_root, withdrawal_storage_root, l2_block_hash)`.
	#[error("Invalid optimism output root proof")]
	OutputRootMismatch,
	/// The block-number-and-timestamp slot wasn't present in the storage proof.
	#[error("Block-and-timestamp slot not found in L2Oracle storage proof")]
	BlockTimestampSlotMissing,
	/// The recovered block number or timestamp doesn't match the value in the payload.
	#[error("Invalid optimism block and timestamp proof")]
	BlockTimestampMismatch,
	/// The proof's `game_type` isn't in the configured set of respected game types.
	#[error("Game type {0} is not in the respected game types")]
	UnsupportedGameType(u32),
	/// The dispute game's id wasn't present in the `_disputeGames` map proof.
	#[error("Dispute Game's Id not found in proof")]
	DisputeGameIdMissing,
	/// The dispute game id recovered from the proof doesn't match the one
	/// derived from `(game_type, timestamp, proxy)`.
	#[error("Dispute Game Id from proof does not match derived game id")]
	DisputeGameIdMismatch,
	/// The `gameImpls[gameType]` slot wasn't present in the factory storage proof.
	#[error("gameImpls[gameType] not found in factory storage")]
	GameImplsMissing,
	/// The implementation address proved by `gameImpls[gameType]` doesn't match the configured
	/// `expected_impl` for this game type.
	#[error("gameImpls[{game_type}] is {actual:?}, expected {expected:?}")]
	GameImplMismatch {
		/// The game type whose implementation was being checked.
		game_type: u32,
		/// The implementation address recovered from the factory proof.
		actual: H160,
		/// The implementation address configured for this game type.
		expected: H160,
	},
	/// The `FaultDisputeGame.claimData` length slot wasn't present in the proxy storage proof.
	#[error("claimData length slot not found in proxy storage")]
	ClaimDataSlotMissing,
	/// The `FaultDisputeGame.claimData.length` is not `1` — a `move()` has appended a
	/// counter-claim, so the game has been challenged.
	#[error("FaultDisputeGame has been challenged: claimData.length != 1")]
	FaultDisputeGameChallenged,
	/// The `AggregateVerifier.counteredByIntermediateRootIndexPlusOne` value is non-zero — the
	/// game is challenged.
	#[error(
		"AggregateVerifier game has been challenged: \
		counteredByIntermediateRootIndexPlusOne != 0"
	)]
	AggregateVerifierChallenged,
	/// The dispute game proxy referenced by this proof has been blacklisted by the fishermen
	/// pallet. The consensus verifier refuses to process any further proofs for it.
	#[error("Dispute game proxy {0:?} has been blacklisted by fishermen")]
	DisputeGameBlacklisted(H160),

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
