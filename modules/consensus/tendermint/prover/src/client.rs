use crate::SignedHeader;
use async_trait::async_trait;
use cometbft::{block::Height, validator::Info as Validator};
use cometbft_rpc::{
	endpoint::abci_query::AbciQuery, Client as OtherClient, HttpClient, Paging, Url,
};
use futures::future::join_all;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tendermint_primitives::{Client, ProverError};

/// A client implementation for interacting with CometBFT nodes.
///
/// This client uses the official CometBFT RPC client to communicate with
/// Tendermint-compatible blockchain nodes that support the standard RPC interface.
pub struct CometBFTClient {
	client: HttpClient,
	raw_client: ReqwestClient,
	rpc_url: String,
}

// Key trait and chain implementations are defined in the `keys` module.

impl CometBFTClient {
	/// Creates a new CometBFT client instance.
	///
	/// # Arguments
	///
	/// * `url` - The RPC endpoint URL of the CometBFT node
	///
	/// # Returns
	///
	/// - `Ok(Self)`: A new CometBFT client instance
	/// - `Err(ProverError)`: If the URL is invalid or the client cannot be created
	pub async fn new(url: &str) -> Result<Self, ProverError> {
		let client_url = url
			.parse::<Url>()
			.map_err(|e| ProverError::ConversionError(format!("Invalid URL: {}", e)))?;

		let client =
			HttpClient::new(client_url).map_err(|e| ProverError::NetworkError(e.to_string()))?;

		let raw_client = ReqwestClient::new();

		Ok(Self { client, raw_client, rpc_url: url.to_string() })
	}

	/// Performs a JSON-RPC request and returns the deserialized `result`.
	async fn rpc_request<T>(&self, method: &str, params: Value) -> Result<T, ProverError>
	where
		T: for<'de> Deserialize<'de>,
	{
		let request_body = json!({
			"jsonrpc": "2.0",
			"id": "1",
			"method": method,
			"params": params,
		});

		let response = self
			.raw_client
			.post(&self.rpc_url)
			.json(&request_body)
			.send()
			.await
			.map_err(|e| ProverError::NetworkError(format!("Request failed: {}", e)))?;

		if !response.status().is_success() {
			return Err(ProverError::NetworkError(format!("HTTP error: {}", response.status())));
		}

		let rpc_response: RpcResponse<T> = response
			.json()
			.await
			.map_err(|e| ProverError::RpcError(format!("Failed to parse response: {}", e)))?;

		if let Some(err) = rpc_response.error {
			return Err(ProverError::RpcError(format!("RPC error: {}", err.message)));
		}

		rpc_response
			.result
			.ok_or_else(|| ProverError::RpcError("No result in response".to_string()))
	}

	/// Fallback path: fetch latest height via raw JSON-RPC status call and parse string height.
	async fn latest_status_via_raw(&self) -> Result<RawStatus, ProverError> {
		let status: RawStatus = self.rpc_request("status", json!({})).await?;
		Ok(status)
	}

