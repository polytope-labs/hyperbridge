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
	/// Hash of the finalized header
	pub finalized_header_hash: [u8; 32],
	/// Current validator set
	pub validators: Vec<Validator>,
	/// Next validator set
	pub next_validators: Vec<Validator>,
	/// Hash of the next validator set
	pub next_validators_hash: [u8; 32],
	/// Trusting period in seconds
	pub trusting_period: u64,
	/// Verification options for this consensus state
	pub verification_options: VerificationOptions,
}

impl TrustedState {
	pub fn new(
		chain_id: String,
		height: u64,
		timestamp: u64,
		finalized_header_hash: [u8; 32],
		validators: Vec<Validator>,
		next_validators: Vec<Validator>,
		next_validators_hash: [u8; 32],
		trusting_period: u64,
		verification_options: VerificationOptions,
	) -> Self {
		Self {
			chain_id,
			height,
			timestamp,
			finalized_header_hash,
			validators,
			next_validators,
			next_validators_hash,
			trusting_period,
			verification_options,
		}
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
		if self.finalized_header_hash == [0u8; 32] {
			return Err("Finalized header hash cannot be zero".to_string());
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
		// Validate verification options
		self.verification_options.validate()?;
		Ok(())
	}

	/// Check if the state is valid for a given height
	pub fn is_valid_for_height(&self, height: u64) -> bool {
		height <= self.height
	}

	/// Update the finalized header hash
	pub fn update_finalized_header_hash(&mut self, new_hash: [u8; 32]) {
		self.finalized_header_hash = new_hash;
	}

	/// Get the finalized header hash
	pub fn get_finalized_header_hash(&self) -> [u8; 32] {
		self.finalized_header_hash
	}

	/// Get verification options
	pub fn get_verification_options(&self) -> &VerificationOptions {
		&self.verification_options
	}

	/// Update verification options
	pub fn update_verification_options(&mut self, options: VerificationOptions) {
		self.verification_options = options;
	}
}

impl Default for TrustedState {
	fn default() -> Self {
		Self {
			chain_id: "test-chain".to_string(),
			height: 1,
			timestamp: 0,
			finalized_header_hash: [0u8; 32],
			validators: Vec::new(),
			next_validators: Vec::new(),
			next_validators_hash: [0u8; 32],
			trusting_period: 3600, // 1 hour default
			verification_options: VerificationOptions::default(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusProof {
	/// Signed header containing the block header and commit
	pub signed_header: SignedHeader,
	/// Ancestry of signed headers from trusted block height to the latest signed header
	pub ancestry: Vec<SignedHeader>,
	/// Next validator set  (optional) - target height + 1
	pub next_validators: Option<Vec<Validator>>,
}

impl ConsensusProof {
	pub fn new(
		signed_header: SignedHeader,
		ancestry: Vec<SignedHeader>,
		next_validators: Option<Vec<Validator>>,
	) -> Self {
		Self { signed_header, ancestry, next_validators }
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

	/// Get the lowest height in the ancestry (trusted block height)
	pub fn trusted_height(&self) -> u64 {
		self.ancestry
			.first()
			.map(|header| header.header.height.value())
			.unwrap_or_else(|| self.signed_header.header.height.value())
	}

	/// Get the highest height in the ancestry (before the latest signed header)
	pub fn ancestry_highest_height(&self) -> u64 {
		self.ancestry
			.last()
			.map(|header| header.header.height.value())
			.unwrap_or_else(|| self.trusted_height())
	}

	/// Validate the consensus proof
	pub fn validate(&self) -> Result<(), String> {
		// Validate that if next_validators_hash is not empty, next_validators must be provided
		let header_next_validators_hash = &self.signed_header.header.next_validators_hash;
		if !header_next_validators_hash.is_empty() {
			// Hash is not empty, so next_validators must be provided
			if self.next_validators.is_none() {
				return Err("Header has non-empty next_validators_hash but consensus proof has no next_validators".to_string());
			}
			if self.next_validators.as_ref().unwrap().is_empty() {
				return Err("Header has non-empty next_validators_hash but consensus proof has empty next_validators".to_string());
			}
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
		self.next_validators.is_some() && !self.next_validators.as_ref().unwrap().is_empty()
	}

	/// Get the next validators if available
	pub fn get_next_validators(&self) -> Option<&Vec<Validator>> {
		self.next_validators.as_ref()
	}

	/// Get the total number of blocks in the proof (ancestry + latest)
	pub fn total_blocks(&self) -> usize {
		self.ancestry.len() + 1
	}

	/// Get all signed headers in order (ancestry + latest)
	pub fn all_signed_headers(&self) -> Vec<&SignedHeader> {
		let mut headers: Vec<&SignedHeader> = self.ancestry.iter().collect();
		headers.push(&self.signed_header);
		headers
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
