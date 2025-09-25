use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use cometbft::{
	account::Id as CometbftAccountId,
	block::{signed_header::SignedHeader, Height},
	public_key::PublicKey,
	validator::Info as Validator,
};
use cometbft_rpc::{endpoint::abci_query::AbciQuery, Client as OtherClient, HttpClient, Url};
use geth_primitives::CodecHeader;
use ismp_polygon::Milestone;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tendermint_primitives::{Client, ProverError};

use ethers::{
	prelude::Provider,
	providers::{Http, Middleware},
	types::BlockId,
};

use base64::Engine;

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
	execution_rpc_client: Arc<Provider<Http>>,
}

impl HeimdallClient {
	/// Creates a new Heimdall client instance.
	///
	/// # Arguments
	///
	/// * `url` - The consensus RPC endpoint URL of the Heimdall node
	/// * `execution_rpc` - The execution RPC endpoint URL for Ethereum/Polygon PoS chain
	///
	/// # Returns
	///
	/// A new Heimdall client instance
	///
	/// # Errors
	///
	/// Returns `ProverError` if:
	/// - The consensus RPC URL is invalid
	/// - The HTTP client cannot be created
	/// - The execution RPC provider cannot be created
	pub fn new(url: &str, execution_rpc: &str) -> Result<Self, ProverError> {
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

		Ok(Self { raw_client, consensus_rpc_url, http_client, execution_rpc_client })
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

	/// Retrieves the ICS23 proof for a specific milestone.
	///
	/// This method queries the Heimdall node's ABCI store to get the proof
	/// for a milestone at the specified count and height.
	///
	/// # Arguments
	///
	/// * `count` - The milestone count to get the proof for
	/// * `latest_consensus_height` - The latest consensus height for the proof
	///
	/// # Returns
	///
	/// - `Ok(AbciQuery)`: The ABCI query response containing the proof
	/// - `Err(ProverError)`: If the request fails or the response cannot be parsed
	///
	/// # Errors
	///
	/// Returns `ProverError` if:
	/// - The ABCI query fails
	/// - The height conversion fails
	pub async fn get_milestone_proof(
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

	/// Retrieves the milestone count at a specific height using ABCI query.
	///
	/// This method queries the Heimdall node's ABCI store to get the milestone count
	/// at the specified height, allowing milestone updates even when syncing.
	///
	/// # Arguments
	///
	/// * `height` - The height at which to query the milestone count
	///
	/// # Returns
	///
	/// - `Ok(u64)`: The milestone count at the specified height
	/// - `Err(ProverError)`: If the request fails or the response cannot be parsed
	///
	/// # Errors
	///
	/// Returns `ProverError` if:
	/// - The ABCI query fails
	/// - The height conversion fails
	/// - The response cannot be deserialized
	pub async fn get_milestone_count_at_height(
		&self,
		height: u64,
	) -> Result<Option<u64>, ProverError> {
		let key = vec![0x83];

		let abci_query: AbciQuery = self
			.http_client
			.abci_query(
				Some("/store/milestone/key".to_string()),
				key,
				Some(Height::try_from(height).unwrap()),
				true,
			)
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;

		let count_bytes = abci_query.value;
		if count_bytes.is_empty() {
			return Ok(None); // No milestones yet
		}

		// The count is stored as a u64 in big-endian format
		if count_bytes.len() != 8 {
			return Err(ProverError::ConversionError(
				"Invalid milestone count bytes length".to_string(),
			));
		}

		let mut bytes = [0u8; 8];
		bytes.copy_from_slice(&count_bytes[..=7]);
		let count = u64::from_be_bytes(bytes);

		Ok(Some(count))
	}

	/// Retrieves the latest milestone at a specific height using ABCI query.
	///
	/// This method queries the Heimdall node's ABCI store to get the latest milestone
	/// at the specified height, allowing milestone updates even when syncing.
	///
	/// # Arguments
	///
	/// * `height` - The height at which to query the latest milestone
	///
	/// # Returns
	///
	/// - `Ok(Option<(u64, Milestone)>)`: The milestone number and data if available
	/// - `Err(ProverError)`: If the request fails or the response cannot be parsed
	///
	/// # Errors
	///
	/// Returns `ProverError` if:
	/// - The ABCI query fails
	/// - The height conversion fails
	/// - The response cannot be deserialized
	pub async fn get_latest_milestone_at_height(
		&self,
		height: u64,
	) -> Result<Option<(u64, Milestone)>, ProverError> {
		let count = self.get_milestone_count_at_height(height).await?;

		match count {
			Some(count) => {
				let milestone = self.get_milestone_at_height(count, height).await?;
				Ok(Some((count, milestone)))
			},
			None => Ok(None),
		}
	}

	/// Retrieves a specific milestone at a specific height using ABCI query.
	///
	/// This method queries the Heimdall node's ABCI store to get a specific milestone
	/// at the specified height.
	///
	/// # Arguments
	///
	/// * `milestone_number` - The milestone number to retrieve
	/// * `height` - The height at which to query the milestone
	///
	/// # Returns
	///
	/// - `Ok(Milestone)`: The milestone data
	/// - `Err(ProverError)`: If the request fails or the response cannot be parsed
	///
	/// # Errors
	///
	/// Returns `ProverError` if:
	/// - The ABCI query fails
	/// - The height conversion fails
	/// - The response cannot be deserialized
	pub async fn get_milestone_at_height(
		&self,
		milestone_number: u64,
		height: u64,
	) -> Result<Milestone, ProverError> {
		let mut key = vec![0x81];
		key.extend_from_slice(&milestone_number.to_be_bytes());

		let abci_query: AbciQuery = self
			.http_client
			.abci_query(
				Some("/store/milestone/key".to_string()),
				key,
				Some(Height::try_from(height).unwrap()),
				true,
			)
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;

		let milestone = ismp_polygon::Milestone::proto_decode(&abci_query.value).map_err(|e| {
			ProverError::ConversionError(format!("Failed to decode milestone: {}", e))
		})?;

		Ok(milestone)
	}

	/// Fetches an Ethereum block header from the execution RPC client and converts it to a
	/// CodecHeader.
	///
	/// This method queries the Polygon PoS chain (or Ethereum mainnet) via the execution RPC
	/// to retrieve block header information.
	///
	/// # Arguments
	///
	/// * `block` - The block identifier (number or hash) to fetch. Must implement `Into<BlockId>`.
	///
	/// # Returns
	///
	/// - `Ok(Some(CodecHeader))` if the block exists and conversion succeeds
	/// - `Ok(None)` if the block does not exist
	/// - `Err(ProverError)` if the RPC call fails
	///
	/// # Errors
	///
	/// Returns `ProverError` if:
	/// - The RPC call to get the block fails
	/// - The block conversion fails
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
	/// This method first gets the latest block number, then fetches the corresponding header.
	///
	/// # Returns
	///
	/// - `Ok(CodecHeader)` if the latest block exists and conversion succeeds
	/// - `Err(ProverError)` if the RPC call fails or the block cannot be fetched
	///
	/// # Errors
	///
	/// Returns `ProverError` if:
	/// - The RPC call to get the block number fails
	/// - The latest block header cannot be fetched
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
		let mut all_validators = Vec::new();
		let mut page = 1;
		let page_size = 100;
		let mut expected_total = 0usize;

		loop {
			log::debug!(target: "tesseract", "Requesting validators page {} with page_size {} for height {}",
					   page, page_size, height);

			let heimdall_response: HeimdallValidatorsResponse = self
				.rpc_request(
					"validators",
					json!({
						"height": height.to_string(),
						"page": page.to_string(),
						"per_page": page_size.to_string()
					}),
				)
				.await?;

			log::trace!(target: "tesseract", "Received {} validators in page {}",
					   heimdall_response.validators.len(), page);

			if page == 1 {
				expected_total = heimdall_response
					.total
					.parse()
					.map_err(|_| ProverError::ConversionError("Invalid total count".to_string()))?;

				log::info!(target: "tesseract", "Total validators expected: {} for height {}",
						  expected_total, height);
			}

			let validators_before = all_validators.len();
			all_validators.extend(heimdall_response.clone().validators);

			log::trace!(target: "tesseract", "Added {} validators, total collected: {}/{}",
					   all_validators.len() - validators_before, all_validators.len(), expected_total);

			if all_validators.len() >= expected_total {
				if all_validators.len() > expected_total {
					log::warn!(target: "tesseract", "Collected more validators than expected! Got {}, expected {}. Truncating to expected count.",
							  all_validators.len(), expected_total);
				} else {
					log::debug!(target: "tesseract", "Successfully collected all {} validators for height {}",
							   expected_total, height);
				}

				all_validators.truncate(expected_total);
				break;
			}

			page += 1;
		}

		let complete_response = HeimdallValidatorsResponse {
			block_height: height.to_string(),
			validators: all_validators.clone(),
			count: all_validators.len().to_string(),
			total: expected_total.to_string(),
		};

		let heimdall_response = complete_response;
		let validators_response: ValidatorsResponse = heimdall_response.into();

		Ok(validators_response.validators)
	}

	async fn next_validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		self.validators(height + 1).await
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

// Types

/// RPC Response wrapper for Heimdall client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse<T> {
	/// JSON-RPC version
	pub jsonrpc: String,
	/// Request ID
	#[serde(deserialize_with = "deserialize_rpc_id")]
	pub id: String,
	/// Response result
	pub result: Option<T>,
	/// Error information if request failed
	pub error: Option<RpcError>,
}

// Helper function to deserialize RPC ID as either string or integer
fn deserialize_rpc_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::Deserialize;

