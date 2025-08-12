use alloc::{boxed::Box, string::String, vec::Vec};
use async_trait::async_trait;
use cometbft::{block::signed_header::SignedHeader, validator::Info as Validator};
use thiserror::Error;

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

/// Errors that can occur during proof generation
#[derive(Error, Debug, Clone)]
pub enum ProverError {
	/// RPC communication error
	#[error("RPC error: {0}")]
	RpcError(String),

	/// Invalid block height
	#[error("Invalid height: {0}")]
	InvalidHeight(String),

	/// Invalid chain identifier
	#[error("Invalid chain ID: {0}")]
	InvalidChainId(String),

	/// No signed header found at specified height
	#[error("No signed header found at height {0}")]
	NoSignedHeader(u64),

	/// No validators found at specified height
	#[error("No validators found at height {0}")]
	NoValidators(u64),

	/// Invalid ancestry information
	#[error("Invalid ancestry: {0}")]
	InvalidAncestry(String),

	/// Height gap detected between expected and actual values
	#[error("Height gap detected: expected {expected}, got {actual}")]
	HeightGap {
		/// Expected height value
		expected: u64,
		/// Actual height value
		actual: u64,
	},

	/// Chain ID mismatch between expected and actual values
	#[error("Chain ID mismatch: expected {expected}, got {actual}")]
	ChainIdMismatch {
		/// Expected chain ID
		expected: String,
		/// Actual chain ID
		actual: String,
	},

	/// Timestamp-related error
	#[error("Timestamp error: {0}")]
	TimestampError(String),

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
}
