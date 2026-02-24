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

//! Low-level HTTP client for the TRON `/wallet/*` JSON API.
//!
//! This module wraps the TRON full-node HTTP endpoints used for:
//! - Triggering smart contract calls (read & write)
//! - Broadcasting signed transactions
//! - Querying blocks, transactions, and events

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sp_core::Pair;
use std::time::Duration;

/// Configuration for a [`TronApi`] client.
#[derive(Debug, Clone)]
pub struct TronApiConfig {
	/// Base URL of the TRON full node.
	/// Example: `https://api.trongrid.io`
	pub full_host: String,
	/// Optional API key for the TRON full node.
	/// If provided, it will be sent as the `TRON-PRO-API-KEY` header.
	pub api_key: Option<String>,
	/// HTTP request timeout.
	pub timeout: Duration,
}

/// A thin HTTP wrapper around the TRON full-node REST API.
#[derive(Clone)]
pub struct TronApi {
	client: reqwest::Client,
	config: TronApiConfig,
}

impl TronApi {
	/// Create a new [`TronApi`] instance.
	pub fn new(config: TronApiConfig) -> anyhow::Result<Self> {
		let mut headers = reqwest::header::HeaderMap::new();
		headers.insert(
			reqwest::header::CONTENT_TYPE,
			reqwest::header::HeaderValue::from_static("application/json"),
		);

		// Add TRON-PRO-API-KEY header if an API key is provided
		if let Some(api_key) = &config.api_key {
			headers.insert(
				"TRON-PRO-API-KEY",
				reqwest::header::HeaderValue::from_str(api_key)
					.context("invalid API key format")?,
			);
		}

		let client = reqwest::Client::builder()
			.default_headers(headers)
			.timeout(config.timeout)
			.build()
			.context("failed to build reqwest client")?;

		Ok(Self { client, config })
	}

	/// The base URL this client is configured against.
	pub fn full_host(&self) -> &str {
		&self.config.full_host
	}

	async fn post<Req: Serialize, Res: serde::de::DeserializeOwned>(
		&self,
		path: &str,
		body: &Req,
	) -> anyhow::Result<Res> {
		let url = format!("{}{}", self.config.full_host, path);
		let resp = self
			.client
			.post(&url)
			.json(body)
			.send()
			.await
			.with_context(|| format!("POST {url} failed"))?;

		let status = resp.status();
		if !status.is_success() {
			let text = resp.text().await.unwrap_or_default();
			return Err(anyhow!("POST {url} returned HTTP {status}: {text}"));
		}

		let text = resp.text().await.context("failed to read response body")?;
		serde_json::from_str(&text)
			.with_context(|| format!("POST {url}: failed to deserialize response: {text}"))
	}

	/// Execute a **read-only** smart contract call.
	///
	/// Maps to `POST /wallet/triggerconstantcontract`.
	/// This does not cost energy and does not require signing.
	pub async fn trigger_constant_contract(
		&self,
		req: &TriggerContractRequest,
	) -> anyhow::Result<TriggerConstantContractResponse> {
		self.post("/wallet/triggerconstantcontract", req).await
	}

	/// Build an **unsigned** smart-contract trigger transaction.
	///
	/// Maps to `POST /wallet/triggersmartcontract`.
	/// The returned [`TriggerSmartContractResponse`] contains the unsigned
	/// transaction that must be signed and broadcast separately.
	pub async fn trigger_smart_contract(
		&self,
		req: &TriggerContractRequest,
	) -> anyhow::Result<TriggerSmartContractResponse> {
		self.post("/wallet/triggersmartcontract", req).await
	}

	/// Broadcast a **signed** transaction to the network.
	///
	/// Maps to `POST /wallet/broadcasttransaction`.
	pub async fn broadcast_transaction(
		&self,
		tx: &SignedTransaction,
	) -> anyhow::Result<BroadcastResult> {
		self.post("/wallet/broadcasttransaction", tx).await
	}

