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

//! Error types for the Arc prover.

use thiserror::Error;

/// Errors that can occur while proving Arc consensus.
#[derive(Debug, Error)]
pub enum ProverError {
	/// Invalid RPC endpoint URL
	#[error("Invalid RPC url: {0}")]
	InvalidUrl(String),

	/// HTTP transport error
	#[error("HTTP error: {0}")]
	Http(#[from] reqwest::Error),

	/// The RPC node returned an error object
	#[error("RPC error {code}: {message}")]
	RpcError {
		/// JSON-RPC error code
		code: i64,
		/// JSON-RPC error message
		message: String,
	},

	/// The RPC response had neither a result nor an error
	#[error("RPC response missing result")]
	MissingRpcResult,

	/// Alloy provider error
	#[error("Provider error: {0}")]
	ProviderError(String),

	/// The requested block does not exist
	#[error("Block {0} not found")]
	BlockNotFound(u64),

	/// A hex field could not be decoded
	#[error("Invalid hex in RPC response")]
	HexDecode,

	/// A base64 field could not be decoded
	#[error("Invalid base64 in RPC response")]
	Base64Decode,

	/// A fixed-length field had the wrong length
	#[error("Invalid length {got} for {field}, expected {expected}")]
	InvalidLength {
		/// Field name
		field: &'static str,
		/// Expected byte length
		expected: usize,
		/// Actual byte length
		got: usize,
	},

	/// A numeric field could not be parsed
	#[error("Invalid number in RPC response")]
	InvalidNumber,

	/// The certificate references a different block than requested
	#[error("Certificate mismatch: {0}")]
	CertificateMismatch(String),

	/// The anchor block of a `"latest"`-anchored proof could not be identified
	#[error(
		"Could not identify the anchor block of a latest-anchored proof in [{lower}, {upper}]"
	)]
	AnchorNotFound {
		/// Tip observed before the proof request
		lower: u64,
		/// Tip observed after the proof request
		upper: u64,
	},

	/// Verification of a fetched update failed
	#[error("Verification error: {0}")]
	Verifier(#[from] arc_verifier::error::Error),
}
