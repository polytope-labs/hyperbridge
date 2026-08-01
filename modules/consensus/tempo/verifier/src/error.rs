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

use ismp::error::Error as IsmpError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
	/// Input ended before the expected structure was fully decoded.
	UnexpectedEndOfInput,
	/// A varint exceeded the 64-bit range.
	VarintOverflow,
	/// A varint used a non-minimal (non-canonical) encoding.
	NonMinimalVarint,
	/// A decoded length or count does not fit in the target's `usize`.
	LengthOverflow,
	/// Decoded structure was followed by unexpected trailing bytes.
	TrailingBytes,
	/// The DKG outcome embedded in a boundary header was structurally invalid.
	InvalidDkgOutcome,
	/// Failed to decode the SCALE-encoded client update.
	DecodeClientUpdate,
	/// Failed to decode the SCALE-encoded consensus state.
	DecodeConsensusState,
	/// A BLS point failed to decompress or was not in the prime-order subgroup.
	InvalidPoint,
	/// The threshold signature did not verify against the network identity.
	SignatureVerificationFailed,
	/// The certificate's epoch precedes the activation epoch of the network
	/// identity held by the client.
	EpochBeforeIdentity { certificate_epoch: u64, from_epoch: u64 },
	/// The supplied header does not hash to the finalized digest.
	DigestMismatch,
	/// The update does not advance the finalized height.
	ExpiredUpdate { current: u64, update: u64 },
	/// A boundary block's DKG outcome was recorded for an unexpected epoch.
	DkgOutcomeEpochMismatch { expected: u64, got: u64 },
	/// The boundary block header is missing its DKG outcome.
	MissingDkgOutcome,
	/// The update would advance past an epoch boundary the client has not
	/// verified, which would let it skip an identity rotation.
	UnprocessedEpochBoundary { finalized_height: u64, update_height: u64 },
	/// A boundary block tried to change the group key without the previous
	/// boundary having announced a full DKG ceremony.
	UnannouncedIdentityRotation { epoch: u64 },
	/// A rotation carried a group key that is not a valid G2 point.
	InvalidIdentity,
	/// The two certificates in a fraud proof do not conflict.
	InvalidFraudProof,
	/// The consensus state has an invalid configuration.
	InvalidConsensusState,
}

impl core::fmt::Display for Error {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Error::UnexpectedEndOfInput => write!(f, "unexpected end of input"),
			Error::VarintOverflow => write!(f, "varint exceeds 64 bits"),
			Error::NonMinimalVarint => write!(f, "varint uses a non-minimal encoding"),
			Error::LengthOverflow => write!(f, "decoded length does not fit in usize"),
			Error::TrailingBytes => write!(f, "unexpected trailing bytes"),
			Error::InvalidDkgOutcome => write!(f, "structurally invalid DKG outcome"),
			Error::DecodeClientUpdate => write!(f, "failed to decode TempoClientUpdate"),
			Error::DecodeConsensusState => write!(f, "failed to decode ConsensusState"),
			Error::InvalidPoint => write!(f, "invalid BLS12-381 point"),
			Error::SignatureVerificationFailed => {
				write!(f, "threshold signature verification failed")
			},
			Error::EpochBeforeIdentity { certificate_epoch, from_epoch } => write!(
				f,
				"certificate epoch {certificate_epoch} precedes identity activation epoch {from_epoch}"
			),
			Error::DigestMismatch => write!(f, "header hash does not match finalized digest"),
			Error::ExpiredUpdate { current, update } => {
				write!(f, "expired update: finalized height {current}, update height {update}")
			},
			Error::DkgOutcomeEpochMismatch { expected, got } => {
				write!(f, "DKG outcome epoch mismatch: expected {expected}, got {got}")
			},
			Error::MissingDkgOutcome => write!(f, "boundary header is missing its DKG outcome"),
			Error::UnprocessedEpochBoundary { finalized_height, update_height } => write!(
				f,
				"update at height {update_height} would skip the epoch boundary after finalized height {finalized_height}"
			),
			Error::UnannouncedIdentityRotation { epoch } => {
				write!(f, "epoch {epoch} rotated the group key without an announced full DKG")
			},
			Error::InvalidIdentity => write!(f, "rotation carried an invalid group public key"),
			Error::InvalidFraudProof => write!(f, "fraud proof certificates do not conflict"),
			Error::InvalidConsensusState => write!(f, "invalid consensus state"),
		}
	}
}

impl From<Error> for IsmpError {
	fn from(error: Error) -> Self {
		IsmpError::Custom(alloc::format!("{error}"))
	}
}