	/// Fetch the on-chain receipt / execution info for a transaction.
	///
	/// Maps to `POST /wallet/gettransactioninfobyid`.
	/// Returns `Ok(None)` if the transaction has not been included in a block
	/// yet (receipt not available).
	pub async fn get_transaction_info(
		&self,
		tx_id: &str,
	) -> anyhow::Result<Option<TransactionInfo>> {
		let body = serde_json::json!({ "value": tx_id });
		let raw: serde_json::Value = self.post("/wallet/gettransactioninfobyid", &body).await?;
		// An empty object `{}` means "not found / not yet mined".
		if raw.as_object().map_or(true, |m| m.is_empty()) {
			return Ok(None);
		}
		serde_json::from_value(raw.clone())
			.with_context(|| format!("failed to parse TransactionInfo: {raw}"))
			.map(Some)
	}

	/// Fetch a full transaction by its ID.
	///
	/// Maps to `POST /wallet/gettransactionbyid`.
	pub async fn get_transaction_by_id(
		&self,
		tx_id: &str,
	) -> anyhow::Result<Option<serde_json::Value>> {
		let body = serde_json::json!({ "value": tx_id });
		let raw: serde_json::Value = self.post("/wallet/gettransactionbyid", &body).await?;
		if raw.as_object().map_or(true, |m| m.is_empty()) {
			return Ok(None);
		}
		Ok(Some(raw))
	}

	/// Fetch the latest ("now") block.
	///
	/// Maps to `POST /wallet/getnowblock`.
	pub async fn get_now_block(&self) -> anyhow::Result<BlockResponse> {
		self.post("/wallet/getnowblock", &serde_json::json!({})).await
	}

	/// Fetch a block by its number.
	///
	/// Maps to `POST /wallet/getblockbynum`.
	pub async fn get_block_by_num(&self, num: u64) -> anyhow::Result<BlockResponse> {
		self.post("/wallet/getblockbynum", &serde_json::json!({ "num": num })).await
	}

	/// Fetch account information (balance, resources).
	///
	/// Maps to `POST /wallet/getaccount`.
	pub async fn get_account(&self, address: &str) -> anyhow::Result<AccountResponse> {
		self.post("/wallet/getaccount", &serde_json::json!({ "address": address }))
			.await
	}

	/// Fetch account resource information (energy, bandwidth).
	///
	/// Maps to `POST /wallet/getaccountresource`.
	pub async fn get_account_resource(&self, address: &str) -> anyhow::Result<serde_json::Value> {
		self.post("/wallet/getaccountresource", &serde_json::json!({ "address": address }))
			.await
	}

	/// Fetch the chain parameters.
	///
	/// Maps to `POST /wallet/getchainparameters`.
	pub async fn get_chain_parameters(&self) -> anyhow::Result<ChainParametersResponse> {
		self.post("/wallet/getchainparameters", &serde_json::json!({})).await
	}

	/// Query contract events using the TronGrid event API.
	///
	/// Maps to `GET /v1/contracts/{address}/events`.
	///
	/// **Note:** This endpoint is only available on TronGrid (and TRE), not
	/// on plain full-nodes.  For plain nodes, use `get_transaction_info` to
	/// read logs from individual transactions.
	pub async fn get_contract_events(
		&self,
		contract_address: &str,
		event_name: Option<&str>,
		min_block_timestamp: Option<u64>,
		max_block_timestamp: Option<u64>,
		limit: Option<u32>,
	) -> anyhow::Result<EventQueryResponse> {
		let mut url = format!("{}/v1/contracts/{}/events", self.config.full_host, contract_address);

		let mut params: Vec<String> = Vec::new();
		if let Some(name) = event_name {
			params.push(format!("event_name={name}"));
		}
		if let Some(ts) = min_block_timestamp {
			params.push(format!("min_block_timestamp={ts}"));
		}
		if let Some(ts) = max_block_timestamp {
			params.push(format!("max_block_timestamp={ts}"));
		}
		if let Some(l) = limit {
			params.push(format!("limit={l}"));
		}
		if !params.is_empty() {
			url.push('?');
			url.push_str(&params.join("&"));
		}

		let resp = self
			.client
			.get(&url)
			.send()
			.await
			.with_context(|| format!("GET {url} failed"))?;

		let status = resp.status();
		if !status.is_success() {
			let text = resp.text().await.unwrap_or_default();
			return Err(anyhow!("GET {url} returned HTTP {status}: {text}"));
		}

		resp.json().await.context("failed to parse event query response")
	}
}

