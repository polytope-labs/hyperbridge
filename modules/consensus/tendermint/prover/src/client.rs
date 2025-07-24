use std::{fmt::Debug, sync::Arc};

use crate::SignedHeader;
use async_trait::async_trait;
use cometbft::{block::Height, validator::Info as Validator};
use cometbft_rpc::{
	endpoint::abci_query::AbciQuery, Client as OtherClient, HttpClient, Paging, Url,
};
use geth_primitives::CodecHeader;
use reqwest::Client as ReqwestClient;
use serde::Deserialize;
use serde_json::{json, Value};
use tendermint_primitives::{
	prover::{
		CommitResponse, HeimdallValidatorsResponse, Milestone, RpcResponse, StatusResponse,
		ValidatorsResponse,
	},
	ProverError,
};

use ethers::{
	prelude::Provider,
	providers::{Http, Middleware},
	types::BlockId,
};

/// A trait defining the interface for interacting with Tendermint-compatible blockchain nodes.
///
/// This trait provides methods to query blockchain data such as block headers, validators,
/// and network status. It is implemented by different client types that can communicate
/// with various Tendermint-based networks (e.g., CometBFT, Heimdall).
#[async_trait]
pub trait Client {
	/// Retrieves the latest block height from the blockchain.
	///
	/// # Returns
	///
	/// - `Ok(u64)`: The latest block height
	/// - `Err(ProverError)`: If the request fails due to network issues or other errors
	async fn latest_height(&self) -> Result<u64, ProverError>;

	/// Retrieves a signed header for a specific block height.
	///
	/// # Arguments
	///
	/// * `height` - The block height to query
	///
	/// # Returns
	///
	/// - `Ok(SignedHeader)`: The signed header for the specified height
	/// - `Err(ProverError)`: If the height is invalid or the request fails
	async fn signed_header(&self, height: u64) -> Result<SignedHeader, ProverError>;

	/// Retrieves the validator set for a specific block height.
	///
	/// # Arguments
	///
	/// * `height` - The block height to query
	///
	/// # Returns
	///
	/// - `Ok(Vec<Validator>)`: The list of validators at the specified height
	/// - `Err(ProverError)`: If the height is invalid or the request fails
	async fn validators(&self, height: u64) -> Result<Vec<Validator>, ProverError>;

	/// Retrieves the validator set for the next block height.
	///
	/// This is typically used to get the validator set that will be active
	/// for the next block after the specified height.
	///
	/// # Arguments
	///
	/// * `height` - The current block height
	///
	/// # Returns
	///
	/// - `Ok(Vec<Validator>)`: The list of validators for the next height
	/// - `Err(ProverError)`: If the height is invalid or the request fails
	async fn next_validators(&self, height: u64) -> Result<Vec<Validator>, ProverError>;

	/// Retrieves a range of signed headers between two block heights.
	///
	/// # Arguments
	///
	/// * `start_height` - The starting block height (inclusive)
	/// * `end_height` - The ending block height (inclusive)
	///
	/// # Returns
	///
	/// - `Ok(Vec<SignedHeader>)`: The list of signed headers in the specified range
	/// - `Err(ProverError)`: If the height range is invalid or any request fails
	async fn signed_headers_range(
		&self,
		start_height: u64,
		end_height: u64,
	) -> Result<Vec<SignedHeader>, ProverError>;

	/// Retrieves the chain ID of the blockchain.
	///
	/// # Returns
	///
	/// - `Ok(String)`: The chain ID
	/// - `Err(ProverError)`: If the request fails
	async fn chain_id(&self) -> Result<String, ProverError>;

	/// Checks if the blockchain node is healthy and responding.
	///
	/// # Returns
	///
	/// - `Ok(true)`: If the node is healthy and responding
	/// - `Ok(false)`: If the node is not responding or unhealthy
	/// - `Err(ProverError)`: If the health check request fails
	async fn is_healthy(&self) -> Result<bool, ProverError>;
}

/// A client implementation for interacting with CometBFT nodes.
///
/// This client uses the official CometBFT RPC client to communicate with
/// Tendermint-compatible blockchain nodes that support the standard RPC interface.
pub struct CometBFTClient {
	client: HttpClient,
}

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

		Ok(Self { client })
	}
}

