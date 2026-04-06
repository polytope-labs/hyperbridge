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

//! RPC client for Pharos node interactions.
//!
//! Standard Ethereum JSON-RPC calls (`eth_blockNumber`, `eth_getBlockByNumber`)
//! are handled via the alloy provider. Pharos-specific endpoints that use
//! non-standard response formats (`eth_getProof`, `debug_getBlockProof`,
//! `debug_getValidatorInfo`) are called through a raw reqwest client.

use crate::ProverError;
use alloy_eips::BlockNumberOrTag;
use alloy_provider::{Provider, RootProvider};
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

impl<P> JsonRpcRequest<P> {
	fn new(method: &'static str, params: P, id: u64) -> Self {
		Self { jsonrpc: "2.0", method, params, id }
	}
}

/// JSON-RPC response structure.
#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
	#[allow(dead_code)]
	pub jsonrpc: String,
	pub result: Option<T>,
	pub error: Option<JsonRpcError>,
	#[allow(dead_code)]
	pub id: u64,
}

/// JSON-RPC error structure.
#[derive(Debug, Deserialize)]
struct JsonRpcError {
	pub code: i64,
	pub message: String,
	#[allow(dead_code)]
	pub data: Option<serde_json::Value>,
}

/// Block proof response from `debug_getBlockProof`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcBlockProof {
	/// Block number as hex string (e.g., "0x1234")
	pub block_number: String,
	/// Block proof hash - the message that was signed
	pub block_proof_hash: String,
	/// Aggregated BLS signature as hex string
	pub bls_aggregated_signature: String,
	/// List of BLS public keys that signed, as hex strings
	pub signed_bls_keys: Vec<String>,
}

/// Single proof node in the Pharos hexary hash tree format.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcProofNode {
	/// Raw node bytes as hex string
	pub proof_node: String,
	/// Start offset where the child hash begins
	pub next_begin_offset: u32,
	/// End offset where the child hash ends
	pub next_end_offset: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcSiblingProof {
	pub slot_index: u8,
	pub leftmost_leaf_key: String,
	pub proof_path: Vec<RpcProofNode>,
}

/// Account proof response from `eth_getProof`.
///
/// Uses a custom response format (Pharos hexary hash tree nodes instead of
/// standard Ethereum MPT nodes), so this endpoint is called via raw JSON-RPC
/// rather than the alloy provider.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcAccountProof {
	pub account_proof: Vec<RpcProofNode>,
	pub balance: String,
	pub code_hash: String,
	pub nonce: String,
	pub storage_hash: String,
	/// RLP-encoded account value (rawValue)
	pub raw_value: String,
	pub storage_proof: Vec<RpcStorageProof>,
	pub is_exist: bool,
	#[serde(default)]
	pub sibling_leftmost_leaf_proofs: Vec<RpcSiblingProof>,
}

/// Storage proof entry from `eth_getProof`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcStorageProof {
	pub key: String,
	pub value: String,
	pub proof: Vec<RpcProofNode>,
	pub is_exist: bool,
	#[serde(default)]
	pub sibling_leftmost_leaf_proofs: Vec<RpcSiblingProof>,
}

/// Validator info from `debug_getValidatorInfo`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcValidatorInfo {
	pub bls_key: String,
	pub identity_key: String,
	pub staking: String,
	#[serde(rename = "validatorID")]
	pub validator_id: String,
}

/// Response from `debug_getValidatorInfo`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcValidatorInfoResponse {
	pub block_number: String,
	pub validator_set: Vec<RpcValidatorInfo>,
}

/// RPC client for Pharos node.
///
/// Uses an alloy provider for standard Ethereum JSON-RPC queries and a raw
/// reqwest client for Pharos-specific debug endpoints and `eth_getProof`
/// (which returns non-standard proof node formats).
pub struct PharosRpcClient {
	endpoint: String,
	client: reqwest::Client,
	provider: RootProvider,
	request_id: AtomicU64,
}