/// Request body shared by `triggerConstantContract` and `triggerSmartContract`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerContractRequest {
	/// Caller address, hex-encoded with `41` prefix (e.g. `"41..."`)
	pub owner_address: String,
	/// Contract address, hex-encoded with `41` prefix
	pub contract_address: String,
	/// Solidity function selector (e.g. `"handleConsensus(address,bytes)"`)
	pub function_selector: String,
	/// ABI-encoded parameters as a hex string (**without** `0x` prefix)
	pub parameter: String,
	/// Maximum TRX fee in SUN (1 TRX = 1_000_000 SUN).
	/// Only meaningful for `triggerSmartContract`.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fee_limit: Option<u64>,
	/// TRX amount to send with the call (in SUN).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub call_value: Option<u64>,
	/// Whether to make the call visible
	#[serde(skip_serializing_if = "Option::is_none")]
	pub visible: Option<bool>,
}

/// Response from `triggerConstantContract`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConstantContractResponse {
	/// ABI-encoded return data, hex strings.
	#[serde(default)]
	pub constant_result: Vec<String>,
	/// The result object.
	#[serde(default)]
	pub result: TriggerResult,
	/// Energy used for the simulated execution.
	#[serde(default)]
	pub energy_used: u64,
	/// Energy penalty, if any.
	#[serde(default)]
	pub energy_penalty: u64,
}

/// Response from `triggerSmartContract`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSmartContractResponse {
	/// The result metadata.
	#[serde(default)]
	pub result: TriggerResult,
	/// The unsigned transaction object.
	/// This is `None` if the trigger failed validation.
	pub transaction: Option<UnsignedTransaction>,
}

/// Metadata about a trigger result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggerResult {
	/// `true` if the call/trigger succeeded.
	#[serde(default)]
	pub result: bool,
	/// Error code (only present on failure).
	#[serde(default)]
	pub code: Option<String>,
	/// Human-readable error message (only present on failure).
	#[serde(default)]
	pub message: Option<String>,
}

impl TriggerResult {
	/// Returns an `Err` if this result indicates a failure.
	pub fn into_result(self) -> anyhow::Result<()> {
		if self.result {
			Ok(())
		} else {
			let code = self.code.unwrap_or_default();
			let msg = self.message.as_deref().unwrap_or("");
			// Message is often hex-encoded, try to decode it
			let decoded_msg = hex::decode(msg)
				.ok()
				.and_then(|b| String::from_utf8(b).ok())
				.unwrap_or_else(|| msg.to_string());
			Err(anyhow!("TRON trigger failed: code={code}, message={decoded_msg}"))
		}
	}
}

/// An unsigned transaction as returned by `triggerSmartContract`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsignedTransaction {
	/// Whether addresses are in visible (base58) format.
	#[serde(default)]
	pub visible: bool,
	/// The transaction ID (hex SHA-256 of `raw_data`).
	#[serde(rename = "txID")]
	pub tx_id: String,
	/// The raw transaction data.
	pub raw_data: serde_json::Value,
	/// Hex-encoded raw_data bytes.
	pub raw_data_hex: String,
}