#[async_trait]
impl Client for CometBFTClient {
	async fn latest_height(&self) -> Result<u64, ProverError> {
		let status = self
			.client
			.status()
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;
		Ok(status.sync_info.latest_block_height.value())
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

	async fn signed_headers_range(
		&self,
		start_height: u64,
		end_height: u64,
	) -> Result<Vec<SignedHeader>, ProverError> {
		if start_height >= end_height {
			return Err(ProverError::InvalidHeight(
				"Start height must be less than end height".to_string(),
			));
		}

		let mut headers = Vec::new();
		for height in start_height..=end_height {
			let header = self.signed_header(height).await?;
			headers.push(header);
		}

		Ok(headers)
	}

	async fn chain_id(&self) -> Result<String, ProverError> {
		let status = self
			.client
			.status()
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;
		Ok(status.node_info.network.to_string())
	}

	async fn is_healthy(&self) -> Result<bool, ProverError> {
		match self.client.health().await {
			Ok(_) => Ok(true),
			Err(_) => Ok(false),
		}
	}
}

#[derive(Debug, Clone)]
/// A client implementation for interacting with Heimdall nodes.
///
/// This client uses HTTP requests to communicate with Heimdall nodes,
/// which are part of the Polygon network's validator layer.
/// Heimdall nodes provide a JSON-RPC interface for querying blockchain data.
pub struct HeimdallClient {
	raw_client: ReqwestClient,
	consensus_rpc_url: String,
	http_client: HttpClient,
	rest_url: String,
	execution_rpc_client: Arc<Provider<Http>>,
}

impl HeimdallClient {
	/// Creates a new Heimdall client instance.
	///
	/// # Arguments
	///
	/// * `url` - The RPC endpoint URL of the Heimdall node
	///
	/// # Returns
	///
	/// A new Heimdall client instance
	pub fn new(url: &str, rest_url: &str, execution_rpc: &str) -> Result<Self, ProverError> {
		let raw_client = ReqwestClient::new();
		let consensus_rpc_url = url.to_string();
		let client_url = url
			.parse::<Url>()
			.map_err(|e| ProverError::ConversionError(format!("Invalid URL: {}", e)))?;

		let http_client =
			HttpClient::new(client_url).map_err(|e| ProverError::NetworkError(e.to_string()))?;

		let provider = Provider::<Http>::try_from(execution_rpc).map_err(|e| {
			ProverError::NetworkError(format!("Failed to create ethers provider: {}", e))
		})?;
		let execution_rpc_client = Arc::new(provider);

		Ok(Self {
			raw_client,
			consensus_rpc_url,
			http_client,
			rest_url: rest_url.to_string(),
			execution_rpc_client,
		})
	}

	/// Performs a JSON-RPC request to the Heimdall node.
	///
	/// # Arguments
	///
	/// * `method` - The RPC method to call
	/// * `params` - The parameters for the RPC call
	///
	/// # Returns
	///
	/// - `Ok(T)`: The deserialized response
	/// - `Err(ProverError)`: If the request fails or the response cannot be parsed
	async fn rpc_request<T>(&self, method: &str, params: Value) -> Result<T, ProverError>
	where
		T: for<'de> Deserialize<'de>,
	{
		let request_body = json!({
			"jsonrpc": "2.0",
			"id": "1",
			"method": method,
			"params": params
		});

		let response = self
			.raw_client
			.post(&self.consensus_rpc_url)
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

		// Check for JSON-RPC error
		if let Some(error) = rpc_response.error {
			return Err(ProverError::RpcError(format!("RPC error: {}", error.message)));
		}

		rpc_response
			.result
			.ok_or_else(|| ProverError::RpcError("No result in response".to_string()))
	}

	/// Retrieves the number of milestones in the blockchain.
	///
	/// # Returns
	///
	/// - `Ok(u64)`: The number of milestones
	/// - `Err(ProverError)`: If the request fails or the response cannot be parsed
	pub async fn get_milestone_count(&self) -> Result<u64, ProverError> {
		let response = self
			.raw_client
			.get(format!("{}/milestones/count", self.rest_url))
			.send()
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;
		let count_resp: serde_json::Value =
			response.json().await.map_err(|e| ProverError::NetworkError(e.to_string()))?;

		let latest_count = count_resp["count"].as_str().unwrap().parse::<u64>().unwrap();
		Ok(latest_count)
	}

	/// Retrieves a milestone by its count.
	///
	/// # Arguments
	///
	/// * `count` - The count of the milestone to retrieve
	///
	/// # Returns
	pub async fn get_milestone(&self, count: u64) -> Result<Milestone, ProverError> {
		let response = self
			.raw_client
			.get(format!("{}/milestones/{}", self.rest_url, count))
			.send()
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;
		let milestone_resp: serde_json::Value =
			response.json().await.map_err(|e| ProverError::NetworkError(e.to_string()))?;

		// The actual milestone is likely under the "milestone" key, as seen in the integration test
		let milestone_value = milestone_resp.get("milestone").ok_or_else(|| {
			ProverError::NetworkError("Missing 'milestone' field in response".to_string())
		})?;
		let milestone: Milestone =
			serde_json::from_value(milestone_value.clone()).map_err(|e| {
				ProverError::NetworkError(format!("Failed to deserialize Milestone: {}", e))
			})?;
		Ok(milestone)
	}