	// Try to deserialize as Value first to handle both string and integer
	let value = Value::deserialize(deserializer)?;

	match value {
		Value::String(s) => Ok(s),
		Value::Number(n) => Ok(n.to_string()),
		_ => Err(serde::de::Error::custom("RPC ID must be string or number")),
	}
}

/// RPC error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
	/// Error code
	pub code: i32,
	/// Error message
	pub message: String,
	/// Additional error data
	pub data: Option<Value>,
}

/// Node status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
	/// Node information
	pub node_info: NodeInfo,
	/// Synchronization information
	pub sync_info: SyncInfo,
	/// Validator information
	pub validator_info: ValidatorInfo,
}

/// Node information details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
	/// Protocol version information
	pub protocol_version: ProtocolVersion,
	/// Node identifier
	pub id: String,
	/// Listen address
	pub listen_addr: String,
	/// Network identifier
	pub network: String,
	/// Node version
	pub version: String,
	/// Channel information
	pub channels: String,
	/// Node moniker
	pub moniker: String,
	/// Additional node information
	pub other: std::collections::HashMap<String, Value>,
}

/// Protocol version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolVersion {
	/// P2P protocol version
	#[serde(with = "cometbft::serializers::from_str")]
	pub p2p: u64,
	/// Block protocol version
	#[serde(with = "cometbft::serializers::from_str")]
	pub block: u64,
	/// Application protocol version
	#[serde(with = "cometbft::serializers::from_str")]
	pub app: u64,
}