impl UnsignedTransaction {
	/// Compute the transaction ID from `raw_data_hex`.
	///
	/// TRON defines `txID = SHA256(raw_data_bytes)`.
	pub fn compute_tx_id(&self) -> anyhow::Result<[u8; 32]> {
		let raw_bytes = hex::decode(&self.raw_data_hex).context("failed to decode raw_data_hex")?;
		let hash = Sha256::digest(&raw_bytes);
		Ok(hash.into())
	}
}

/// A signed transaction ready for broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
	/// Whether addresses are in visible (base58) format.
	#[serde(default)]
	pub visible: bool,
	/// The transaction ID.
	#[serde(rename = "txID")]
	pub tx_id: String,
	/// Raw transaction data (opaque JSON).
	pub raw_data: serde_json::Value,
	/// Hex-encoded raw_data bytes.
	pub raw_data_hex: String,
	/// Array of hex-encoded signatures.
	pub signature: Vec<String>,
}

impl SignedTransaction {
	/// Sign an [`UnsignedTransaction`] using the provided secp256k1 secret key.
	///
	/// TRON signing: `signature = secp256k1_sign(SHA256(raw_data))`.
	/// The key is the raw 32-byte secret key.
	pub fn sign(unsigned: UnsignedTransaction, secret_key: &[u8; 32]) -> anyhow::Result<Self> {
		let tx_id_bytes = unsigned.compute_tx_id()?;

		let signing_key =
			sp_core::ecdsa::Pair::from_seed_slice(secret_key).context("invalid secret key")?;

		// TRON expects a raw secp256k1 signature (r || s || v), 65 bytes.
		// sp_core::ecdsa::Pair::sign produces a 65-byte compact signature.
		let sig = signing_key.sign_prehashed(&tx_id_bytes);
		let sig_hex = hex::encode(sig.0);

		Ok(Self {
			visible: unsigned.visible,
			tx_id: unsigned.tx_id,
			raw_data: unsigned.raw_data,
			raw_data_hex: unsigned.raw_data_hex,
			signature: vec![sig_hex],
		})
	}
}

/// Result of broadcasting a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastResult {
	/// `true` if the node accepted the transaction.
	#[serde(default)]
	pub result: bool,
	/// Error code (e.g. `"CONTRACT_VALIDATE_ERROR"`).
	#[serde(default)]
	pub code: Option<String>,
	/// Transaction ID.
	#[serde(rename = "txid", default)]
	pub tx_id: Option<String>,
	/// Error message (often hex-encoded).
	#[serde(default)]
	pub message: Option<String>,
}

impl BroadcastResult {
	/// Returns an `Err` if the broadcast was rejected.
	pub fn into_result(self) -> anyhow::Result<String> {
		if self.result {
			Ok(self.tx_id.unwrap_or_default())
		} else {
			let code = self.code.unwrap_or_default();
			let msg = self.message.as_deref().unwrap_or("");
			let decoded_msg = hex::decode(msg)
				.ok()
				.and_then(|b| String::from_utf8(b).ok())
				.unwrap_or_else(|| msg.to_string());
			Err(anyhow!("TRON broadcast rejected: code={code}, message={decoded_msg}"))
		}
	}
}

/// On-chain transaction execution info (receipt).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
	/// The transaction ID.
	#[serde(default)]
	pub id: String,
	/// Block number.
	#[serde(rename = "blockNumber", default)]
	pub block_number: u64,
	/// Block timestamp (ms since epoch).
	#[serde(rename = "blockTimeStamp", default)]
	pub block_timestamp: u64,
	/// Fee charged (in SUN).
	#[serde(default)]
	pub fee: u64,
	/// Contract execution result: `"SUCCESS"` or `"REVERT"`, etc.
	#[serde(default)]
	pub result: Option<String>,
	/// Revert reason (hex-encoded ABI error).
	#[serde(rename = "resMessage", default)]
	pub res_message: Option<String>,
	/// Contract execution return value.
	#[serde(rename = "contractResult", default)]
	pub contract_result: Vec<String>,
	/// Address of contract created (if deployment).
	#[serde(rename = "contract_address", default)]
	pub contract_address: Option<String>,
	/// Receipt sub-object.
	#[serde(default)]
	pub receipt: Option<TransactionReceipt>,
	/// Event logs.
	#[serde(default)]
	pub log: Vec<TransactionLog>,
}

