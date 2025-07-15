use thiserror::Error;

/// Errors that can occur during Tendermint verification
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VerificationError {
	/// Verification failed due to insufficient voting power
	#[error("Not enough validators signed the block: {0}")]
	NotEnoughTrust(String),

	/// Verification failed due to invalid data
	#[error("Invalid verification data: {0}")]
	Invalid(String),

	/// Time-related verification error
	#[error("Time verification failed: {0}")]
	TimeError(String),

	/// Height-related verification error
	#[error("Height verification failed: {0}")]
	HeightError(String),

	/// Chain ID mismatch
	#[error("Chain ID mismatch: expected {expected}, got {got}")]
	ChainIdMismatch { expected: String, got: String },

	/// Validator set verification failed
	#[error("Validator set verification failed: {0}")]
	ValidatorSetError(String),

	/// Commit verification failed
	#[error("Commit verification failed: {0}")]
	CommitError(String),

	/// Trust period expired
	#[error("Trust period expired: {0}")]
	TrustPeriodExpired(String),

	/// Header from the future
	#[error("Header timestamp is in the future: {0}")]
	HeaderFromFuture(String),

	/// Conversion error
	#[error("Conversion error: {0}")]
	ConversionError(String),

	/// State validation error
	#[error("State validation failed: {0}")]
	StateValidationError(String),

	/// Configuration error
	#[error("Configuration error: {0}")]
	ConfigurationError(String),
}
