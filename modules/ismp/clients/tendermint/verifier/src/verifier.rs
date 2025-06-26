use std::time::Duration;

use tendermint::{block::Height, chain::Id, Hash, Time};
use tendermint_light_client_verifier::{
	options::Options,
	types::{TrustThreshold, TrustedBlockState, UntrustedBlockState, ValidatorSet},
	ProdVerifier, Verdict, Verifier,
};
use tendermint_proto::google::protobuf::Timestamp;

use crate::{
	ConsensusProof, TrustedState, UpdatedTrustedState, VerificationError, VerificationOptions,
};

/// Main verification function for header updates
pub fn verify_header_update(
	trusted_state: TrustedState,
	consensus_proof: ConsensusProof,
	options: VerificationOptions,
	current_time: u64,
) -> Result<UpdatedTrustedState, VerificationError> {
	if trusted_state.is_frozen() {
		return Err(VerificationError::Invalid("Trusted state is frozen".to_string()));
	}

	let chain_id = Id::try_from(trusted_state.chain_id.clone())
		.map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let height = Height::try_from(trusted_state.height)
		.map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let timestamp = Timestamp { seconds: trusted_state.timestamp as i64, nanos: 0 };
	let time = Time::try_from(timestamp).map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let next_validators = ValidatorSet::new(trusted_state.next_validators.clone(), None);
	let next_validators_hash = Hash::Sha256(trusted_state.next_validators_hash);

	let tendermint_trusted_state = TrustedBlockState {
		chain_id: &chain_id,
		header_time: time,
		height,
		next_validators: &next_validators,
		next_validators_hash,
	};

	let validators = ValidatorSet::new(consensus_proof.validators.clone(), None);
	let next_validators_proof =
		ValidatorSet::new(consensus_proof.next_validators.clone().unwrap_or_default(), None);

	let untrusted_block_state = UntrustedBlockState {
		signed_header: &consensus_proof.signed_header,
		validators: &validators,
		next_validators: Some(&next_validators_proof),
	};

	let verifier_options =
		convert_verification_options(&options, trusted_state.trusting_period_duration())?;
	let now = convert_timestamp(current_time)?;

	let verifier = ProdVerifier::default();
	let result = verifier.verify_update_header(
		untrusted_block_state,
		tendermint_trusted_state,
		&verifier_options,
		now,
	);

	match result {
		Verdict::Success => {
			let updated_state = create_updated_trusted_state(&trusted_state, &consensus_proof)?;
			Ok(updated_state)
		},
		Verdict::NotEnoughTrust(tally) =>
			Err(VerificationError::NotEnoughTrust(format!("Voting power tally: {}", tally))),
		Verdict::Invalid(detail) => Err(VerificationError::Invalid(format!("{:?}", detail))),
	}
}

/// Verify a header for misbehaviour detection (more relaxed verification)
pub fn verify_misbehaviour_header(
	trusted_state: TrustedState,
	consensus_proof: ConsensusProof,
	options: VerificationOptions,
	current_time: u64,
) -> Result<UpdatedTrustedState, VerificationError> {
	if trusted_state.is_frozen() {
		return Err(VerificationError::Invalid("Trusted state is frozen".to_string()));
	}

	let chain_id = Id::try_from(trusted_state.chain_id.clone())
		.map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let height = Height::try_from(trusted_state.height)
		.map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let timestamp = Timestamp { seconds: trusted_state.timestamp as i64, nanos: 0 };
	let time = Time::try_from(timestamp).map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let next_validators = ValidatorSet::new(trusted_state.next_validators.clone(), None);
	let next_validators_hash = Hash::Sha256(trusted_state.next_validators_hash);

	let tendermint_trusted_state = TrustedBlockState {
		chain_id: &chain_id,
		header_time: time,
		height,
		next_validators: &next_validators,
		next_validators_hash,
	};

	let validators = ValidatorSet::new(consensus_proof.validators.clone(), None);
	let next_validators_proof =
		ValidatorSet::new(consensus_proof.next_validators.clone().unwrap_or_default(), None);

	let untrusted_block_state = UntrustedBlockState {
		signed_header: &consensus_proof.signed_header,
		validators: &validators,
		next_validators: Some(&next_validators_proof),
	};

	let verifier_options =
		convert_verification_options(&options, trusted_state.trusting_period_duration())?;
	let now = convert_timestamp(current_time)?;

	let verifier = ProdVerifier::default();
	let result = verifier.verify_misbehaviour_header(
		untrusted_block_state,
		tendermint_trusted_state,
		&verifier_options,
		now,
	);

	match result {
		Verdict::Success => {
			let updated_state = create_updated_trusted_state(&trusted_state, &consensus_proof)?;
			Ok(updated_state)
		},
		Verdict::NotEnoughTrust(tally) =>
			Err(VerificationError::NotEnoughTrust(format!("Voting power tally: {}", tally))),
		Verdict::Invalid(detail) => Err(VerificationError::Invalid(format!("{:?}", detail))),
	}
}