impl TransactionInfo {
	/// `true` if the contract execution succeeded.
	pub fn succeeded(&self) -> bool {
		// `result` is absent on success in many TRON node versions,
		// but `receipt.result` should be "SUCCESS".
		if let Some(ref receipt) = self.receipt {
			receipt.result.as_deref() == Some("SUCCESS")
		} else {
			// If there's no receipt, check the top-level result field
			self.result.as_deref() != Some("FAILED")
		}
	}
}

/// The `receipt` sub-object inside [`TransactionInfo`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
	/// Energy used.
	#[serde(rename = "energy_usage", default)]
	pub energy_usage: u64,
	/// Total energy used (including fee).
	#[serde(rename = "energy_usage_total", default)]
	pub energy_usage_total: u64,
	/// Energy fee (in SUN).
	#[serde(rename = "energy_fee", default)]
	pub energy_fee: u64,
	/// Net (bandwidth) usage.
	#[serde(rename = "net_usage", default)]
	pub net_usage: u64,
	/// Execution result: `"SUCCESS"`, `"REVERT"`, `"OUT_OF_ENERGY"`, etc.
	#[serde(default)]
	pub result: Option<String>,
}

/// An event log entry inside [`TransactionInfo`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLog {
	/// Contract address that emitted the event (hex, 41-prefixed).
	#[serde(default)]
	pub address: String,
	/// Indexed event topics (hex-encoded, 32 bytes each).
	#[serde(default)]
	pub topics: Vec<String>,
	/// Non-indexed ABI-encoded event data (hex).
	#[serde(default)]
	pub data: String,
}

/// Response from `getnowblock` / `getblockbynum`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockResponse {
	/// The block ID (hash).
	#[serde(rename = "blockID", default)]
	pub block_id: String,
	/// Block header.
	#[serde(default)]
	pub block_header: Option<BlockHeader>,
}

impl BlockResponse {
	/// Extract the block number from the header.
	pub fn number(&self) -> u64 {
		self.block_header
			.as_ref()
			.and_then(|h| h.raw_data.as_ref())
			.map(|r| r.number)
			.unwrap_or(0)
	}

	/// Extract the block timestamp (ms) from the header.
	pub fn timestamp_ms(&self) -> u64 {
		self.block_header
			.as_ref()
			.and_then(|h| h.raw_data.as_ref())
			.map(|r| r.timestamp)
			.unwrap_or(0)
	}
}

/// Block header wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
	/// Raw header data.
	pub raw_data: Option<BlockHeaderRawData>,
}

/// Raw data inside a block header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeaderRawData {
	/// Block number.
	#[serde(default)]
	pub number: u64,
	/// Block timestamp (ms since epoch).
	#[serde(default)]
	pub timestamp: u64,
	/// Parent block hash.
	#[serde(rename = "parentHash", default)]
	pub parent_hash: String,
}

/// Account information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountResponse {
	/// Account address (hex).
	#[serde(default)]
	pub address: String,
	/// Balance in SUN.
	#[serde(default)]
	pub balance: u64,
}

/// Chain parameters response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainParametersResponse {
	/// List of chain parameter key-value pairs.
	#[serde(rename = "chainParameter", default)]
	pub chain_parameter: Vec<ChainParameter>,
}

impl ChainParametersResponse {
	/// Look up a chain parameter by key.
	pub fn get(&self, key: &str) -> Option<u64> {
		self.chain_parameter.iter().find(|p| p.key == key).and_then(|p| p.value)
	}
}

