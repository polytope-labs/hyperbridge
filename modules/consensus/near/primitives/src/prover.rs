use alloc::{boxed::Box, string::String, vec::Vec};
use async_trait::async_trait;
use codec::{Decode, Encode};
use near_primitives::{
	hash::CryptoHash,
	views::{
		validator_stake_view::ValidatorStakeView, BlockHeaderView, LightClientBlockLiteView,
		LightClientBlockView,
	},
};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A trait defining the interface for interacting with NEAR blockchain nodes.
///
/// This trait provides methods to query blockchain data such as light client blocks,
/// validators, and network status.
#[async_trait]
pub trait Client {
	/// Retrieves the latest block height from the blockchain.
	///
	/// # Returns
	///
	/// - `Ok(u64)`: The latest block height
	/// - `Err(ProverError)`: If the request fails due to network issues or other errors
	async fn latest_height(&self) -> Result<u64, ProverError>;

	/// Retrieves the next light client block after a given block hash.
	///
	/// # Arguments
	///
	/// * `last_block_hash` - The hash of the last verified block
	///
	/// # Returns
	///
	/// - `Ok(Option<LightClientBlockView>)`: The next light client block, or None if not available
	/// - `Err(ProverError)`: If the request fails
	async fn next_light_client_block(
		&self,
		last_block_hash: CryptoHash,
	) -> Result<Option<LightClientBlockView>, ProverError>;

	/// Retrieves a light client block by block hash.
	///
	/// # Arguments
	///
	/// * `block_hash` - The hash of the block to retrieve
	///
	/// # Returns
	///
	/// - `Ok(LightClientBlockView)`: The light client block
	/// - `Err(ProverError)`: If the block is not found or the request fails
	async fn light_client_block(
		&self,
		block_hash: CryptoHash,
	) -> Result<LightClientBlockView, ProverError>;

	/// Retrieves the validator set for a specific epoch.
	///
	/// # Arguments
	///
	/// * `epoch_id` - The epoch ID to query
	///
	/// # Returns
	///
	/// - `Ok(Vec<ValidatorStakeView>)`: The list of validators for the epoch
	/// - `Err(ProverError)`: If the epoch is invalid or the request fails
	async fn validators(
		&self,
		epoch_id: CryptoHash,
	) -> Result<Vec<ValidatorStakeView>, ProverError>;

	/// Retrieves a block header by block hash.
	///
	/// # Arguments
	///
	/// * `block_hash` - The hash of the block
	///
	/// # Returns
	///
	/// - `Ok(BlockHeaderView)`: The block header
	/// - `Err(ProverError)`: If the block is not found or the request fails
	async fn block_header(&self, block_hash: CryptoHash) -> Result<BlockHeaderView, ProverError>;

	/// Checks if the blockchain node is healthy and responding.
	///
	/// # Returns
	///
	/// - `Ok(true)`: If the node is healthy and responding
	/// - `Ok(false)`: If the node is not responding or unhealthy
	/// - `Err(ProverError)`: If the health check request fails
	async fn is_healthy(&self) -> Result<bool, ProverError>;
}

/// Errors that can occur during proof generation
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ProverError {
	/// RPC communication error
	#[error("RPC error: {0}")]
	RpcError(String),

	/// Invalid block height
	#[error("Invalid height: {0}")]
	InvalidHeight(String),

	/// No block found at specified height
	#[error("No block found at height {0}")]
	NoBlockFound(u64),

	/// No light client block found
	#[error("No light client block found for hash: {0}")]
	NoLightClientBlock(String),

	/// No validators found for epoch
	#[error("No validators found for epoch: {0}")]
	NoValidators(String),

	/// Invalid epoch ID
	#[error("Invalid epoch ID: {0}")]
	InvalidEpochId(String),

	/// Data conversion error
	#[error("Conversion error: {0}")]
	ConversionError(String),

	/// Network communication error
	#[error("Network error: {0}")]
	NetworkError(String),

	/// Request timeout error
	#[error("Timeout error: {0}")]
	TimeoutError(String),

	/// Invalid trusted state
	#[error("Invalid trusted state: {0}")]
	InvalidTrustedState(String),

	/// Proof construction failure
	#[error("Proof construction failed: {0}")]
	ProofConstructionError(String),

	/// Block hash mismatch
	#[error("Block hash mismatch: expected {expected}, got {actual}")]
	BlockHashMismatch { expected: String, actual: String },

	/// Epoch transition error
	#[error("Epoch transition error: {0}")]
	EpochTransitionError(String),
}

