use base64::Engine;
use cometbft::validator::Info as Validator;
use reqwest::Client as ReqwestClient;
use serde_json::{json, Value};

use crate::{error::ProverError, SignedHeader};

pub struct PeppermintRpcClient {
	raw_client: ReqwestClient,
	base_url: String,
}

impl PeppermintRpcClient {
	pub fn new(url: &str) -> Self {
		let raw_client = ReqwestClient::new();
		let base_url = url.to_string();
		Self { raw_client, base_url }
	}

	pub async fn latest_height(&self) -> Result<u64, ProverError> {
		println!("Getting latest height from Polygon/peppermint endpoint");
		let response = self.raw_request("status", json!({})).await?;

		let sync_info = response
			.get("sync_info")
			.ok_or_else(|| ProverError::RpcError("No sync_info in response".to_string()))?;

		let height_str = sync_info
			.get("latest_block_height")
			.ok_or_else(|| {
				ProverError::RpcError("No latest_block_height in sync_info".to_string())
			})?
			.as_str()
			.ok_or_else(|| {
				ProverError::RpcError("latest_block_height is not a string".to_string())
			})?;

		height_str
			.parse::<u64>()
			.map_err(|e| ProverError::RpcError(format!("Failed to parse height: {}", e)))
	}

	pub async fn signed_header(&self, height: u64) -> Result<SignedHeader, ProverError> {
		println!("Getting signed header from Polygon/peppermint endpoint");
		let response = self.raw_request("commit", json!({"height": height.to_string()})).await?;
		let signed_header = response
			.get("signed_header")
			.ok_or_else(|| ProverError::RpcError("No signed_header in response".to_string()))?;

		// Detect if this is a Polygon/peppermint endpoint (by chain_id or network name)
		let is_polygon = self.is_polygon_chain().await.unwrap_or(false);
		let cleaned_header = if is_polygon {
			transform_signed_header(signed_header.clone(), height)
		} else {
			signed_header.clone()
		};
		serde_json::from_value(cleaned_header)
			.map_err(|e| ProverError::RpcError(format!("Failed to parse signed_header: {}", e)))
	}

	pub async fn validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		println!("Getting validators from Polygon/peppermint endpoint");
		let response =
			self.raw_request("validators", json!({"height": height.to_string()})).await?;

		let validators = response
			.get("validators")
			.ok_or_else(|| ProverError::RpcError("No validators in response".to_string()))?
			.as_array()
			.ok_or_else(|| ProverError::RpcError("validators is not an array".to_string()))?;

		// Detect if this is a Polygon/peppermint endpoint and apply transformation
		let is_polygon = self.is_polygon_chain().await.unwrap_or(false);

		let mut result = Vec::new();
		for validator in validators {
			let validator_value =
				if is_polygon { transform_validator(validator.clone()) } else { validator.clone() };

			let parsed = serde_json::from_value(validator_value)
				.map_err(|e| ProverError::RpcError(format!("Failed to parse validator: {}", e)))?;
			result.push(parsed);
		}

		Ok(result)
	}

	pub async fn chain_id(&self) -> Result<String, ProverError> {
		println!("Getting chain_id from Polygon/peppermint endpoint");
		let response = self.raw_request("status", json!({})).await?;

		let node_info = response
			.get("node_info")
			.ok_or_else(|| ProverError::RpcError("No node_info in response".to_string()))?;

		let network = node_info
			.get("network")
			.ok_or_else(|| ProverError::RpcError("No network in node_info".to_string()))?
			.as_str()
			.ok_or_else(|| ProverError::RpcError("network is not a string".to_string()))?;

		Ok(network.to_string())
	}

	pub async fn is_healthy(&self) -> Result<bool, ProverError> {
		println!("Checking health of Polygon/peppermint endpoint");
		match self.raw_client.get(&format!("{}/health", self.base_url)).send().await {
			Ok(response) => Ok(response.status().is_success()),
			Err(_) => Ok(false),
		}
	}

	async fn is_polygon_chain(&self) -> Result<bool, ProverError> {
		// Try to get the chain_id/network and check for known Polygon/peppermint values
		let chain_id = self.chain_id().await.unwrap_or_default().to_lowercase();
		Ok(chain_id.contains("heimdall") || chain_id.contains("polygon"))
	}

	async fn raw_request(&self, method: &str, params: Value) -> Result<Value, ProverError> {
		let request_body = json!({
			"jsonrpc": "2.0",
			"id": "1",
			"method": method,
			"params": params
		});

		let response = self
			.raw_client
			.post(&self.base_url)
			.json(&request_body)
			.send()
			.await
			.map_err(|e| ProverError::NetworkError(format!("Request failed: {}", e)))?;

		if !response.status().is_success() {
			return Err(ProverError::NetworkError(format!("HTTP error: {}", response.status())));
		}

		let response_json: Value = response
			.json()
			.await
			.map_err(|e| ProverError::RpcError(format!("Failed to parse response: {}", e)))?;

		// Check for JSON-RPC error
		if let Some(error) = response_json.get("error") {
			return Err(ProverError::RpcError(format!("RPC error: {}", error)));
		}

		response_json
			.get("result")
			.ok_or_else(|| ProverError::RpcError("No result in response".to_string()))
			.map(|v| v.clone())
	}
}

pub fn transform_signed_header(mut signed_header: Value, height: u64) -> Value {
	// Note: Based on actual RPC response, the header structure is standard Tendermint
	// and doesn't contain peppermint-specific fields like num_txs or total_txs

	// Handle commit structure - the actual response uses block_id_flag (not type)
	// and signatures are already in the correct format, but may need signature trimming
	if let Some(commit) = signed_header.get_mut("commit") {
		if let Some(commit_obj) = commit.as_object_mut() {
			if let Some(signatures) = commit_obj.get_mut("signatures") {
				if let Some(signatures_array) = signatures.as_array_mut() {
					for signature in signatures_array {
						if let Some(signature_obj) = signature.as_object_mut() {
							// Handle secp256k1 signature format (65 bytes -> 64 bytes)
							if let Some(sig) = signature_obj.get("signature") {
								if let Some(sig_str) = sig.as_str() {
									if let Ok(sig_bytes) =
										base64::engine::general_purpose::STANDARD.decode(sig_str)
									{
										if sig_bytes.len() == 65 {
											let trimmed_sig = &sig_bytes[1..];
											let trimmed_sig_b64 =
												base64::engine::general_purpose::STANDARD
													.encode(trimmed_sig);
											signature_obj.insert(
												"signature".to_string(),
												json!(trimmed_sig_b64),
											);
										}
									}
								}
							}
						}
					}
				}
			}

			// Ensure required fields are present
			if !commit_obj.contains_key("height") {
				commit_obj.insert("height".to_string(), json!(height.to_string()));
			}
			if !commit_obj.contains_key("round") {
				commit_obj.insert("round".to_string(), json!(0));
			}
		}
	}
	signed_header
}

pub fn transform_validator(mut validator: Value) -> Value {
	// Transform peppermint-specific validator fields to standard tendermint format
	if let Some(pub_key) = validator.get_mut("pub_key") {
		if let Some(pub_key_obj) = pub_key.as_object_mut() {
			// Transform cometbft/PubKeySecp256k1eth to tendermint/PubKeySecp256k1
			if let Some(key_type) = pub_key_obj.get("type") {
				if let Some(type_str) = key_type.as_str() {
					if type_str == "cometbft/PubKeySecp256k1eth" {
						pub_key_obj.insert("type".to_string(), json!("tendermint/PubKeySecp256k1"));
					}
				}
			}
		}
	}
	validator
}
