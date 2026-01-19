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

use crate::ProverError;
use primitive_types::{H160, H256};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// JSON-RPC request structure.
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest<P> {
	pub jsonrpc: &'static str,
	pub method: &'static str,
	pub params: P,
	pub id: u64,
}

impl<P> JsonRpcRequest<P> {
	pub fn new(method: &'static str, params: P, id: u64) -> Self {
		Self { jsonrpc: "2.0", method, params, id }
	}
}

/// JSON-RPC response structure.
#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse<T> {
	pub jsonrpc: String,
	pub result: Option<T>,
	pub error: Option<JsonRpcError>,
	pub id: u64,
}

/// JSON-RPC error structure.
#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
	pub code: i64,
	pub message: String,
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

/// Block header response from `eth_getBlockByNumber`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcBlock {
	pub number: String,
	pub hash: String,
	pub parent_hash: String,
	pub nonce: Option<String>,
	pub sha3_uncles: String,
	pub logs_bloom: String,
	pub transactions_root: String,
	pub state_root: String,
	pub receipts_root: String,
	pub miner: String,
	pub difficulty: String,
	pub total_difficulty: Option<String>,
	pub extra_data: String,
	pub size: String,
	pub gas_limit: String,
	pub gas_used: String,
	pub timestamp: String,
	pub transactions: serde_json::Value,
	pub uncles: Vec<String>,
	pub base_fee_per_gas: Option<String>,
	pub withdrawals_root: Option<String>,
	pub blob_gas_used: Option<String>,
	pub excess_blob_gas: Option<String>,
	pub parent_beacon_block_root: Option<String>,
}

/// Account proof response from `eth_getProof`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcAccountProof {
	pub address: String,
	pub account_proof: Vec<String>,
	pub balance: String,
	pub code_hash: String,
	pub nonce: String,
	pub storage_hash: String,
	pub storage_proof: Vec<RpcStorageProof>,
}

/// Storage proof entry from `eth_getProof`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcStorageProof {
	pub key: String,
	pub value: String,
	pub proof: Vec<String>,
}

/// RPC client for Pharos node.
pub struct PharosRpcClient {
	endpoint: String,
	client: reqwest::Client,
	request_id: AtomicU64,
}

impl PharosRpcClient {
	/// Create a new RPC client.
	pub fn new(endpoint: impl Into<String>) -> Self {
		Self {
			endpoint: endpoint.into(),
			client: reqwest::Client::new(),
			request_id: AtomicU64::new(1),
		}
	}

	/// Get the next request ID.
	fn next_id(&self) -> u64 {
		self.request_id.fetch_add(1, Ordering::SeqCst)
	}

	/// Make a JSON-RPC call.
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

	/// Fetch block proof using `debug_getBlockProof`.
	pub async fn get_block_proof(&self, block_number: u64) -> Result<RpcBlockProof, ProverError> {
		let block_hex = format!("0x{:x}", block_number);
		self.call("debug_getBlockProof", vec![block_hex]).await
	}

	/// Fetch block by number using `eth_getBlockByNumber`.
	pub async fn get_block_by_number(&self, block_number: u64) -> Result<RpcBlock, ProverError> {
		let block_hex = format!("0x{:x}", block_number);
		self.call("eth_getBlockByNumber", (block_hex, false)).await
	}

	/// Fetch account and storage proofs using `eth_getProof`.
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

	/// Fetch the latest block number using `eth_blockNumber`.
	pub async fn get_block_number(&self) -> Result<u64, ProverError> {
		let result: String = self.call("eth_blockNumber", Vec::<()>::new()).await?;
		u64::from_str_radix(result.trim_start_matches("0x"), 16)
			.map_err(|_| ProverError::InvalidNumber)
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
	u64::from_str_radix(hex, 16).map_err(|_| ProverError::InvalidNumber)
}