/// Verify validator sets independently
pub fn verify_validator_sets(consensus_proof: &ConsensusProof) -> Result<(), VerificationError> {
	let validators = ValidatorSet::new(consensus_proof.validators.clone(), None);
	let next_validators =
		ValidatorSet::new(consensus_proof.next_validators.clone().unwrap_or_default(), None);

	let untrusted_block_state = UntrustedBlockState {
		signed_header: &consensus_proof.signed_header,
		validators: &validators,
		next_validators: Some(&next_validators),
	};

	let verifier = ProdVerifier::default();
	let result = verifier.verify_validator_sets(&untrusted_block_state);

	match result {
		Verdict::Success => Ok(()),
		Verdict::NotEnoughTrust(tally) =>
			Err(VerificationError::NotEnoughTrust(format!("Voting power tally: {}", tally))),
		Verdict::Invalid(detail) => Err(VerificationError::Invalid(format!("{:?}", detail))),
	}
}

/// Verify commit independently
pub fn verify_commit(consensus_proof: &ConsensusProof) -> Result<(), VerificationError> {
	let validators = ValidatorSet::new(consensus_proof.validators.clone(), None);
	let next_validators =
		ValidatorSet::new(consensus_proof.next_validators.clone().unwrap_or_default(), None);

	let untrusted_block_state = UntrustedBlockState {
		signed_header: &consensus_proof.signed_header,
		validators: &validators,
		next_validators: Some(&next_validators),
	};

	let verifier = ProdVerifier::default();
	let result = verifier.verify_commit(&untrusted_block_state);

	match result {
		Verdict::Success => Ok(()),
		Verdict::NotEnoughTrust(tally) =>
			Err(VerificationError::NotEnoughTrust(format!("Voting power tally: {}", tally))),
		Verdict::Invalid(detail) => Err(VerificationError::Invalid(format!("{:?}", detail))),
	}
}

/// Verify commit against trusted state
pub fn verify_commit_against_trusted(
	trusted_state: &TrustedState,
	consensus_proof: &ConsensusProof,
	options: &VerificationOptions,
) -> Result<(), VerificationError> {
	// Convert trusted state
	let chain_id = Id::try_from(trusted_state.chain_id.clone())
		.map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let height = Height::try_from(trusted_state.height)
		.map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let timestamp = Timestamp { seconds: trusted_state.timestamp as i64, nanos: 0 };
	let time = Time::try_from(timestamp).map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let next_validators = ValidatorSet::new(trusted_state.next_validators.clone(), None);
	let next_validators_hash = Hash::Sha256(trusted_state.next_validators_hash);

	let tendermint_trusted_state = TrustedBlockState {
		chain_id: &chain_id,
		header_time: time,
		height,
		next_validators: &next_validators,
		next_validators_hash,
	};

	// Convert consensus proof
	let validators = ValidatorSet::new(consensus_proof.validators.clone(), None);
	let next_validators_proof =
		ValidatorSet::new(consensus_proof.next_validators.clone().unwrap_or_default(), None);

	let untrusted_block_state = UntrustedBlockState {
		signed_header: &consensus_proof.signed_header,
		validators: &validators,
		next_validators: Some(&next_validators_proof),
	};

	let verifier_options =
		convert_verification_options(options, trusted_state.trusting_period_duration())?;
	let verifier = ProdVerifier::default();

	let result = verifier.verify_commit_against_trusted(
		&untrusted_block_state,
		&tendermint_trusted_state,
		&verifier_options,
	);

	match result {
		Verdict::Success => Ok(()),
		Verdict::NotEnoughTrust(tally) =>
			Err(VerificationError::NotEnoughTrust(format!("Voting power tally: {}", tally))),
		Verdict::Invalid(detail) => Err(VerificationError::Invalid(format!("{:?}", detail))),
	}
}

/// Validate trusted state against current time
pub fn validate_trusted_state(
	trusted_state: &TrustedState,
	options: &VerificationOptions,
	current_time: u64,
) -> Result<(), VerificationError> {
	let now = convert_timestamp(current_time)?;
	let trusted_time = convert_timestamp(trusted_state.timestamp)?;
	let trusting_period = Duration::from_secs(trusted_state.trusting_period);
	let clock_drift = options.clock_drift_duration();

	// Check if trusted state is within trusting period
	let time_diff = now
		.duration_since(trusted_time)
		.map_err(|e| VerificationError::TimeError(e.to_string()))?;

	if time_diff > trusting_period + clock_drift {
		return Err(VerificationError::TrustPeriodExpired(format!(
			"Trusted state expired. Time diff: {:?}, Trusting period: {:?}",
			time_diff, trusting_period
		)));
	}

	Ok(())
}