impl PharosRpcClient {
	/// Create a new RPC client for the given endpoint URL.
	pub fn new(endpoint: impl Into<String>) -> Result<Self, ProverError> {
		let endpoint = endpoint.into();
		let provider = RootProvider::new_http(
			endpoint.parse().map_err(|_| ProverError::InvalidUrl(endpoint.clone()))?,
		);
		Ok(Self {
			endpoint,
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
		let request = JsonRpcRequest::new(method, params, self.next_id());

		let response = self.client.post(&self.endpoint).json(&request).send().await?;

		let rpc_response: JsonRpcResponse<R> =
			response.json().await.map_err(|_| ProverError::JsonDeserialization)?;

		if let Some(error) = rpc_response.error {
			return Err(ProverError::RpcError { code: error.code, message: error.message });
		}

		rpc_response.result.ok_or(ProverError::MissingRpcResult)
	}

	/// Fetch the latest block number.
	pub async fn get_storage_at(
		&self,
		address: H160,
		slot: U256,
		block_number: u64,
	) -> Result<U256, ProverError> {
		let address_hex = format!("0x{:x}", address);
		let slot_hex = format!("0x{:064x}", slot);
		let block_hex = format!("0x{:x}", block_number);
		let value: String =
			self.call("eth_getStorageAt", (address_hex, slot_hex, block_hex)).await?;
		let bytes = hex_to_bytes(&value)?;
		let mut padded = [0u8; 32];
		if bytes.len() <= 32 {
			padded[32 - bytes.len()..].copy_from_slice(&bytes);
		}
		Ok(U256::from_big_endian(&padded))
	}

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

	/// Fetch block proof using `debug_getBlockProof`.
	pub async fn get_block_proof(&self, block_number: u64) -> Result<RpcBlockProof, ProverError> {
		let block_hex = format!("0x{:x}", block_number);
		self.call("debug_getBlockProof", vec![block_hex]).await
	}

	/// Fetch account and storage proofs using `eth_getProof`.
	///
	/// This uses the raw JSON-RPC client because Pharos returns proof nodes
	/// in its own hexary hash tree format rather than standard Ethereum MPT nodes.
	pub async fn get_proof(
		&self,
		address: H160,
		storage_keys: Vec<H256>,
		block_number: u64,
	) -> Result<RpcAccountProof, ProverError> {
		let address_hex = format!("0x{:x}", address);
		let keys_hex: Vec<String> = storage_keys.iter().map(|k| format!("0x{:x}", k)).collect();
		let block_hex = format!("0x{:x}", block_number);

		self.call("eth_getProof", (address_hex, keys_hex, block_hex)).await
	}

	/// Fetch validator info using `debug_getValidatorInfo`.
	pub async fn get_validator_info(
		&self,
		block_number: Option<u64>,
	) -> Result<RpcValidatorInfoResponse, ProverError> {
		let block_param = match block_number {
			Some(n) => format!("0x{:x}", n),
			None => "latest".to_string(),
		};
		self.call("debug_getValidatorInfo", vec![block_param]).await
	}
}

/// Parse a hex string to bytes.
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, ProverError> {
	let hex = hex.trim_start_matches("0x");
	hex::decode(hex).map_err(|_| ProverError::HexDecode)
}

/// Parse a hex string to H256.
pub fn hex_to_h256(hex: &str) -> Result<H256, ProverError> {
	let bytes = hex_to_bytes(hex)?;
	if bytes.len() != 32 {
		return Err(ProverError::InvalidH256Length(bytes.len()));
	}
	Ok(H256::from_slice(&bytes))
}

/// Parse a hex string to u64.
pub fn hex_to_u64(hex: &str) -> Result<u64, ProverError> {
	let hex = hex.trim_start_matches("0x");
	if hex.is_empty() {
		return Ok(0);
	}
	u64::from_str_radix(hex, 16).map_err(|_| ProverError::InvalidNumber)
}
