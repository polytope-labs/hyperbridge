use std::time::Duration;

use crate::{SignedHeader, Validator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedState {
	/// Chain ID
	pub chain_id: String,
	/// Block height
	pub height: u64,
	/// Block timestamp
	pub timestamp: u64,
	/// Current validator set
	pub validators: Vec<Validator>,
	/// Next validator set
	pub next_validators: Vec<Validator>,
	/// Hash of the next validator set
	pub next_validators_hash: [u8; 32],
	/// Trusting period in seconds
	pub trusting_period: u64,
	/// Frozen height (if any)
	pub frozen_height: Option<u64>,
}

impl TrustedState {
	pub fn new(
		chain_id: String,
		height: u64,
		timestamp: u64,
		validators: Vec<Validator>,
		next_validators: Vec<Validator>,
		next_validators_hash: [u8; 32],
		trusting_period: u64,
	) -> Self {
		Self {
			chain_id,
			height,
			timestamp,
			validators,
			next_validators,
			next_validators_hash,
			trusting_period,
			frozen_height: None,
		}
	}

	pub fn is_frozen(&self) -> bool {
		self.frozen_height.is_some()
	}

	pub fn trusting_period_duration(&self) -> Duration {
		Duration::from_secs(self.trusting_period)
	}

	/// Validate the trusted state
	pub fn validate(&self) -> Result<(), String> {
		if self.chain_id.is_empty() {
			return Err("Chain ID cannot be empty".to_string());
		}
		if self.height == 0 {
			return Err("Height cannot be zero".to_string());
		}
		if self.timestamp == 0 {
			return Err("Timestamp cannot be zero".to_string());
		}
		if self.validators.is_empty() {
			return Err("Validator set cannot be empty".to_string());
		}
		if self.next_validators.is_empty() {
			return Err("Next validator set cannot be empty".to_string());
		}
		if self.trusting_period == 0 {
			return Err("Trusting period cannot be zero".to_string());
		}
		Ok(())
	}

	/// Create a frozen trusted state
	pub fn freeze_at_height(mut self, height: u64) -> Self {
		self.frozen_height = Some(height);
		self
	}

	/// Check if the state is valid for a given height
	pub fn is_valid_for_height(&self, height: u64) -> bool {
		match self.frozen_height {
			Some(frozen_height) => height < frozen_height,
			None => true,
		}
	}
}

impl Default for TrustedState {
	fn default() -> Self {
		Self {
			chain_id: "test-chain".to_string(),
			height: 1,
			timestamp: 0,
			validators: Vec::new(),
			next_validators: Vec::new(),
			next_validators_hash: [0u8; 32],
			trusting_period: 3600, // 1 hour default
			frozen_height: None,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusProof {
	/// Signed header containing the block header and commit
	pub signed_header: SignedHeader,
	/// Validator set at the block height
	pub validators: Vec<Validator>,
	/// Next validator set (optional)
	pub next_validators: Option<Vec<Validator>>,
}

impl ConsensusProof {
	pub fn new(
		signed_header: SignedHeader,
		validators: Vec<Validator>,
		next_validators: Option<Vec<Validator>>,
	) -> Self {
		Self { signed_header, validators, next_validators }
	}

	pub fn height(&self) -> u64 {
		self.signed_header.header.height.value()
	}

	pub fn timestamp(&self) -> u64 {
		self.signed_header.header.time.unix_timestamp() as u64
	}

	pub fn chain_id(&self) -> &str {
		self.signed_header.header.chain_id.as_str()
	}

	/// Validate the consensus proof
	pub fn validate(&self) -> Result<(), String> {
		if self.validators.is_empty() {
			return Err("Validator set cannot be empty".to_string());
		}
		if self.height() == 0 {
			return Err("Height cannot be zero".to_string());
		}
		if self.timestamp() == 0 {
			return Err("Timestamp cannot be zero".to_string());
		}
		if self.chain_id().is_empty() {
			return Err("Chain ID cannot be empty".to_string());
		}
		Ok(())
	}

	/// Check if the proof has next validators
	pub fn has_next_validators(&self) -> bool {
		self.next_validators.is_some()
	}

	/// Get the next validators if available
	pub fn get_next_validators(&self) -> Option<&Vec<Validator>> {
		self.next_validators.as_ref()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationOptions {
	/// Trust threshold as a fraction (numerator/denominator)
	/// Default is 2/3
	pub trust_threshold_numerator: u64,
	pub trust_threshold_denominator: u64,
	/// Clock drift tolerance in seconds
	pub clock_drift: u64,
}

impl VerificationOptions {
	pub fn new(
		trust_threshold_numerator: u64,
		trust_threshold_denominator: u64,
		clock_drift: u64,
	) -> Self {
		Self { trust_threshold_numerator, trust_threshold_denominator, clock_drift }
	}

	pub fn trust_threshold_fraction(&self) -> f64 {
		self.trust_threshold_numerator as f64 / self.trust_threshold_denominator as f64
	}

	pub fn clock_drift_duration(&self) -> Duration {
		Duration::from_secs(self.clock_drift)
	}

	/// Validate the verification options
	pub fn validate(&self) -> Result<(), String> {
		if self.trust_threshold_numerator == 0 {
			return Err("Trust threshold numerator cannot be zero".to_string());
		}
		if self.trust_threshold_denominator == 0 {
			return Err("Trust threshold denominator cannot be zero".to_string());
		}
		if self.trust_threshold_numerator > self.trust_threshold_denominator {
			return Err("Trust threshold numerator cannot be greater than denominator".to_string());
		}
		if self.trust_threshold_fraction() < 0.5 {
			return Err("Trust threshold must be at least 0.5 (50%)".to_string());
		}
		if self.trust_threshold_fraction() > 1.0 {
			return Err("Trust threshold cannot be greater than 1.0 (100%)".to_string());
		}
		Ok(())
	}

	/// Create default verification options (2/3 trust threshold, 5 second clock drift)
	pub fn create_default() -> Self {
		Self { trust_threshold_numerator: 2, trust_threshold_denominator: 3, clock_drift: 5 }
	}

	/// Create verification options with custom trust threshold
	pub fn with_trust_threshold(
		trust_threshold_numerator: u64,
		trust_threshold_denominator: u64,
	) -> Self {
		Self {
			trust_threshold_numerator,
			trust_threshold_denominator,
			clock_drift: 5, // Default 5 seconds
		}
	}
}

impl Default for VerificationOptions {
	fn default() -> Self {
		Self::create_default()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatedTrustedState {
	/// The new trusted state
	pub trusted_state: TrustedState,
	/// Height of the verified header
	pub verified_height: u64,
	/// Timestamp of the verified header
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

	/// Get the time difference between the old and new trusted state
	pub fn time_difference(&self) -> u64 {
		self.verified_timestamp.saturating_sub(self.trusted_state.timestamp)
	}

	/// Check if the update was successful
	pub fn is_successful(&self) -> bool {
		self.verified_height > self.trusted_state.height
	}
}
