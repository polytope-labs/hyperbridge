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

use alloc::string::ToString;

/// Errors from the Kaia consensus verifier and client.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
	#[error("Update contains no headers")]
	EmptyUpdate,
	#[error("Update contains more headers than the maximum of {0}")]
	TooManyHeaders(usize),
	#[error("Header extra data is malformed")]
	InvalidExtraData,
	#[error("Header {header} does not advance the finalized height {finalized}")]
	StaleHeader { header: u64, finalized: u64 },
	#[error("Header {header} does not follow {previous}; updates must be strictly ascending")]
	UnorderedHeaders { header: u64, previous: u64 },
	#[error("The consensus state has a zero epoch length")]
	ZeroEpoch,
	#[error("The consensus state has a zero committee size")]
	ZeroCommitteeSize,
	#[error("The consensus state's council is empty, unsorted or holds duplicates")]
	MalformedCouncil,
	#[error("Header carries {found} committed seals, more than the council's {council}")]
	TooManySeals { found: u64, council: u64 },
	#[error(
		"Header declares validator {validator} that is not in the tracked council: \
		 a validator vote was skipped and the council has drifted"
	)]
	CouncilDrift { validator: primitive_types::H160 },
	#[error("Malformed proposer or committed seal")]
	InvalidSeal,
	#[error("Recovered block author is not a council member")]
	UnauthorizedAuthor,
	#[error("Header carries {found} council committed seals, quorum is {required}")]
	InsufficientSeals { found: u64, required: u64 },
	#[error("Header vote data is malformed")]
	InvalidVoteData,
	#[error("Vote voter does not match the block author")]
	VoteNotFromAuthor,
	#[error("Header governance data is malformed")]
	InvalidGovernanceData,
	#[error("Failed to decode the client update")]
	DecodeClientUpdate,
	#[error("Failed to decode the consensus state")]
	DecodeConsensusState,
	#[error("Fraud proof headers are not at the same height")]
	FraudProofHeightMismatch,
	#[error("Fraud proof headers are identical")]
	FraudProofIdenticalHeaders,
}

impl From<Error> for ismp::error::Error {
	fn from(value: Error) -> Self {
		ismp::error::Error::Custom(value.to_string())
	}
}
