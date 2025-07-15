use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha3::{Digest, Keccak256};

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
	pub signed_header: crate::SignedHeader,
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
		let address = crate::custom_account_id_from_pubkey(&pub_key);

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

fn deserialize_signed_header_for_heimdall<'de, D>(
	deserializer: D,
) -> Result<crate::SignedHeader, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::Deserialize;

	let value: Value = Value::deserialize(deserializer)?;

	let transformed_value = transform_signed_header_json(value);

	crate::SignedHeader::deserialize(transformed_value).map_err(serde::de::Error::custom)
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
