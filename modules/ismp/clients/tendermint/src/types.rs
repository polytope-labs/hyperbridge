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
}