	/// Perform a generic ABCI query for a single key under a module store, returning an ICS23
	/// proof.
	///
	/// - `store_key`: the Cosmos SDK module store key (e.g., "evm"). This is the same string used
	///   by the module when creating its KVStore (Sei EVM uses `StoreKey = "evm"`). The ABCI path
	///   becomes `/store/{store_key}/key`.
	/// - `key`: raw key bytes within that store (including any module-defined prefix). Example: For
	///   Sei EVM, common keys are:
	///   - Nonce: `0x0a || <20-byte evm_address>` (prefix `NonceKeyPrefix`)
	///   - Code hash: `0x08 || <20-byte evm_address>` (prefix `CodeHashKeyPrefix`)
	///   - Code: `0x07 || <20-byte evm_address>` (prefix `CodeKeyPrefix`)
	///   - Storage slot: `0x03 || <20-byte evm_address> || <32-byte storage_key>` (prefix
	///     `StateKeyPrefix`)
	///   where the 20-byte address is the Ethereum address bytes and the 32-byte storage key
	///   is the Keccak-256 slot key.
	/// - `height`: consensus height to query at (must match a height you have a verified header
	///   for).
	///
	/// Returns the ABCI response including `proof_ops` (ICS23). Verify this proof against the
	/// Tendermint app hash you obtained from the verified signed header at `height`.
	pub async fn abci_query_key(
		&self,
		store_key: &str,
		key: Vec<u8>,
		height: u64,
	) -> Result<AbciQuery, ProverError> {
		let height =
			Height::try_from(height).map_err(|e| ProverError::InvalidHeight(e.to_string()))?;
		self.client
			.abci_query(Some(format!("/store/{}/key", store_key)), key, Some(height), true)
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))
	}

	/// Perform multiple ABCI key queries concurrently under the same store key and height.
	/// Returns results in the same order as the input keys.
	pub async fn abci_query_keys(
		&self,
		store_key: &str,
		keys: Vec<Vec<u8>>,
		height: u64,
	) -> Result<Vec<AbciQuery>, ProverError> {
		let futs = keys.into_iter().map(|key| self.abci_query_key(store_key, key, height));
		let results = join_all(futs).await;
		let mut out = Vec::with_capacity(results.len());
		for res in results {
			out.push(res?);
		}
		Ok(out)
	}
}

#[async_trait]
impl Client for CometBFTClient {
	async fn latest_height(&self) -> Result<u64, ProverError> {
		match self.client.status().await {
			Ok(status) => Ok(status.sync_info.latest_block_height.value()),
			Err(_e) => {
				let status = self.latest_status_via_raw().await?;
				Ok(status.sync_info.latest_block_height)
			},
		}
	}

	async fn signed_header(&self, height: u64) -> Result<SignedHeader, ProverError> {
		let height =
			Height::try_from(height).map_err(|e| ProverError::InvalidHeight(e.to_string()))?;
		let commit_response = self
			.client
			.commit(height)
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;
		Ok(commit_response.signed_header)
	}

	async fn validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		let height =
			Height::try_from(height).map_err(|e| ProverError::InvalidHeight(e.to_string()))?;
		let validators_response = self
			.client
			.validators(height, Paging::All)
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;
		Ok(validators_response.validators)
	}

	async fn next_validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		self.validators(height + 1).await
	}

	async fn chain_id(&self) -> Result<String, ProverError> {
		match self.client.status().await {
			Ok(status) => Ok(status.node_info.network.to_string()),
			Err(_e) => {
				let status = self.latest_status_via_raw().await?;
				Ok(status.node_info.network)
			},
		}
	}

	async fn is_healthy(&self) -> Result<bool, ProverError> {
		match self.client.health().await {
			Ok(_) => Ok(true),
			Err(_) => Ok(false),
		}
	}
}

// Minimal types for decoding JSON-RPC commit response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcResponse<T> {
	#[allow(dead_code)]
	jsonrpc: String,
	#[allow(dead_code)]
	id: Value,
	result: Option<T>,
	error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcError {
	code: i32,
	message: String,
	#[allow(dead_code)]
	data: Option<Value>,
}

// Minimal status response with tolerant height parsing and network support
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawStatus {
	node_info: RawNodeInfo,
	sync_info: RawSyncInfo,
	#[allow(dead_code)]
	validator_info: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawNodeInfo {
	network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawSyncInfo {
	#[serde(deserialize_with = "deserialize_height")]
	latest_block_height: u64,
}

fn deserialize_height<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let v: Value = Value::deserialize(deserializer)?;
	if let Some(s) = v.as_str() {
		s.parse::<u64>().map_err(serde::de::Error::custom)
	} else if let Some(n) = v.as_u64() {
		Ok(n)
	} else {
		Err(serde::de::Error::custom("latest_block_height not string or number"))
	}
}
