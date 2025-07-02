use thiserror::Error;

/// Errors that can occur during proof generation
#[derive(Error, Debug, Clone)]
pub enum ProverError {
	#[error("RPC error: {0}")]
	RpcError(String),

	#[error("Invalid height: {0}")]
	InvalidHeight(String),

	#[error("Invalid chain ID: {0}")]
	InvalidChainId(String),

	#[error("No signed header found at height {0}")]
	NoSignedHeader(u64),

	#[error("No validators found at height {0}")]
	NoValidators(u64),

	#[error("Invalid ancestry: {0}")]
	InvalidAncestry(String),

	#[error("Height gap detected: expected {expected}, got {actual}")]
	HeightGap { expected: u64, actual: u64 },

	#[error("Chain ID mismatch: expected {expected}, got {actual}")]
	ChainIdMismatch { expected: String, actual: String },

	#[error("Timestamp error: {0}")]
	TimestampError(String),

	#[error("Conversion error: {0}")]
	ConversionError(String),

	#[error("Network error: {0}")]
	NetworkError(String),

	#[error("Timeout error: {0}")]
	TimeoutError(String),

	#[error("Invalid trusted state: {0}")]
	InvalidTrustedState(String),

	#[error("Proof construction failed: {0}")]
	ProofConstructionError(String),
}

impl From<tendermint_rpc::Error> for ProverError {
	fn from(err: tendermint_rpc::Error) -> Self {
		ProverError::RpcError(err.to_string())
	}
}

impl From<std::time::SystemTimeError> for ProverError {
	fn from(err: std::time::SystemTimeError) -> Self {
		ProverError::TimestampError(err.to_string())
	}
}