/// Validate consensus proof against trusted state (without full verification)
pub fn validate_consensus_proof_against_trusted(
	trusted_state: &TrustedState,
	consensus_proof: &ConsensusProof,
	options: &VerificationOptions,
	current_time: u64,
) -> Result<(), VerificationError> {
	// Check chain ID
	if consensus_proof.chain_id() != trusted_state.chain_id {
		return Err(VerificationError::ChainIdMismatch {
			expected: trusted_state.chain_id.clone(),
			got: consensus_proof.chain_id().to_string(),
		});
	}

	// Check height monotonicity
	if consensus_proof.height() <= trusted_state.height {
		return Err(VerificationError::HeightError(format!(
			"Consensus proof height {} must be greater than trusted height {}",
			consensus_proof.height(),
			trusted_state.height
		)));
	}

	// Check time monotonicity
	if consensus_proof.timestamp() <= trusted_state.timestamp {
		return Err(VerificationError::TimeError(format!(
			"Consensus proof timestamp {} must be greater than trusted timestamp {}",
			consensus_proof.timestamp(),
			trusted_state.timestamp
		)));
	}

	// Check if header is from the future
	let now = convert_timestamp(current_time)?;
	let proof_time = convert_timestamp(consensus_proof.timestamp())?;
	let clock_drift = options.clock_drift_duration();
	let future_threshold = now
		.checked_add(clock_drift)
		.ok_or_else(|| VerificationError::TimeError("Time overflow".to_string()))?;

	if proof_time > future_threshold {
		return Err(VerificationError::HeaderFromFuture(format!(
			"Header timestamp {} is in the future (current: {})",
			consensus_proof.timestamp(),
			current_time
		)));
	}

	Ok(())
}

// Helper functions for type conversion

fn convert_verification_options(
	options: &VerificationOptions,
	trusting_period: Duration,
) -> Result<Options, VerificationError> {
	let trust_threshold =
		TrustThreshold::new(options.trust_threshold_numerator, options.trust_threshold_denominator)
			.map_err(|e| {
				VerificationError::ConversionError(format!("Invalid trust threshold: {}", e))
			})?;

	Ok(Options { trust_threshold, trusting_period, clock_drift: options.clock_drift_duration() })
}

fn convert_timestamp(timestamp: u64) -> Result<Time, VerificationError> {
	Time::from_unix_timestamp(timestamp as i64, 0)
		.map_err(|e| VerificationError::Invalid(e.to_string()))
}

fn create_updated_trusted_state(
	old_trusted_state: &TrustedState,
	consensus_proof: &ConsensusProof,
) -> Result<UpdatedTrustedState, VerificationError> {
	// Create new trusted state with the verified header information
	let new_trusted_state = TrustedState {
		chain_id: consensus_proof.chain_id().to_string(),
		height: consensus_proof.height(),
		timestamp: consensus_proof.timestamp(),
		validators: consensus_proof.validators.clone(),
		next_validators: consensus_proof.next_validators.clone().unwrap_or_default(),
		next_validators_hash: consensus_proof
			.signed_header
			.header
			.next_validators_hash
			.as_bytes()
			.try_into()
			.unwrap(),
		trusting_period: old_trusted_state.trusting_period,
		frozen_height: old_trusted_state.frozen_height,
	};

	Ok(UpdatedTrustedState::new(
		new_trusted_state,
		consensus_proof.height(),
		consensus_proof.timestamp(),
	))
}

#[cfg(test)]
mod tests {
	use super::*;
	use tendermint::{block::CommitSig, Time};
	use tendermint_light_client_verifier::types::LightBlock;
	use tendermint_testgen::{
		light_block::LightBlock as TestgenLightBlock, Generator, Header, Validator as TestValidator,
	};

