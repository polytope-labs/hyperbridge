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

//! RPC client for Arc node interactions.
//!
//! Standard Ethereum JSON-RPC calls (`eth_blockNumber`, `eth_getBlockByNumber`)
//! go through the alloy provider. `arc_getCertificate` (Arc's commit
//! certificate endpoint, proxied by the execution node) and `eth_getProof` are
//! called through a raw reqwest client.

use crate::ProverError;
use alloy_eips::BlockNumberOrTag;
use alloy_provider::{Provider, RootProvider};
use arc_primitives::{CommitCertificate, CommitSignature};
use base64::Engine;
use ethabi::ethereum_types::H64;
use geth_primitives::CodecHeader;
use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// JSON-RPC request structure.
#[derive(Debug, Serialize)]
struct JsonRpcRequest<P> {
	pub jsonrpc: &'static str,
	pub method: &'static str,
	pub params: P,
	pub id: u64,
}

/// JSON-RPC response structure.
#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
	pub result: Option<T>,
	pub error: Option<JsonRpcError>,
}

/// JSON-RPC error structure.
#[derive(Debug, Deserialize)]
struct JsonRpcError {
	pub code: i64,
	pub message: String,
}

/// Signature entry within an `arc_getCertificate` response
/// (base64-encoded ed25519 signature bytes).
#[derive(Debug, Clone, Deserialize)]
pub struct RpcCommitSignature {
	/// Validator consensus address as a 0x hex string
	pub address: String,
	/// Base64-encoded 64-byte ed25519 signature
	pub signature: String,
}

/// Commit certificate JSON as returned by `arc_getCertificate` and the
/// consensus node's `GET /commit?height=` endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct RpcCommitCertificate {
	/// The certified height
	pub height: u64,
	/// The consensus round the value was decided in
	pub round: i64,
	/// The decided execution block hash as a 0x hex string
	pub block_hash: String,
	/// Precommit signatures
	pub signatures: Vec<RpcCommitSignature>,
}

impl RpcCommitCertificate {
	/// Convert the wire JSON into a [`CommitCertificate`].
	pub fn try_into_certificate(self) -> Result<CommitCertificate, ProverError> {
		let commit_signatures = self
			.signatures
			.into_iter()
			.map(|entry| {
				let address = hex_to_h160(&entry.address)?;
				let bytes = base64::engine::general_purpose::STANDARD
					.decode(&entry.signature)
					.map_err(|_| ProverError::Base64Decode)?;
				let signature: [u8; 64] = bytes.try_into().map_err(|v: Vec<u8>| {
					ProverError::InvalidLength { field: "signature", expected: 64, got: v.len() }
				})?;
				Ok(CommitSignature { address, signature })
			})
			.collect::<Result<Vec<_>, ProverError>>()?;

		let round = u32::try_from(self.round).map_err(|_| ProverError::InvalidNumber)?;

		Ok(CommitCertificate {
			height: self.height,
			round,
			block_hash: hex_to_h256(&self.block_hash)?,
			commit_signatures,
		})
	}
}

/// Account proof response from `eth_getProof` (standard EIP-1186 format).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcAccountProof {
	/// Merkle-Patricia proof of the account
	pub account_proof: Vec<String>,
	/// Account storage root
	pub storage_hash: String,
	/// Per-slot storage proofs
	pub storage_proof: Vec<RpcStorageProof>,
}

/// Storage proof entry from `eth_getProof`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcStorageProof {
	/// The storage slot key
	pub key: String,
	/// The slot's value
	pub value: String,
	/// Merkle-Patricia proof path for the slot
	pub proof: Vec<String>,
}

/// RPC client for an Arc node.
pub struct ArcRpcClient {
	endpoint: String,
	/// Endpoint used for `arc_getCertificate`. Third-party providers
	/// (e.g. Alchemy) often don't proxy Arc's custom namespace, so
	/// certificates can be sourced from a different node.
	certificate_endpoint: String,
	client: reqwest::Client,
	provider: RootProvider,
	request_id: AtomicU64,
}