/// Synchronization information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncInfo {
	/// Latest block hash
	#[serde(with = "cometbft::serializers::hash")]
	pub latest_block_hash: cometbft::Hash,
	/// Latest application hash
	#[serde(with = "cometbft::serializers::apphash")]
	pub latest_app_hash: cometbft::AppHash,
	/// Latest block height
	pub latest_block_height: cometbft::block::Height,
	/// Latest block timestamp
	#[serde(with = "cometbft::serializers::time")]
	pub latest_block_time: cometbft::Time,
	/// Earliest block hash
	#[serde(with = "cometbft::serializers::hash")]
	pub earliest_block_hash: cometbft::Hash,
	/// Earliest application hash
	#[serde(with = "cometbft::serializers::apphash")]
	pub earliest_app_hash: cometbft::AppHash,
	/// Earliest block height
	pub earliest_block_height: cometbft::block::Height,
	/// Earliest block timestamp
	#[serde(with = "cometbft::serializers::time")]
	pub earliest_block_time: cometbft::Time,
	/// Whether node is catching up
	pub catching_up: bool,
}

/// Validator information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
	/// Validator address
	pub address: String,
	/// Validator public key
	pub pub_key: PubKey,
	/// Validator voting power
	pub voting_power: String,
}

/// Commit response containing signed header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResponse {
	/// Signed header information
	#[serde(deserialize_with = "deserialize_signed_header_for_heimdall")]
	pub signed_header: SignedHeader,
	/// Whether the commit is canonical
	pub canonical: bool,
}

/// Validators response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorsResponse {
	/// Block height for validators
	pub block_height: cometbft::block::Height,
	/// List of validators
	pub validators: Vec<cometbft::validator::Info>,
	/// Total number of validators
	#[serde(with = "cometbft::serializers::from_str")]
	pub total: i32,
}

/// Public key representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubKey {
	/// Key type identifier
	#[serde(rename = "type")]
	pub key_type: String,
	/// Base64 encoded key value
	pub value: String,
}

/// Heimdall-specific validator information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeimdallValidator {
	/// Validator address
	pub address: String,
	/// Validator public key
	pub pub_key: PubKey,
	/// Validator voting power
	pub voting_power: String,
	/// Validator proposer priority
	pub proposer_priority: Option<String>,
}

