// Copyright (C) 2022 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

//! Typed errors for the BSC consensus support — both the verifier
//! (`verify_bsc_header`) and the `ismp-bsc` client wrapper.
//!
//! The same enum spans both layers so the ismp client doesn't have to
//! redefine variants; downstream callers map to `ismp::error::Error`
//! via the `From` impl below.

use alloc::string::{String, ToString};
use ismp::host::StateMachine;

/// Failure modes for BSC consensus proof verification and the ismp
/// client wrapper.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	// -- verifier-level (produced by `verify_bsc_header`) --
	/// The header's `extra_data` couldn't be parsed.
	#[error("Could not parse extra data from header")]
	ParseExtraData,
	/// The header's `extra_data` for the epoch header couldn't be parsed.
	#[error("Could not parse extra data from epoch header")]
	ParseEpochExtraData,
	/// Vote data is empty (source or target hash is zero).
	#[error("Vote data is empty")]
	EmptyVoteData,
	/// The SSZ vote address set couldn't be deserialised.
	#[error("Could not deserialize vote address set")]
	DeserializeVoteAddressSet,
	/// The vote address set has bits set beyond the validator-count range.
	#[error("Vote address set has bits set beyond validator count")]
	VoteAddressSetBeyondValidatorCount,
	/// Fewer than the 2/3 supermajority signed the vote.
	#[error("Not enough participants")]
	NotEnoughParticipants,
	/// The hashed source/target headers don't match the vote data.
	#[error("Target and source headers do not match vote data")]
	HeaderVoteDataMismatch,
	/// One or more participant public keys failed to decompress.
	#[error("Failed to aggregate participant public keys: {0}")]
	AggregatePublicKeys(String),
	/// The aggregate BLS signature failed verification.
	#[error("Could not verify aggregate signature")]
	InvalidSignature,
	/// The epoch ancestry doesn't chain back to the source header.
	#[error("Epoch ancestry submitted is invalid")]
	InvalidEpochAncestry,
	/// The epoch header has no validator set encoded.
	#[error("Epoch header provided does not have a validator set present in its extra data")]
	MissingValidatorSet,

	// -- client-wrapper (produced by `ismp-bsc`) --
	/// The submitted consensus proof failed to SCALE-decode into a `BscClientUpdate`.
	#[error("Cannot decode BSC client update")]
	DecodeBscClientUpdate,
	/// The submitted trusted state failed to SCALE-decode into a `ConsensusState`.
	#[error("Cannot decode trusted consensus state")]
	DecodeConsensusState,
	/// The submitted update is at or below the current finalized height.
	#[error("Expired update: source header {update} at or below finalized {current}")]
	ExpiredUpdate {
		/// Currently finalized BSC block height.
		current: u64,
		/// Source header height in the submitted update.
		update: u64,
	},
	/// No epoch length configured for the client.
	#[error("Epoch length not set")]
	EpochLengthNotSet,
	/// During an authority-set rotation, the source header must be from
	/// the same epoch as the attested header.
	#[error(
		"Source header epoch {source_epoch} does not match attested epoch {attested_epoch} \
		 during authority set rotation"
	)]
	SourceHeaderEpochMismatch {
		/// Epoch derived from the attested header number.
		attested_epoch: u64,
		/// Epoch derived from the source header number.
		source_epoch: u64,
	},
	/// Fraud-proof header pair didn't satisfy "same height, different hash".
	#[error("Invalid fraud proof")]
	InvalidFraudProof,
	/// Asked to serve a state machine the client doesn't support.
	#[error("Unsupported state machine: {0:?}")]
	UnsupportedStateMachine(StateMachine),
}

impl From<Error> for ismp::error::Error {
	fn from(value: Error) -> Self {
		ismp::error::Error::Custom(value.to_string())
	}
}
