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

use thiserror::Error;

/// Errors that can occur during proof generation.
#[derive(Error, Debug)]
pub enum ProverError {
	/// HTTP request failed
	#[error("HTTP request failed: {0}")]
	HttpRequest(#[from] reqwest::Error),

	/// JSON deserialization failed
	#[error("JSON deserialization failed")]
	JsonDeserialization,

	/// JSON-RPC returned an error
	#[error("RPC error (code {code}): {message}")]
	RpcError { code: i64, message: String },

	/// RPC response missing result field
	#[error("RPC response missing result")]
	MissingRpcResult,

	/// Block not found at the specified height
	#[error("Block not found: {0}")]
	BlockNotFound(u64),

	/// Block proof not available (debug_getBlockProof may be disabled)
	#[error("Block proof not available for block {0}")]
	BlockProofNotAvailable(u64),

	/// Hex decoding failed
	#[error("Invalid hex encoding")]
	HexDecode,

	/// Invalid number format
	#[error("Invalid number format")]
	InvalidNumber,

	/// Invalid address length
	#[error("Invalid address length: expected 20, got {0}")]
	InvalidAddressLength(usize),

	/// Invalid H256 length
	#[error("Invalid H256 length: expected 32, got {0}")]
	InvalidH256Length(usize),

	/// Invalid logs bloom length
	#[error("Invalid logs bloom length: expected 256, got {0}")]
	InvalidLogsBloomLength(usize),

	/// Invalid BLS public key length
	#[error("Invalid BLS public key length: expected 48, got {0}")]
	InvalidBlsKeyLength(usize),

	/// Invalid BLS signature length
	#[error("Invalid BLS signature length: expected 96, got {0}")]
	InvalidBlsSignatureLength(usize),

	/// Validator set proof required but not available
	#[error("Validator set proof required but not available")]
	ValidatorSetProofRequired,

	/// Storage proof verification failed
	#[error("Storage proof verification failed")]
	StorageProofVerification,

	/// Missing storage value
	#[error("Missing storage value at slot index {0}")]
	MissingStorageValue(usize),
}
