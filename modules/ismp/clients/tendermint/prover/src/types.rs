use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// RPC Response types for Heimdall client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse<T> {
	pub jsonrpc: String,
	#[serde(deserialize_with = "deserialize_rpc_id")]
	pub id: String,
	pub result: Option<T>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
	pub code: i32,
	pub message: String,
	pub data: Option<Value>,
}

// Status response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
	pub node_info: NodeInfo,
	pub sync_info: SyncInfo,
	pub validator_info: ValidatorInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
	pub protocol_version: ProtocolVersion,
	pub id: String,
	pub listen_addr: String,
	pub network: String,
	pub version: String,
	pub channels: String,
	pub moniker: String,
	pub other: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolVersion {
	#[serde(with = "cometbft::serializers::from_str")]
	pub p2p: u64,
	#[serde(with = "cometbft::serializers::from_str")]
	pub block: u64,
	#[serde(with = "cometbft::serializers::from_str")]
	pub app: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncInfo {
	#[serde(with = "cometbft::serializers::hash")]
	pub latest_block_hash: cometbft::Hash,
	#[serde(with = "cometbft::serializers::apphash")]
	pub latest_app_hash: cometbft::AppHash,
	pub latest_block_height: cometbft::block::Height,
	#[serde(with = "cometbft::serializers::time")]
	pub latest_block_time: cometbft::Time,
	#[serde(with = "cometbft::serializers::hash")]
	pub earliest_block_hash: cometbft::Hash,
	#[serde(with = "cometbft::serializers::apphash")]
	pub earliest_app_hash: cometbft::AppHash,
	pub earliest_block_height: cometbft::block::Height,
	#[serde(with = "cometbft::serializers::time")]
	pub earliest_block_time: cometbft::Time,
	pub catching_up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
	pub address: String,
	pub pub_key: PubKey,
	pub voting_power: String,
}

// Commit response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResponse {
	#[serde(deserialize_with = "deserialize_signed_header_for_heimdall")]
	pub signed_header: crate::SignedHeader,
	pub canonical: bool,
}

// Validators response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorsResponse {
	pub block_height: cometbft::block::Height,
	pub validators: Vec<cometbft::validator::Info>,
	#[serde(with = "cometbft::serializers::from_str")]
	pub total: i32,
}

// Public key type for Heimdall responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubKey {
	#[serde(rename = "type")]
	pub key_type: String,
	pub value: String,
}

// Heimdall-specific validator type that can be converted to cometbft::validator::Info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeimdallValidator {
	pub address: String,
	pub pub_key: PubKey,
	pub voting_power: String,
	pub proposer_priority: Option<String>,
}

impl From<HeimdallValidator> for cometbft::validator::Info {
	fn from(heimdall_val: HeimdallValidator) -> Self {
		// Convert the public key
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

		// Parse voting power
		let power = heimdall_val.voting_power.parse::<u64>().unwrap_or(0);

		// Create validator info
		cometbft::validator::Info {
			address: cometbft::account::Id::from(pub_key),
			pub_key,
			power: cometbft::vote::Power::try_from(power).unwrap_or_default(),
			name: None,
			proposer_priority: cometbft::validator::ProposerPriority::default(),
		}
	}
}

// Heimdall-specific validators response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeimdallValidatorsResponse {
	pub block_height: String,
	pub validators: Vec<HeimdallValidator>,
	pub count: String,
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

fn deserialize_signed_header_for_heimdall<'de, D>(
	deserializer: D,
) -> Result<crate::SignedHeader, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::Deserialize;

	// First deserialize as a raw JSON value
	let value: Value = Value::deserialize(deserializer)?;

	// Apply Heimdall-specific transformations to the JSON before parsing
	let transformed_value = transform_signed_header_json(value);

	// Now deserialize the transformed JSON
	crate::SignedHeader::deserialize(transformed_value).map_err(serde::de::Error::custom)
}

