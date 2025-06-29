use std::time::Duration;

use tendermint::{block::Height, chain::Id, Hash, Time};
use tendermint_light_client_verifier::{
	options::Options,
	types::{TrustThreshold, TrustedBlockState, UntrustedBlockState, ValidatorSet},
	Verdict, Verifier,
};
use tendermint_proto::google::protobuf::Timestamp;

use crate::{
	ConsensusProof, SpIoVerifier, TrustedState, UpdatedTrustedState, VerificationError,
	VerificationOptions,
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

	let validators = ValidatorSet::new(trusted_state.validators.clone(), None);
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

	let verifier = SpIoVerifier::default();
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

	let validators = ValidatorSet::new(trusted_state.validators.clone(), None);
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

	let verifier = SpIoVerifier::default();
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
		validators: old_trusted_state.validators.clone(),
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
		verification_options: old_trusted_state.verification_options.clone(),
		finalized_header_hash: old_trusted_state.finalized_header_hash,
	};

	Ok(UpdatedTrustedState::new(
		new_trusted_state,
		consensus_proof.height(),
		consensus_proof.timestamp(),
	))
}
