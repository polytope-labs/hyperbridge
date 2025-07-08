#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		ConsensusProof, TrustedState, UpdatedTrustedState, VerificationError, VerificationOptions,
	};

	#[test]
	fn test_verification_options_creation() {
		let options = VerificationOptions::new(2, 3, 5);
		assert_eq!(options.trust_threshold_numerator, 2);
		assert_eq!(options.trust_threshold_denominator, 3);
		assert_eq!(options.clock_drift, 5);
	}

	#[test]
	fn test_trusted_state_creation() {
		let verification_options = VerificationOptions::default();
		let validators = vec![];
		let next_validators = vec![];

		let trusted_state = TrustedState::new(
			"test-chain".to_string(),
			1,
			1000,
			[0u8; 32],
			validators,
			next_validators,
			[0u8; 32],
			3600,
			verification_options,
		);

		assert_eq!(trusted_state.chain_id, "test-chain");
		assert_eq!(trusted_state.height, 1);
		assert_eq!(trusted_state.timestamp, 1000);
		assert_eq!(trusted_state.trusting_period, 3600);
	}

	#[test]
	fn test_verification_options_validation() {
		let options = VerificationOptions::new(2, 3, 5);
		assert!(options.validate().is_ok());

		// Test invalid trust threshold (numerator > denominator)
		let invalid_options = VerificationOptions::new(4, 3, 5);
		assert!(invalid_options.validate().is_err());
	}
}
