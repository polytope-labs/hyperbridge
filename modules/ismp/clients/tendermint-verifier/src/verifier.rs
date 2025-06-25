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
	let next_validators_proof = ValidatorSet::new(
		consensus_proof.next_validators.clone().unwrap_or_default(),
		None,
	);

	let untrusted_block_state = UntrustedBlockState {
		signed_header: &consensus_proof.signed_header,
		validators: &validators,
		next_validators: Some(&next_validators_proof),
	};

	let verifier_options = convert_verification_options(&options, trusted_state.trusting_period_duration())?;
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
	let next_validators_proof = ValidatorSet::new(
		consensus_proof.next_validators.clone().unwrap_or_default(),
		None,
	);

	let untrusted_block_state = UntrustedBlockState {
		signed_header: &consensus_proof.signed_header,
		validators: &validators,
		next_validators: Some(&next_validators_proof),
	};

	let verifier_options = convert_verification_options(&options, trusted_state.trusting_period_duration())?;
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
pub fn verify_validator_sets(
	consensus_proof: &ConsensusProof,
) -> Result<(), VerificationError> {
	let validators = ValidatorSet::new(consensus_proof.validators.clone(), None);
	let next_validators = ValidatorSet::new(
		consensus_proof.next_validators.clone().unwrap_or_default(),
		None,
	);

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
	let next_validators = ValidatorSet::new(
		consensus_proof.next_validators.clone().unwrap_or_default(),
		None,
	);

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
	let next_validators_proof = ValidatorSet::new(
		consensus_proof.next_validators.clone().unwrap_or_default(),
		None,
	);

	let untrusted_block_state = UntrustedBlockState {
		signed_header: &consensus_proof.signed_header,
		validators: &validators,
		next_validators: Some(&next_validators_proof),
	};

	let verifier_options = convert_verification_options(options, trusted_state.trusting_period_duration())?;
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
	let time_diff = now.duration_since(trusted_time)
		.map_err(|e| VerificationError::TimeError(e.to_string()))?;
	
	if time_diff > trusting_period + clock_drift {
		return Err(VerificationError::TrustPeriodExpired(
			format!("Trusted state expired. Time diff: {:?}, Trusting period: {:?}", 
				time_diff, trusting_period)
		));
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
		return Err(VerificationError::HeightError(
			format!("Consensus proof height {} must be greater than trusted height {}", 
				consensus_proof.height(), trusted_state.height)
		));
	}
	
	// Check time monotonicity
	if consensus_proof.timestamp() <= trusted_state.timestamp {
		return Err(VerificationError::TimeError(
			format!("Consensus proof timestamp {} must be greater than trusted timestamp {}", 
				consensus_proof.timestamp(), trusted_state.timestamp)
		));
	}
	
	// Check if header is from the future
	let now = convert_timestamp(current_time)?;
	let proof_time = convert_timestamp(consensus_proof.timestamp())?;
	let clock_drift = options.clock_drift_duration();
	let future_threshold = now.checked_add(clock_drift)
		.ok_or_else(|| VerificationError::TimeError("Time overflow".to_string()))?;
	
	if proof_time > future_threshold {
		return Err(VerificationError::HeaderFromFuture(
			format!("Header timestamp {} is in the future (current: {})", 
				consensus_proof.timestamp(), current_time)
		));
	}
	
	Ok(())
}

// Helper functions for type conversion

fn convert_verification_options(options: &VerificationOptions, trusting_period: Duration) -> Result<Options, VerificationError> {
	let trust_threshold = TrustThreshold::new(
		options.trust_threshold_numerator, 
		options.trust_threshold_denominator
	).map_err(|e| {
		VerificationError::ConversionError(format!("Invalid trust threshold: {}", e))
	})?;
	
	Ok(Options {
		trust_threshold,
		trusting_period,
		clock_drift: options.clock_drift_duration(),
	})
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