impl From<HeimdallValidator> for cometbft::validator::Info {
	fn from(heimdall_val: HeimdallValidator) -> Self {
		let pub_key = base64::engine::general_purpose::STANDARD
			.decode(&heimdall_val.pub_key.value)
			.ok()
			.and_then(|key_bytes| {
				if heimdall_val.pub_key.key_type == "cometbft/PubKeySecp256k1eth" {
					cometbft::PublicKey::from_raw_secp256k1(&key_bytes)
				} else {
					cometbft::PublicKey::try_from_type_and_bytes(
						&heimdall_val.pub_key.key_type,
						&key_bytes,
					)
					.ok()
				}
			})
			.unwrap_or_else(|| {
				cometbft::PublicKey::Ed25519(
					cometbft::crypto::ed25519::VerificationKey::try_from([0u8; 32].as_slice())
						.unwrap(),
				)
			});

		let power = heimdall_val.voting_power.parse::<u64>().unwrap_or(0);
		let address = custom_account_id_from_pubkey(&pub_key);

		cometbft::validator::Info {
			address,
			pub_key,
			power: cometbft::vote::Power::try_from(power).unwrap_or_default(),
			name: None,
			proposer_priority: heimdall_val
				.proposer_priority
				.map(|p| p.parse::<i64>().unwrap_or(0))
				.map(cometbft::validator::ProposerPriority::from)
				.unwrap_or_default(),
		}
	}
}

/// Heimdall-specific validators response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeimdallValidatorsResponse {
	/// Block height as string
	pub block_height: String,
	/// List of Heimdall validators
	pub validators: Vec<HeimdallValidator>,
	/// Validator count
	pub count: String,
	/// Total validator count
	pub total: String,
}

impl From<HeimdallValidatorsResponse> for ValidatorsResponse {
	fn from(heimdall_resp: HeimdallValidatorsResponse) -> Self {
		let mut validators: Vec<cometbft::validator::Info> =
			heimdall_resp.validators.into_iter().map(Into::into).collect();

		// Sort validators using the same logic as CometBFT's internal sort_validators
		// (v. 0.34 -> first by validator power, descending, then by address, ascending)
		validators.sort_by_key(|v| (core::cmp::Reverse(v.power), v.address));

		let block_height = heimdall_resp.block_height.parse::<u64>().unwrap_or(0);
		let total = heimdall_resp.total.parse::<i32>().unwrap_or(0);

		ValidatorsResponse {
			block_height: cometbft::block::Height::try_from(block_height).unwrap_or_default(),
			validators,
			total,
		}
	}
}

fn deserialize_signed_header_for_heimdall<'de, D>(deserializer: D) -> Result<SignedHeader, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::Deserialize;

	let value: Value = Value::deserialize(deserializer)?;

	let transformed_value = transform_signed_header_json(value);

	SignedHeader::deserialize(transformed_value).map_err(serde::de::Error::custom)
}

fn transform_signed_header_json(mut value: Value) -> Value {
	if let Some(signatures) = value
		.as_object_mut()
		.and_then(|obj| obj.get_mut("commit"))
		.and_then(|commit| commit.as_object_mut())
		.and_then(|commit_obj| commit_obj.get_mut("signatures"))
		.and_then(|sigs| sigs.as_array_mut())
	{
		for signature in signatures {
			if let Some(signature_obj) = signature.as_object_mut() {
				normalize_signature_in_object(signature_obj);
			}
		}
	}
	value
}

fn normalize_signature_in_object(signature_obj: &mut serde_json::Map<String, Value>) {
	if let Some(sig) = signature_obj.get("signature") {
		if let Some(sig_str) = sig.as_str() {
			if let Ok(sig_bytes) = base64::engine::general_purpose::STANDARD.decode(sig_str) {
				if sig_bytes.len() == 65 {
					let trimmed_sig = &sig_bytes[..64];
					let trimmed_sig_b64 =
						base64::engine::general_purpose::STANDARD.encode(trimmed_sig);
					signature_obj.insert("signature".to_string(), json!(trimmed_sig_b64));
				}
			}
		}
	}
}

/// Custom account ID that matches Go CometBFT fork's address calculation
/// For Secp256k1: uses Keccak256 (Ethereum-style) instead of RIPEMD160(SHA256)
pub fn custom_account_id_from_pubkey(pub_key: &PublicKey) -> CometbftAccountId {
	match pub_key {
		PublicKey::Ed25519(pk) => {
			// SHA256(pk)[:20] - same as standard
			use sha2::{Digest, Sha256};
			let digest = Sha256::digest(pk.as_bytes());
			CometbftAccountId::new(digest[..20].try_into().unwrap())
		},
		PublicKey::Secp256k1(pk) => {
			// Keccak256(pubkey)[12:] - Ethereum-style like Go fork
			use sha3::{Digest, Keccak256};
			let pubkey_bytes = pk.to_encoded_point(false).as_bytes().to_vec();
			// Remove the 0x04 prefix (first byte) as done in Go implementation
			let keccak_hash = Keccak256::digest(&pubkey_bytes[1..]);
			// Take last 20 bytes (bytes 12-31)
			CometbftAccountId::new(keccak_hash[12..32].try_into().unwrap())
		},
		#[allow(unreachable_patterns)]
		_ => {
			// Catch-all for non_exhaustive enum
			CometbftAccountId::new([0u8; 20])
		},
	}
}