/// A single chain parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainParameter {
	/// Parameter name.
	pub key: String,
	/// Parameter value (absent means the feature is disabled / zero).
	pub value: Option<u64>,
}

/// Response from the TronGrid event query API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventQueryResponse {
	/// Whether the query was successful.
	#[serde(default)]
	pub success: bool,
	/// Returned event data.
	#[serde(default)]
	pub data: Vec<EventData>,
	/// Metadata (pagination, etc.)
	#[serde(default)]
	pub meta: Option<serde_json::Value>,
}

/// A single event entry from the TronGrid event API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
	/// Block number.
	#[serde(default)]
	pub block_number: u64,
	/// Block timestamp (ms).
	#[serde(default)]
	pub block_timestamp: u64,
	/// Contract address.
	#[serde(default)]
	pub contract_address: String,
	/// Event name (e.g. `"StateMachineUpdated"`).
	#[serde(default)]
	pub event_name: String,
	/// Decoded event parameters.
	#[serde(default)]
	pub result: serde_json::Value,
	/// The transaction ID.
	#[serde(default)]
	pub transaction_id: String,
	/// Type of the result (indexed, etc.)
	#[serde(default)]
	pub result_type: Option<serde_json::Value>,
	/// Raw event data, if present.
	#[serde(default)]
	pub event: Option<String>,
}

/// Convert a 20-byte hex address (with or without `0x` prefix) to TRON's
/// 21-byte hex format with `41` prefix.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(to_tron_hex("0xaBcD...1234"), "41aBcD...1234");
/// assert_eq!(to_tron_hex("aBcD...1234"),   "41aBcD...1234");
/// assert_eq!(to_tron_hex("41aBcD...1234"), "41aBcD...1234"); // already prefixed
/// ```
pub fn to_tron_hex(addr: &str) -> String {
	let stripped = addr.strip_prefix("0x").unwrap_or(addr);
	if stripped.starts_with("41") && stripped.len() == 42 {
		stripped.to_string()
	} else {
		format!("41{stripped}")
	}
}

/// Convert TRON's 21-byte hex address (`41`-prefixed) to a 20-byte EVM
/// address with `0x` prefix.
pub fn to_evm_hex(tron_hex: &str) -> String {
	let stripped = tron_hex.strip_prefix("41").unwrap_or(tron_hex);
	format!("0x{stripped}")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_address_conversion() {
		let evm = "0xabcdef1234567890abcdef1234567890abcdef12";
		let tron = to_tron_hex(evm);
		assert_eq!(tron, "41abcdef1234567890abcdef1234567890abcdef12");
		assert_eq!(to_evm_hex(&tron), evm);
	}

	#[test]
	fn test_already_tron_hex() {
		let addr = "41abcdef1234567890abcdef1234567890abcdef12";
		assert_eq!(to_tron_hex(addr), addr);
	}

	#[test]
	fn test_trigger_result_success() {
		let r = TriggerResult { result: true, code: None, message: None };
		assert!(r.into_result().is_ok());
	}

	#[test]
	fn test_trigger_result_failure() {
		let r = TriggerResult {
			result: false,
			code: Some("CONTRACT_VALIDATE_ERROR".into()),
			message: Some(hex::encode("fee limit too low")),
		};
		let err = r.into_result().unwrap_err();
		assert!(err.to_string().contains("fee limit too low"));
	}

	#[test]
	fn test_broadcast_result_success() {
		let r = BroadcastResult {
			result: true,
			code: None,
			tx_id: Some("abc123".into()),
			message: None,
		};
		assert_eq!(r.into_result().unwrap(), "abc123");
	}

	#[test]
	fn test_broadcast_result_failure() {
		let r = BroadcastResult {
			result: false,
			code: Some("SIGERROR".into()),
			tx_id: None,
			message: Some(hex::encode("bad signature")),
		};
		let err = r.into_result().unwrap_err();
		assert!(err.to_string().contains("bad signature"));
	}
}
