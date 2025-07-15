use thiserror::Error;

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

impl From<cometbft_rpc::Error> for ProverError {
	fn from(err: cometbft_rpc::Error) -> Self {
		ProverError::RpcError(err.to_string())
	}
}

impl From<std::time::SystemTimeError> for ProverError {
	fn from(err: std::time::SystemTimeError) -> Self {
		ProverError::TimestampError(err.to_string())
	}
}