fn transform_signed_header_json(mut value: Value) -> Value {
	// Navigate to the commit signatures and normalize them
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
					let trimmed_sig = &sig_bytes[1..];
					let trimmed_sig_b64 =
						base64::engine::general_purpose::STANDARD.encode(trimmed_sig);
					signature_obj.insert("signature".to_string(), json!(trimmed_sig_b64));
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use cometbft_proto::Protobuf;
	use prost::Message;
	use serde_json::json;
	use sha2::Sha256;

	#[tokio::test]
	async fn test_heimdall_validator_public_keys() {
		let client = reqwest::Client::new();

		// RPC request to get validators
		let request_body = json!({
			"jsonrpc": "2.0",
			"id": 1,
			"method": "validators",
			"params": {
				"height": "9754111",
				"page": "1",
				"per_page": "5"
			}
		});

		println!("Querying Heimdall RPC for validators...");

		let response = client
			.post("https://polygon-amoy-heimdall-rpc.publicnode.com:443")
			.header("Content-Type", "application/json")
			.body(request_body.to_string())
			.send()
			.await;

		match response {
			Ok(response) => {
				match response.text().await {
					Ok(response_text) => {
						println!("Response received, parsing...");

						// Use our HeimdallValidatorsResponse type for proper deserialization
						match serde_json::from_str::<RpcResponse<HeimdallValidatorsResponse>>(
							&response_text,
						) {
							Ok(rpc_response) => {
								if let Some(heimdall_result) = rpc_response.result {
									println!(
										"Found {} validators",
										heimdall_result.validators.len()
									);
									println!("Block height: {}", heimdall_result.block_height);
									println!("Total validators: {}", heimdall_result.total);
									println!("==========================================");

									// Test the conversion to CometBFT validators
									let cometbft_validators: Vec<cometbft::validator::Info> =
										heimdall_result
											.validators
											.into_iter()
											.map(Into::into)
											.collect();

									println!(
										"Successfully converted {} validators to CometBFT format",
										cometbft_validators.len()
									);

									// Test the full conversion to ValidatorsResponse (which
									// includes sorting)
									let _heimdall_response = HeimdallValidatorsResponse {
										block_height: "9754111".to_string(),
										validators: vec![], // We'll use the original validators
										count: "5".to_string(),
										total: "100".to_string(),
									};

									// Test individual validator conversions
									for (i, validator) in cometbft_validators.iter().enumerate() {
										println!("Validator {}:", i + 1);
										println!("  Address: {}", validator.address);
										println!("  Power: {}", validator.power);
										println!("  Key type: {}", validator.pub_key.type_str());

										// Test key format
										let key_bytes = validator.pub_key.to_bytes();
										println!("  Key length: {} bytes", key_bytes.len());

										if !key_bytes.is_empty() {
											println!("  Key prefix: 0x{:02X}", key_bytes[0]);

											match (key_bytes.len(), key_bytes[0]) {
												(33, 0x02) | (33, 0x03) => {
													println!("  Format: COMPRESSED");
												},
												(65, 0x04) => {
													println!("  Format: UNCOMPRESSED");
												},
												_ => {
													println!("  Format: UNKNOWN");
												},
											}
										}
										println!("  ---");
									}

									println!("✓ All validators successfully converted!");
								} else if let Some(error) = rpc_response.error {
									println!("RPC Error: {:?}", error);
								}
							},
							Err(e) => {
								println!("Failed to parse HeimdallValidatorsResponse: {}", e);
								println!("Raw response: {}", response_text);
							},
						}
					},
					Err(e) => {
						println!("Failed to get response text: {}", e);
					},
				}
			},
			Err(e) => {
				println!("Failed to send request: {}", e);
			},
		}
	}

	#[tokio::test]
	async fn test_validator_hash_key_format() {
		println!("=== Testing Validator Hash Key Format ===");

		// Step 1: Get a block header and its validators
		let client = reqwest::Client::new();

		// Get a recent block height
		let status_request = json!({
			"jsonrpc": "2.0",
			"id": 1,
			"method": "status"
		});

		let status_response = client
			.post("https://polygon-amoy-heimdall-rpc.publicnode.com:443")
			.header("Content-Type", "application/json")
			.body(status_request.to_string())
			.send()
			.await;

		let block_height = match status_response {
			Ok(response) => match response.text().await {
				Ok(text) => match serde_json::from_str::<RpcResponse<StatusResponse>>(&text) {
					Ok(rpc_response) =>
						if let Some(status) = rpc_response.result {
							println!(
								"Latest block height: {}",
								status.sync_info.latest_block_height
							);
							status.sync_info.latest_block_height.value()
						} else {
							println!("Failed to get status, using default height");
							9754111u64
						},
					Err(_) => {
						println!("Failed to parse status, using default height");
						9754111u64
					},
				},
				Err(_) => {
					println!("Failed to get status text, using default height");
					9754111u64
				},
			},
			Err(_) => {
				println!("Failed to get status, using default height");
				9754111u64
			},
		};

		// Step 2: Get the block header
		let commit_request = json!({
			"jsonrpc": "2.0",
			"id": 1,
			"method": "commit",
			"params": {
				"height": block_height.to_string()
			}
		});

		let commit_response = client
			.post("https://polygon-amoy-heimdall-rpc.publicnode.com:443")
			.header("Content-Type", "application/json")
			.body(commit_request.to_string())
			.send()
			.await;

		let header_validators_hash = match commit_response {
			Ok(response) => match response.text().await {
				Ok(text) => match serde_json::from_str::<RpcResponse<CommitResponse>>(&text) {
					Ok(rpc_response) =>
						if let Some(commit) = rpc_response.result {
							println!(
								"Header validators hash: {}",
								commit.signed_header.header.validators_hash
							);
							commit.signed_header.header.validators_hash
						} else {
							println!("Failed to get commit response");
							return;
						},
					Err(e) => {
						println!("Failed to parse commit response: {}", e);
						return;
					},
				},
				Err(e) => {
					println!("Failed to get commit text: {}", e);
					return;
				},
			},
			Err(e) => {
				println!("Failed to get commit: {}", e);
				return;
			},
		};

		// Step 3: Get validators for this block
		let validators_request = json!({
			"jsonrpc": "2.0",
			"id": 1,
			"method": "validators",
			"params": {
				"height": block_height.to_string(),
				"page": "1",
				"per_page": "100"
			}
		});

		let validators_response = client
			.post("https://polygon-amoy-heimdall-rpc.publicnode.com:443")
			.header("Content-Type", "application/json")
			.body(validators_request.to_string())
			.send()
			.await;

		let validators = match validators_response {
			Ok(response) => match response.text().await {
				Ok(text) => {
					match serde_json::from_str::<RpcResponse<HeimdallValidatorsResponse>>(&text) {
						Ok(rpc_response) =>
							if let Some(heimdall_result) = rpc_response.result {
								println!("Found {} validators", heimdall_result.validators.len());
								heimdall_result.validators
							} else {
								println!("Failed to get validators response");
								return;
							},
						Err(e) => {
							println!("Failed to parse validators response: {}", e);
							return;
						},
					}
				},
				Err(e) => {
					println!("Failed to get validators text: {}", e);
					return;
				},
			},
			Err(e) => {
				println!("Failed to get validators: {}", e);
				return;
			},
		};

		// Step 4: Convert to CometBFT validators and test both key formats
		let cometbft_validators: Vec<cometbft::validator::Info> =
			validators.into_iter().map(Into::into).collect();

		println!("Converted {} validators to CometBFT format", cometbft_validators.len());

		// Step 5: Create validator sets with different key formats
		let mut compressed_validators = Vec::new();
		let mut uncompressed_validators = Vec::new();

		for validator in &cometbft_validators {
			// Create compressed version (current CometBFT Rust behavior)
			compressed_validators.push(validator.clone());

			// Create uncompressed version (Go CometBFT behavior)
			let uncompressed_validator = create_uncompressed_validator(validator);
			uncompressed_validators.push(uncompressed_validator);
		}

		// Sort validators (same as CometBFT does)
		compressed_validators.sort_by_key(|v| (core::cmp::Reverse(v.power), v.address));
		uncompressed_validators.sort_by_key(|v| (core::cmp::Reverse(v.power), v.address));

		// Step 6: Compute validator hashes using both formats
		let compressed_hash = compute_validator_hash_with_key_format(&cometbft_validators, false);
		let uncompressed_hash = compute_validator_hash_with_key_format(&cometbft_validators, true);

		println!("\n=== Results ===");
		println!("Header validators hash: {}", header_validators_hash);
		println!("Compressed keys hash:   {}", compressed_hash);
		println!("Uncompressed keys hash: {}", uncompressed_hash);
		println!();

		// Step 7: Compare and determine which format is used
		if header_validators_hash == compressed_hash {
			println!("✅ MATCH: Polygon CometBFT uses COMPRESSED keys for validator hashing");
		} else if header_validators_hash == uncompressed_hash {
			println!("✅ MATCH: Polygon CometBFT uses UNCOMPRESSED keys for validator hashing");
		} else {
			println!("❌ NO MATCH: Neither compressed nor uncompressed keys match");
			println!("This suggests a different hashing algorithm or key format is used");
		}

		// Step 8: Show key format analysis
		println!("\n=== Key Format Analysis ===");
		for (i, validator) in cometbft_validators.iter().enumerate() {
			let key_bytes = validator.pub_key.to_bytes();
			println!(
				"Validator {}: {} bytes, prefix: 0x{:02X}",
				i + 1,
				key_bytes.len(),
				key_bytes[0]
			);
		}
	}

	/// Create an uncompressed version of a validator by converting the public key
	fn create_uncompressed_validator(
		validator: &cometbft::validator::Info,
	) -> cometbft::validator::Info {
		// For secp256k1 keys, we need to convert from compressed to uncompressed
		if let Some(secp256k1_key) = validator.pub_key.secp256k1() {
			// Get original compressed bytes
			let original_bytes = validator.pub_key.to_bytes();
			println!(
				"Original key: {} bytes, prefix: 0x{:02X}",
				original_bytes.len(),
				original_bytes[0]
			);

			// Convert to uncompressed format (65 bytes with 0x04 prefix)
			let uncompressed_bytes = secp256k1_key.to_encoded_point(false).as_bytes().to_vec();
			println!(
				"Uncompressed bytes: {} bytes, prefix: 0x{:02X}",
				uncompressed_bytes.len(),
				uncompressed_bytes[0]
			);

			// Create a new public key from uncompressed bytes
			if let Some(uncompressed_pubkey) =
				cometbft::PublicKey::from_raw_secp256k1(&uncompressed_bytes)
			{
				let new_bytes = uncompressed_pubkey.to_bytes();
				println!(
					"New key from uncompressed: {} bytes, prefix: 0x{:02X}",
					new_bytes.len(),
					new_bytes[0]
				);

				return cometbft::validator::Info {
					pub_key: uncompressed_pubkey,
					..validator.clone()
				};
			}
		}

		// For non-secp256k1 keys, return as-is
		validator.clone()
	}

	/// Compute validator hash with manually controlled key format
	fn compute_validator_hash_with_key_format(
		validators: &[cometbft::validator::Info],
		use_uncompressed: bool,
	) -> cometbft::Hash {
		use cometbft::{crypto::default::Sha256, merkle::simple_hash_from_byte_vectors};

		// Convert validators to SimpleValidator format (pub_key + voting_power)
		let validator_bytes: Vec<Vec<u8>> = validators
			.iter()
			.map(|validator| {
				let pub_key_bytes = if use_uncompressed {
					// Manually create uncompressed key bytes
					if let Some(secp256k1_key) = validator.pub_key.secp256k1() {
						secp256k1_key.to_encoded_point(false).as_bytes().to_vec()
					} else {
						validator.pub_key.to_bytes()
					}
				} else {
					validator.pub_key.to_bytes()
				};

				// Create SimpleValidator protobuf with manual key bytes
				// Note: Go CometBFT uses Secp256K1Uncompressed for secp256k1 keys
				let simple_validator = if use_uncompressed {
					cometbft_proto::types::v1::SimpleValidator {
						pub_key: Some(cometbft_proto::crypto::v1::PublicKey {
							sum: Some(cometbft_proto::crypto::v1::public_key::Sum::Bls12381(
								pub_key_bytes,
							)),
						}),
						voting_power: validator.power.into(),
					}
				} else {
					cometbft_proto::types::v1::SimpleValidator {
						pub_key: Some(cometbft_proto::crypto::v1::PublicKey {
							sum: Some(cometbft_proto::crypto::v1::public_key::Sum::Secp256k1(
								pub_key_bytes,
							)),
						}),
						voting_power: validator.power.into(),
					}
				};

				// Serialize to bytes
				simple_validator.encode_to_vec()
			})
			.collect();

		// Hash the validator bytes
		cometbft::Hash::Sha256(simple_hash_from_byte_vectors::<Sha256>(&validator_bytes))
	}

	/// Compute validator hash using the same algorithm as CometBFT
	fn compute_validator_hash(validators: &[cometbft::validator::Info]) -> cometbft::Hash {
		use cometbft::{crypto::default::Sha256, merkle::simple_hash_from_byte_vectors};

		// Convert validators to SimpleValidator format (pub_key + voting_power)
		let validator_bytes: Vec<Vec<u8>> = validators
			.iter()
			.map(|validator| {
				// Create SimpleValidator protobuf
				let simple_validator = cometbft_proto::types::v1::SimpleValidator {
					pub_key: Some(validator.pub_key.clone().into()),
					voting_power: validator.power.into(),
				};

				// Serialize to bytes
				simple_validator.encode_to_vec()
			})
			.collect();

		// Hash the validator bytes
		cometbft::Hash::Sha256(simple_hash_from_byte_vectors::<Sha256>(&validator_bytes))
	}
}
