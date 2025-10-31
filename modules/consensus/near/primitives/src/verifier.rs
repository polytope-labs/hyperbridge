use alloc::string::String;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during NEAR light client verification
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationError {
	/// Verification failed due to insufficient approvals
	#[error("Not enough approvals: {0}")]
	NotEnoughApprovals(String),

	/// Verification failed due to invalid data
	#[error("Invalid verification data: {0}")]
	Invalid(String),

	/// Height-related verification error
	#[error("Height verification failed: {0}")]
	HeightError(String),

	/// Block hash mismatch
	#[error("Block hash mismatch: expected {expected}, got {got}")]
	BlockHashMismatch { expected: String, got: String },

	/// Validator set verification failed
	#[error("Validator set verification failed: {0}")]
	ValidatorSetError(String),

	/// Epoch verification failed
	#[error("Epoch verification failed: {0}")]
	EpochError(String),

	/// Signature verification failed
	#[error("Signature verification failed: {0}")]
	SignatureError(String),

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

	/// Invalid next block producers
	#[error("Invalid next block producers: {0}")]
	InvalidNextBlockProducers(String),

	/// Approvals verification failed
	#[error("Approvals verification failed: {0}")]
	ApprovalsVerificationFailed(String),
}

/// Verification options for NEAR light client
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationOptions {
	/// Minimum approval threshold (e.g., 2/3 of total stake)
	pub approval_threshold_numerator: u64,
	pub approval_threshold_denominator: u64,
}

impl VerificationOptions {
	pub fn new(approval_threshold_numerator: u64, approval_threshold_denominator: u64) -> Self {
		Self { approval_threshold_numerator, approval_threshold_denominator }
	}

	pub fn approval_threshold_fraction(&self) -> f64 {
		self.approval_threshold_numerator as f64 / self.approval_threshold_denominator as f64
	}

	/// Validate the verification options
	pub fn validate(&self) -> Result<(), String> {
		if self.approval_threshold_numerator == 0 {
			return Err("Approval threshold numerator cannot be zero".to_string());
		}
		if self.approval_threshold_denominator == 0 {
			return Err("Approval threshold denominator cannot be zero".to_string());
		}
		if self.approval_threshold_numerator > self.approval_threshold_denominator {
			return Err(
				"Approval threshold numerator cannot be greater than denominator".to_string()
			);
		}
		if self.approval_threshold_fraction() < 0.5 {
			return Err("Approval threshold must be at least 0.5 (50%)".to_string());
		}
		if self.approval_threshold_fraction() > 1.0 {
			return Err("Approval threshold cannot be greater than 1.0 (100%)".to_string());
		}
		Ok(())
	}

	/// Create default verification options (2/3 approval threshold)
	pub fn create_default() -> Self {
		Self { approval_threshold_numerator: 2, approval_threshold_denominator: 3 }
	}
}

impl Default for VerificationOptions {
	fn default() -> Self {
		Self::create_default()
	}
}