/// Represents the trusted state for NEAR light client
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedState {
	/// The hash of the last verified block
	pub last_block_hash: CryptoHash,
	/// The height of the last verified block
	pub height: u64,
	/// The epoch ID of the last verified block
	pub epoch_id: CryptoHash,
	/// The next epoch ID
	pub next_epoch_id: CryptoHash,
	/// Block producers for the current epoch
	pub current_block_producers: Vec<ValidatorStakeView>,
	/// Block producers for the next epoch (if available)
	pub next_block_producers: Option<Vec<ValidatorStakeView>>,
}

impl TrustedState {
	pub fn new(
		last_block_hash: CryptoHash,
		height: u64,
		epoch_id: CryptoHash,
		next_epoch_id: CryptoHash,
		current_block_producers: Vec<ValidatorStakeView>,
		next_block_producers: Option<Vec<ValidatorStakeView>>,
	) -> Self {
		Self {
			last_block_hash,
			height,
			epoch_id,
			next_epoch_id,
			current_block_producers,
			next_block_producers,
		}
	}

	/// Validate the trusted state
	pub fn validate(&self) -> Result<(), String> {
		if self.height == 0 {
			return Err("Height cannot be zero".to_string());
		}
		if self.current_block_producers.is_empty() {
			return Err("Current block producers cannot be empty".to_string());
		}
		Ok(())
	}
}

/// Codec version of TrustedState for encoding/decoding
#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, Eq)]
pub struct CodecTrustedState {
	/// The hash of the last verified block
	pub last_block_hash: Vec<u8>,
	/// The height of the last verified block
	pub height: u64,
	/// The epoch ID of the last verified block
	pub epoch_id: Vec<u8>,
	/// The next epoch ID
	pub next_epoch_id: Vec<u8>,
	/// Block producers for the current epoch (serialized)
	pub current_block_producers: Vec<u8>,
	/// Block producers for the next epoch (serialized, if available)
	pub next_block_producers: Option<Vec<u8>>,
}

/// Consensus proof for NEAR light client
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusProof {
	/// The light client block containing the header and approvals
	pub light_client_block: LightClientBlockView,
	/// Validators for the current epoch
	pub current_validators: Vec<ValidatorStakeView>,
	/// Validators for the next epoch (if epoch boundary)
	pub next_validators: Option<Vec<ValidatorStakeView>>,
}

impl ConsensusProof {
	pub fn new(
		light_client_block: LightClientBlockView,
		current_validators: Vec<ValidatorStakeView>,
		next_validators: Option<Vec<ValidatorStakeView>>,
	) -> Self {
		Self { light_client_block, current_validators, next_validators }
	}

	pub fn height(&self) -> u64 {
		self.light_client_block.inner_lite.height
	}

	pub fn block_hash(&self) -> CryptoHash {
		self.light_client_block.inner_rest_hash
	}

	/// Validate the consensus proof
	pub fn validate(&self) -> Result<(), String> {
		if self.height() == 0 {
			return Err("Height cannot be zero".to_string());
		}
		if self.current_validators.is_empty() {
			return Err("Current validators cannot be empty".to_string());
		}
		Ok(())
	}

	/// Check if this proof includes next validators (epoch boundary)
	pub fn has_next_validators(&self) -> bool {
		self.next_validators.is_some() && !self.next_validators.as_ref().unwrap().is_empty()
	}
}

/// Codec version of ConsensusProof for encoding/decoding
#[derive(Encode, Decode, Debug, Clone, TypeInfo)]
pub struct CodecConsensusProof {
	/// The light client block (serialized)
	pub light_client_block: Vec<u8>,
	/// Validators for the current epoch (serialized)
	pub current_validators: Vec<u8>,
	/// Validators for the next epoch (serialized, if available)
	pub next_validators: Option<Vec<u8>>,
}

/// Result of an updated trusted state after verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatedTrustedState {
	/// The new trusted state
	pub trusted_state: TrustedState,
	/// Height of the verified block
	pub verified_height: u64,
	/// Timestamp of the verified block (nanoseconds since Unix epoch)
	pub verified_timestamp: u64,
}

impl UpdatedTrustedState {
	pub fn new(trusted_state: TrustedState, verified_height: u64, verified_timestamp: u64) -> Self {
		Self { trusted_state, verified_height, verified_timestamp }
	}

	/// Get the height difference between the old and new trusted state
	pub fn height_difference(&self) -> u64 {
		self.verified_height.saturating_sub(self.trusted_state.height)
	}

	/// Check if the update was successful
	pub fn is_successful(&self) -> bool {
		self.verified_height > self.trusted_state.height
	}
}