impl ArcRpcClient {
	/// Create a new RPC client for the given endpoint URL.
	pub fn new(endpoint: impl Into<String>) -> Result<Self, ProverError> {
		let endpoint = endpoint.into();
		Self::with_certificate_endpoint(endpoint.clone(), endpoint)
	}

	/// Create a new RPC client that sources certificates from a separate
	/// endpoint.
	pub fn with_certificate_endpoint(
		endpoint: impl Into<String>,
		certificate_endpoint: impl Into<String>,
	) -> Result<Self, ProverError> {
		let endpoint = endpoint.into();
		let provider = RootProvider::new_http(
			endpoint.parse().map_err(|_| ProverError::InvalidUrl(endpoint.clone()))?,
		);
		Ok(Self {
			endpoint,
			certificate_endpoint: certificate_endpoint.into(),
			client: reqwest::Client::new(),
			provider,
			request_id: AtomicU64::new(1),
		})
	}

	fn next_id(&self) -> u64 {
		self.request_id.fetch_add(1, Ordering::SeqCst)
	}

	/// Make a raw JSON-RPC call for non-standard endpoints.
	async fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
		&self,
		method: &'static str,
		params: P,
	) -> Result<R, ProverError> {
		self.call_at(&self.endpoint, method, params).await
	}

	async fn call_at<P: Serialize, R: for<'de> Deserialize<'de>>(
		&self,
		endpoint: &str,
		method: &'static str,
		params: P,
	) -> Result<R, ProverError> {
		let request = JsonRpcRequest { jsonrpc: "2.0", method, params, id: self.next_id() };

		log::trace!(
			target: "arc-prover",
			"JSON-RPC request: method={method} endpoint={endpoint}",
		);

		let response = self.client.post(endpoint).json(&request).send().await?;
		let rpc_response: JsonRpcResponse<R> = response.json().await?;

		if let Some(error) = rpc_response.error {
			return Err(ProverError::RpcError { code: error.code, message: error.message });
		}

		rpc_response.result.ok_or(ProverError::MissingRpcResult)
	}

	/// Fetch the latest block number.
	pub async fn get_block_number(&self) -> Result<u64, ProverError> {
		self.provider
			.get_block_number()
			.await
			.map_err(|e| ProverError::ProviderError(e.to_string()))
	}

	/// Fetch a block header by number, converting the response to [`CodecHeader`].
	pub async fn get_block_by_number(&self, block_number: u64) -> Result<CodecHeader, ProverError> {
		let block = self
			.provider
			.get_block_by_number(BlockNumberOrTag::Number(block_number))
			.await
			.map_err(|e| ProverError::ProviderError(e.to_string()))?
			.ok_or(ProverError::BlockNotFound(block_number))?;

		let h = &block.header.inner;

		Ok(CodecHeader {
			parent_hash: H256::from(h.parent_hash.0),
			uncle_hash: H256::from(h.ommers_hash.0),
			coinbase: H160::from(h.beneficiary.0 .0),
			state_root: H256::from(h.state_root.0),
			transactions_root: H256::from(h.transactions_root.0),
			receipts_root: H256::from(h.receipts_root.0),
			logs_bloom: {
				let mut bloom = [0u8; 256];
				bloom.copy_from_slice(h.logs_bloom.as_ref());
				bloom.into()
			},
			difficulty: U256::from_big_endian(&h.difficulty.to_be_bytes::<32>()),
			number: U256::from(h.number),
			gas_limit: h.gas_limit,
			gas_used: h.gas_used,
			timestamp: h.timestamp,
			extra_data: h.extra_data.to_vec(),
			mix_hash: H256::from(h.mix_hash.0),
			nonce: H64::from(h.nonce.0),
			base_fee_per_gas: h.base_fee_per_gas.map(U256::from),
			withdrawals_hash: h.withdrawals_root.map(|v| H256::from(v.0)),
			blob_gas_used: h.blob_gas_used,
			excess_blob_gas_used: h.excess_blob_gas,
			parent_beacon_root: h.parent_beacon_block_root.map(|v| H256::from(v.0)),
			requests_hash: h.requests_hash.map(|v| H256::from(v.0)),
		})
	}

	/// Fetch the commit certificate for a height via `arc_getCertificate`.
	pub async fn get_certificate(
		&self,
		block_number: u64,
	) -> Result<RpcCommitCertificate, ProverError> {
		self.call_at(&self.certificate_endpoint, "arc_getCertificate", [block_number])
			.await
	}

	/// Read a storage slot via `eth_getStorageAt`.
	pub async fn get_storage_at(
		&self,
		address: H160,
		slot: H256,
		block_number: u64,
	) -> Result<H256, ProverError> {
		self.get_storage_at_block(address, slot, format!("0x{block_number:x}")).await
	}

	/// Read a storage slot at the node's latest block.
	pub async fn get_storage_at_latest(
		&self,
		address: H160,
		slot: H256,
	) -> Result<H256, ProverError> {
		self.get_storage_at_block(address, slot, "latest".into()).await
	}

	async fn get_storage_at_block(
		&self,
		address: H160,
		slot: H256,
		block: String,
	) -> Result<H256, ProverError> {
		let value: String = self
			.call("eth_getStorageAt", (format!("0x{address:x}"), format!("0x{slot:x}"), block))
			.await?;
		let bytes = hex_to_bytes(&value)?;
		if bytes.len() > 32 {
			return Err(ProverError::InvalidLength {
				field: "storage value",
				expected: 32,
				got: bytes.len(),
			});
		}
		let mut padded = [0u8; 32];
		padded[32 - bytes.len()..].copy_from_slice(&bytes);
		Ok(H256(padded))
	}

	/// Fetch account and storage proofs via `eth_getProof`.
	pub async fn get_proof(
		&self,
		address: H160,
		storage_keys: &[H256],
		block_number: u64,
	) -> Result<RpcAccountProof, ProverError> {
		self.get_proof_at_block(address, storage_keys, format!("0x{block_number:x}"))
			.await
	}

	/// Fetch account and storage proofs at the node's latest block.
	///
	/// Public Arc RPCs run with reth's default zero proof window, so proofs
	/// are only available for the node's current tip; the anchor block must be
	/// discovered afterwards by verifying the account proof against candidate
	/// headers' state roots.
	pub async fn get_proof_latest(
		&self,
		address: H160,
		storage_keys: &[H256],
	) -> Result<RpcAccountProof, ProverError> {
		self.get_proof_at_block(address, storage_keys, "latest".into()).await
	}

	async fn get_proof_at_block(
		&self,
		address: H160,
		storage_keys: &[H256],
		block: String,
	) -> Result<RpcAccountProof, ProverError> {
		let keys_hex: Vec<String> = storage_keys.iter().map(|k| format!("0x{k:x}")).collect();
		self.call("eth_getProof", (format!("0x{address:x}"), keys_hex, block)).await
	}
}

/// Parse a hex string to bytes.
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, ProverError> {
	let hex = hex.trim_start_matches("0x");
	// `eth_getStorageAt` may return odd-length quantities
	if hex.len() % 2 == 1 {
		let padded = format!("0{hex}");
		return hex::decode(padded).map_err(|_| ProverError::HexDecode);
	}
	hex::decode(hex).map_err(|_| ProverError::HexDecode)
}

/// Parse a hex string to H256.
pub fn hex_to_h256(hex: &str) -> Result<H256, ProverError> {
	let bytes = hex_to_bytes(hex)?;
	if bytes.len() != 32 {
		return Err(ProverError::InvalidLength { field: "hash", expected: 32, got: bytes.len() });
	}
	Ok(H256::from_slice(&bytes))
}

/// Parse a hex string to H160.
pub fn hex_to_h160(hex: &str) -> Result<H160, ProverError> {
	let bytes = hex_to_bytes(hex)?;
	if bytes.len() != 20 {
		return Err(ProverError::InvalidLength { field: "address", expected: 20, got: bytes.len() });
	}
	Ok(H160::from_slice(&bytes))
}