	#[test]
	fn test_verification_failure_on_chain_id_mismatch() {
		let now = Time::now();
		// Create a default light block with a valid chain-id for height `1` with a timestamp 20
		// secs before now (to be treated as trusted state)
		let light_block_1 = TestgenLightBlock::new_default_with_time_and_chain_id(
			"chain-1".to_owned(),
			now.checked_sub(Duration::from_secs(20)).unwrap(),
			1u64,
		)
		.generate()
		.unwrap();
		let l_b_1: LightBlock = LightBlock {
			signed_header: light_block_1.signed_header,
			validators: light_block_1.validators,
			next_validators: light_block_1.next_validators,
			provider: light_block_1.provider,
		};

		// Create another default block with a different chain-id for height `2` with a timestamp 10
		// secs before now (to be treated as untrusted state)
		let light_block_2 = TestgenLightBlock::new_default_with_time_and_chain_id(
			"forged-chain".to_owned(),
			now.checked_sub(Duration::from_secs(10)).unwrap(),
			2u64,
		)
		.generate()
		.unwrap();

		let l_b_2: LightBlock = LightBlock {
			signed_header: light_block_2.signed_header,
			validators: light_block_2.validators,
			next_validators: light_block_2.next_validators,
			provider: light_block_2.provider,
		};

		let trusted_state = TrustedState {
			chain_id: "testchain".to_string(),
			height: 1,
			timestamp: (now.unix_timestamp() as u64) - 20,
			validators: l_b_1.validators.validators.clone(),
			next_validators: l_b_1.next_validators.validators.clone(),
			next_validators_hash: l_b_1.next_validators.hash().as_bytes().try_into().unwrap(),
			trusting_period: 60,
			frozen_height: None,
		};

		let consensus_proof = ConsensusProof {
			signed_header: l_b_2.signed_header,
			validators: l_b_2.validators.validators.clone(),
			next_validators: Some(l_b_2.next_validators.validators.clone()),
		};

		let options = VerificationOptions::default();
		let current_time = 1;

		let result = verify_header_update(trusted_state, consensus_proof, options, current_time);
		match result {
			Err(VerificationError::Invalid(error_msg)) => {
				assert!(error_msg.contains("ChainIdMismatch"));
				assert!(error_msg.contains("forged-chain"));
				assert!(error_msg.contains("testchain"));
			},
			_ => panic!("Expected Invalid error with chain ID mismatch, got: {:?}", result),
		}
	}

	#[test]
	fn test_successful_verify_maliciousupdate_header() {
		let now = Time::now();

		let verification_options = VerificationOptions::default();

		let validators = [
			TestValidator::new("EVIL").voting_power(51),
			TestValidator::new("GOOD").voting_power(50),
		];

		let header = Header::new(&validators.clone())
			.height(1u64)
			.chain_id("test-chain")
			.next_validators(&validators)
			.time(now.checked_sub(Duration::from_secs(20)).unwrap());

		let trusted_block = TestgenLightBlock::new_default_with_header(header).generate().unwrap();

		let t_b: LightBlock = LightBlock {
			signed_header: trusted_block.signed_header,
			validators: trusted_block.validators,
			next_validators: trusted_block.next_validators,
			provider: trusted_block.provider,
		};

		let header2 = Header::new(&validators)
			.height(2u64)
			.chain_id("test-chain")
			.next_validators(&validators)
			.time(now.checked_sub(Duration::from_secs(10)).unwrap());

		let untrusted_block =
			TestgenLightBlock::new_default_with_header(header2).generate().unwrap();

		let mut u_b: LightBlock = LightBlock {
			signed_header: untrusted_block.signed_header,
			validators: untrusted_block.validators,
			next_validators: untrusted_block.next_validators,
			provider: untrusted_block.provider,
		};

		u_b.signed_header.commit.signatures[1] = CommitSig::BlockIdFlagAbsent;

		let trusted_state = TrustedState {
			chain_id: "test-chain".to_string(),
			height: 1,
			timestamp: (now.unix_timestamp() as u64) - 20,
			validators: t_b.validators.validators.clone(),
			next_validators: t_b.next_validators.validators.clone(),
			next_validators_hash: t_b.next_validators.hash().as_bytes().try_into().unwrap(),
			trusting_period: 60,
			frozen_height: None,
		};

		let mut consensus_proof = ConsensusProof {
			signed_header: u_b.signed_header,
			validators: u_b.validators.validators.clone(),
			next_validators: Some(u_b.next_validators.validators.clone()),
		};

		let result = verify_header_update(
			trusted_state.clone(),
			consensus_proof.clone(),
			verification_options.clone(),
			now.unix_timestamp() as u64,
		);

		match result {
			Err(VerificationError::Invalid(error_msg)) => {
				assert!(error_msg.contains("InsufficientSignersOverlap"));
			},
			_ => panic!(
				"Expected Invalid error with insufficient signers overlap, got: {:?}",
				result
			),
		}

		// Modify the second validator's address to collide with the malicious one.
		// This does not change the validator set hash (as the address is not part of it), but will
		// cause the voting_power_in_impl to double count the single existing commit vote.

		u_b.validators.validators[1].address = u_b.validators.validators[0].address;

		consensus_proof.validators = u_b.validators.validators.clone();
		consensus_proof.next_validators = Some(u_b.next_validators.validators.clone());

		let result = verify_header_update(
			trusted_state,
			consensus_proof,
			verification_options,
			now.unix_timestamp() as u64,
		);

		match result {
			Err(VerificationError::Invalid(error_msg)) => {
				assert!(error_msg.contains("DuplicateValidator"));
			},
			_ => panic!("Expected Invalid error with duplicate validator, got: {:?}", result),
		}
	}
}
