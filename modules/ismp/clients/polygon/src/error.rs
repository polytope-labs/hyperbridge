// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Typed errors for the Polygon (Heimdall + Bor EVM) consensus client.

use alloc::string::{String, ToString};
use ismp::host::StateMachine;
use thiserror::Error;

/// Failure modes for Polygon consensus and milestone proof verification.
#[derive(Debug, Error)]
pub enum Error {
	/// The submitted consensus proof failed to SCALE-decode into a `PolygonConsensusUpdate`.
	#[error("Cannot decode polygon consensus update: {0}")]
	DecodeConsensusUpdate(String),
	/// The submitted trusted state failed to SCALE-decode into a `ConsensusState`.
	#[error("Cannot decode trusted consensus state: {0}")]
	DecodeConsensusState(String),
	/// Converting the codec-wrapped tendermint proof into its native form failed.
	#[error("Cannot convert tendermint consensus proof: {0}")]
	ConvertTendermintProof(String),
	/// Converting the stored codec trusted state into its native form failed.
	#[error("Cannot convert tendermint trusted state: {0}")]
	ConvertTrustedState(String),
	/// `verify_header_update` rejected the tendermint header update.
	#[error("Tendermint header update verification failed: {0}")]
	VerifyHeaderUpdate(String),
	/// The Bor EVM header hash doesn't match the hash committed in the milestone.
	#[error("EVM header hash does not match milestone hash: {evm:?} != {milestone:?}")]
	EvmHeaderHashMismatch {
		/// Hash computed from the submitted EVM header.
		evm: alloc::vec::Vec<u8>,
		/// Hash committed in the milestone.
		milestone: alloc::vec::Vec<u8>,
	},
	/// The milestone's declared `end_block` doesn't match the submitted EVM header's number.
	#[error("Milestone end block does not match EVM header number")]
	MilestoneEndBlockMismatch,
	/// The submitted EVM header would rewind the consensus state past `last_finalized_block`.
	#[error("EVM header number is less than last finalized block")]
	EvmHeaderBehindFinalized,
	/// The ICS23 commitment proof bytes failed to decode.
	#[error("Cannot decode ICS23 commitment proof: {0}")]
	DecodeCommitmentProof(String),
	/// The ICS23 merkle proof failed to construct from the commitment proof.
	#[error("Cannot construct ICS23 merkle proof: {0}")]
	ConstructMerkleProof(String),
	/// Membership verification against the Heimdall app-hash failed.
	#[error("Milestone membership proof verification failed: {0}")]
	MembershipProofFailed(String),
	/// Fraud proofs were submitted for different block heights.
	#[error("Fraud proofs must be for the same block height")]
	FraudProofsDifferentHeight,
	/// Both fraud proofs commit to the same Tendermint header, so they fail to
	/// demonstrate equivocation.
	#[error("Fraud proofs commit to the same block header")]
	FraudProofsIdentical,
	/// Asked to serve a state machine the client doesn't support.
	#[error("Unsupported state machine or chain ID: {0:?}")]
	UnsupportedStateMachine(StateMachine),
	/// An ISMP-level error surfaced from a nested call.
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
