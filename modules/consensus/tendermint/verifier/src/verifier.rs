use std::time::Duration;

use cometbft::{block::Height, chain::Id, trust_threshold::TrustThresholdFraction, Hash, Time};
use cometbft_light_client_verifier::{
	options::Options,
	types::{TrustedBlockState, UntrustedBlockState, ValidatorSet},
	Verdict, Verifier,
};
use cometbft_proto::google::protobuf::Timestamp;

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

	let validators = ValidatorSet::new(trusted_state.next_validators.clone(), None);
	let next_validators_proof = ValidatorSet::new(consensus_proof.next_validators.clone(), None);

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

	// Validate ancestry chain before cryptographic verification
	validate_ancestry_chain(&consensus_proof, &trusted_state)?;

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

	let validators = ValidatorSet::new(trusted_state.next_validators.clone(), None);
	let next_validators_proof = ValidatorSet::new(consensus_proof.next_validators.clone(), None);

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

	// For misbehaviour detection, we still validate the ancestry chain
	// but the cryptographic verification will be more relaxed
	validate_ancestry_chain(&consensus_proof, &trusted_state)?;

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

/// Validate ancestry chain by following parent hashes back to trusted finalized hash
fn validate_ancestry_chain(
	consensus_proof: &ConsensusProof,
	trusted_state: &TrustedState,
) -> Result<(), VerificationError> {
	// If there's no ancestry, the target header should directly connect to trusted state
	if consensus_proof.ancestry.is_empty() {
		let target_parent_hash = consensus_proof
			.signed_header
			.header
			.last_block_id
			.as_ref()
			.ok_or_else(|| {
				VerificationError::Invalid(
					"Target header has no last_block_id (possibly genesis block)".to_string(),
				)
			})?
			.hash;

		if target_parent_hash.as_bytes() != trusted_state.finalized_header_hash {
			return Err(VerificationError::Invalid(format!(
				"Target header parent hash {:?} does not match trusted finalized hash {:?}",
				target_parent_hash.as_bytes(),
				trusted_state.finalized_header_hash
			)));
		}
		return Ok(());
	}

	// Start from the target header and work backwards through ancestry
	let mut expected_parent_hash = consensus_proof
		.signed_header
		.header
		.last_block_id
		.as_ref()
		.ok_or_else(|| {
			VerificationError::Invalid("Target header has no last_block_id".to_string())
		})?
		.hash;

	for (i, header) in consensus_proof.ancestry.iter().enumerate().rev() {
		let header_hash = header.header.hash();
		if header_hash.as_bytes() != expected_parent_hash.as_bytes() {
			return Err(VerificationError::Invalid(format!(
				"Ancestry header {} hash mismatch: expected {:?}, got {:?}",
				i,
				expected_parent_hash.as_bytes(),
				header_hash.as_bytes()
			)));
		}

		expected_parent_hash = header
			.header
			.last_block_id
			.as_ref()
			.ok_or_else(|| {
				VerificationError::Invalid(format!(
					"Ancestry header {} has no last_block_id (possibly genesis block)",
					i
				))
			})?
			.hash;
	}

	if expected_parent_hash.as_bytes() != trusted_state.finalized_header_hash {
		return Err(VerificationError::Invalid(format!(
			"Ancestry chain does not connect to trusted finalized hash: expected {:?}, got {:?}",
			trusted_state.finalized_header_hash,
			expected_parent_hash.as_bytes()
		)));
	}

	Ok(())
}

// Helper functions for type conversion

fn convert_verification_options(
	options: &VerificationOptions,
	trusting_period: Duration,
) -> Result<Options, VerificationError> {
	let trust_threshold = TrustThresholdFraction::new(
		options.trust_threshold_numerator,
		options.trust_threshold_denominator,
	)
	.map_err(|e| VerificationError::ConversionError(format!("Invalid trust threshold: {}", e)))?;

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
	let new_next_validators_hash = consensus_proof
		.signed_header
		.header
		.next_validators_hash
		.as_bytes()
		.try_into()
		.map_err(|_| VerificationError::Invalid("Invalid next_validators_hash".to_string()))?;

	// Check if validator set rotation is needed
	let (validators, next_validators) =
		if new_next_validators_hash != old_trusted_state.next_validators_hash {
			// Rotate validator set: current validators become next_validators,
			// and new next_validators come from the signed header
			(old_trusted_state.next_validators.clone(), consensus_proof.next_validators.clone())
		} else {
			// No rotation needed
			(old_trusted_state.validators.clone(), consensus_proof.next_validators.clone())
		};

	let mut new_trusted_state = TrustedState {
		chain_id: consensus_proof.chain_id().to_string(),
		height: consensus_proof.height(),
		timestamp: consensus_proof.timestamp(),
		validators,
		next_validators,
		next_validators_hash: new_next_validators_hash,
		trusting_period: old_trusted_state.trusting_period,
		verification_options: old_trusted_state.verification_options.clone(),
		finalized_header_hash: consensus_proof
			.signed_header
			.header
			.hash()
			.as_bytes()
			.try_into()
			.map_err(|_| VerificationError::Invalid("Invalid finalized_header_hash".to_string()))?,
	};

	Ok(UpdatedTrustedState::new(
		new_trusted_state,
		consensus_proof.height(),
		consensus_proof.timestamp(),
	))
}
