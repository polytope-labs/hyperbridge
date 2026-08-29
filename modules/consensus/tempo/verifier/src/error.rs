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

use ismp::{error::Error as IsmpError, host::StateMachine};

/// Errors raised while verifying Tempo consensus proofs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
	/// Input ended before the expected structure was fully decoded.
	#[error("Unexpected end of input")]
	UnexpectedEndOfInput,
	/// A varint exceeded the 64-bit range.
	#[error("Varint exceeds 64 bits")]
	VarintOverflow,
	/// A varint used a non-minimal (non-canonical) encoding.
	#[error("Varint uses a non-minimal encoding")]
	NonMinimalVarint,
	/// A decoded length or count does not fit in the target's `usize`.
	#[error("Decoded length does not fit in usize")]
	LengthOverflow,
	/// Decoded structure was followed by unexpected trailing bytes.
	#[error("Unexpected trailing bytes")]
	TrailingBytes,
	/// The DKG outcome embedded in a boundary header was structurally invalid.
	#[error("Structurally invalid DKG outcome")]
	InvalidDkgOutcome,
	/// Failed to decode the SCALE-encoded client update.
	#[error("Cannot decode TempoClientUpdate")]
	DecodeClientUpdate,
	/// Failed to decode the SCALE-encoded consensus state.
	#[error("Cannot decode ConsensusState")]
	DecodeConsensusState,
	/// A BLS point failed to decompress or was not in the prime-order subgroup.
	#[error("Invalid BLS12-381 point")]
	InvalidPoint,
	/// The threshold signature did not verify against the network identity.
	#[error("Threshold signature verification failed")]
	SignatureVerificationFailed,
	/// The certificate's epoch precedes the activation epoch of the network
	/// identity held by the client.
	#[error(
		"Certificate epoch {certificate_epoch} precedes identity activation epoch {from_epoch}"
	)]
	EpochBeforeIdentity { certificate_epoch: u64, from_epoch: u64 },
	/// The supplied header does not hash to the finalized digest.
	#[error("Header hash does not match finalized digest")]
	DigestMismatch,
	/// The update does not advance the finalized height.
	#[error("Expired update: finalized height {current}, update height {update}")]
	ExpiredUpdate { current: u64, update: u64 },
	/// A boundary block's DKG outcome was recorded for an unexpected epoch.
	#[error("DKG outcome epoch mismatch: expected {expected}, got {got}")]
	DkgOutcomeEpochMismatch { expected: u64, got: u64 },
	/// The boundary block header is missing its DKG outcome.
	#[error("Boundary header is missing its DKG outcome")]
	MissingDkgOutcome,
	/// The update would advance past an epoch boundary the client has not
	/// verified, which would let it skip an identity rotation.
	#[error(
		"Update at height {update_height} would skip the epoch boundary after finalized height \
		 {finalized_height}"
	)]
	UnprocessedEpochBoundary { finalized_height: u64, update_height: u64 },
	/// A boundary block tried to change the group key without the previous
	/// boundary having announced a full DKG ceremony.
	#[error("Epoch {epoch} rotated the group key without an announced full DKG")]
	UnannouncedIdentityRotation { epoch: u64 },
	/// A rotation carried a group key that is not a valid G2 point.
	#[error("Rotation carried an invalid group public key")]
	InvalidIdentity,
	/// The two certificates in a fraud proof do not conflict.
	#[error("Fraud proof certificates do not conflict")]
	InvalidFraudProof,
	/// The consensus state has an invalid configuration.
	#[error("Invalid consensus state")]
	InvalidConsensusState,
	/// A state machine id that this client does not track.
	#[error("State machine not supported: {0:?}")]
	UnsupportedStateMachine(StateMachine),
}

impl From<Error> for IsmpError {
	fn from(error: Error) -> Self {
		IsmpError::AnyHow(anyhow::Error::new(error).into())
	}
}