	/// Retrieves the latest milestone.
	///
	/// # Returns
	///
	/// - `Ok(Milestone)`: The latest milestone
	/// - `Err(ProverError)`: If the request fails or the response cannot be parsed
	pub async fn get_latest_milestone(&self) -> Result<(u64, Milestone), ProverError> {
		let latest_count = self.get_milestone_count().await?;
		let milestone = self.get_milestone(latest_count).await?;
		Ok((latest_count, milestone))
	}

	/// Retrieves the ICS23 proof for the latest milestone.
	///
	/// # Returns
	///
	/// - `Ok(Vec<u8>)`: The ICS23 proof
	/// - `Err(ProverError)`: If the request fails or the response cannot be parsed
	pub async fn get_ics23_proof(
		&self,
		count: u64,
		latest_consensus_height: u64,
	) -> Result<AbciQuery, ProverError> {
		let mut key = vec![0x81];
		key.extend_from_slice(&count.to_be_bytes());

		let abci_query: AbciQuery = self
			.http_client
			.abci_query(
				Some("/store/milestone/key".to_string()),
				key,
				Some(Height::try_from(latest_consensus_height).unwrap()),
				true,
			)
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;

		Ok(abci_query)
	}

	/// Fetches an Ethereum block header from the execution RPC client and converts it to a
	/// CodecHeader.
	///
	/// # Arguments
	///
	/// * `block` - The block identifier (number or hash) to fetch. Must implement Into<BlockId>.
	///
	/// # Returns
	///
	/// - `Ok(Some(CodecHeader))` if the block exists and conversion succeeds
	/// - `Ok(None)` if the block does not exist
	/// - `Err(ProverError)` if the RPC call fails
	pub async fn fetch_header<T: Into<BlockId> + Send + Sync + Debug + Copy>(
		&self,
		block: T,
	) -> Result<Option<CodecHeader>, ProverError> {
		let block = self
			.execution_rpc_client
			.get_block(block)
			.await
			.map_err(|e| ProverError::NetworkError(format!("Failed to get block: {}", e)))?;
		if let Some(block) = block {
			let header = block.into();
			Ok(Some(header))
		} else {
			Ok(None)
		}
	}

	/// Fetches the latest Ethereum block header from the execution RPC client and converts it to a
	/// CodecHeader.
	///
	/// # Returns
	///
	/// - `Ok(CodecHeader)` if the latest block exists and conversion succeeds
	/// - `Err(ProverError)` if the RPC call fails or the block cannot be fetched
	pub async fn latest_header(&self) -> Result<CodecHeader, ProverError> {
		let block_number =
			self.execution_rpc_client.get_block_number().await.map_err(|e| {
				ProverError::NetworkError(format!("Failed to get block number: {}", e))
			})?;
		let header = self.fetch_header(block_number.as_u64()).await?.ok_or_else(|| {
			ProverError::NetworkError(format!(
				"Latest header block could not be fetched {block_number}"
			))
		})?;
		Ok(header)
	}
}

#[async_trait]
impl Client for HeimdallClient {
	async fn latest_height(&self) -> Result<u64, ProverError> {
		let status: StatusResponse = self.rpc_request("status", json!({})).await?;
		Ok(status.sync_info.latest_block_height.value())
	}

	async fn signed_header(&self, height: u64) -> Result<SignedHeader, ProverError> {
		let commit_response: CommitResponse =
			self.rpc_request("commit", json!({"height": height.to_string()})).await?;

		Ok(commit_response.signed_header)
	}

	async fn validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		// Use Heimdall-specific response type
		let heimdall_response: HeimdallValidatorsResponse =
			self.rpc_request("validators", json!({"height": height.to_string()})).await?;

		let validators_response: ValidatorsResponse = heimdall_response.into();
		Ok(validators_response.validators)
	}

	async fn next_validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		self.validators(height + 1).await
	}

	async fn signed_headers_range(
		&self,
		start_height: u64,
		end_height: u64,
	) -> Result<Vec<SignedHeader>, ProverError> {
		if start_height >= end_height {
			return Err(ProverError::InvalidHeight(
				"Start height must be less than end height".to_string(),
			));
		}

		let mut headers = Vec::new();
		for height in start_height..=end_height {
			let header = self.signed_header(height).await?;
			headers.push(header);
		}

		Ok(headers)
	}

	async fn chain_id(&self) -> Result<String, ProverError> {
		let status: StatusResponse = self.rpc_request("status", json!({})).await?;
		Ok(status.node_info.network)
	}

	async fn is_healthy(&self) -> Result<bool, ProverError> {
		match self.raw_client.get(&format!("{}/health", self.consensus_rpc_url)).send().await {
			Ok(response) => Ok(response.status().is_success()),
			Err(_) => Ok(false),
		}
	}
}
