use cometbft_light_client_verifier::{
	errors::VerificationError,
	operations::{
		ProdCommitValidator, ProvidedVotingPowerCalculator, VotingPowerCalculator, VotingPowerTally,
	},
	options::Options,
	predicates::VerificationPredicates,
	types::{SignedHeader, TrustThreshold, TrustedBlockState, UntrustedBlockState, ValidatorSet},
	PredicateVerifier, Verdict, Verifier,
};

use crate::hashing::{SpIoSha256, SpIoSignatureVerifier};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpIoPredicates;

impl VerificationPredicates for SpIoPredicates {
	type Sha256 = SpIoSha256;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpIoVotingPowerCalculator;

impl VotingPowerCalculator for SpIoVotingPowerCalculator {
	fn voting_power_in(
		&self,
		signed_header: &SignedHeader,
		validator_set: &ValidatorSet,
		trust_threshold: TrustThreshold,
	) -> Result<VotingPowerTally, VerificationError> {
		let calculator = ProvidedVotingPowerCalculator::<SpIoSignatureVerifier>::default();
		calculator.voting_power_in(signed_header, validator_set, trust_threshold)
	}

	fn voting_power_in_sets(
		&self,
		signed_header: &SignedHeader,
		first_set: (&ValidatorSet, TrustThreshold),
		second_set: (&ValidatorSet, TrustThreshold),
	) -> Result<(VotingPowerTally, VotingPowerTally), VerificationError> {
		let calculator = ProvidedVotingPowerCalculator::<SpIoSignatureVerifier>::default();
		calculator.voting_power_in_sets(signed_header, first_set, second_set)
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpIoVerifier;

impl Verifier for SpIoVerifier {
	fn verify_update_header(
		&self,
		untrusted_block_state: UntrustedBlockState,
		trusted_block_state: TrustedBlockState,
		options: &Options,
		now: cometbft::time::Time,
	) -> Verdict {
		let verifier =
			PredicateVerifier::new(SpIoPredicates, SpIoVotingPowerCalculator, ProdCommitValidator);
		verifier.verify_update_header(untrusted_block_state, trusted_block_state, options, now)
	}

	fn verify_misbehaviour_header(
		&self,
		untrusted_block_state: UntrustedBlockState,
		trusted_block_state: TrustedBlockState,
		options: &Options,
		now: cometbft::time::Time,
	) -> Verdict {
		let verifier =
			PredicateVerifier::new(SpIoPredicates, SpIoVotingPowerCalculator, ProdCommitValidator);
		verifier.verify_misbehaviour_header(
			untrusted_block_state,
			trusted_block_state,
			options,
			now,
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use cometbft::crypto::{default::Sha256 as DefaultSha256, Sha256};

	#[test]
	fn test_hash_consistency() {
		let test_data = b"Hello, CometBFT!";

		let sp_io_hash = SpIoSha256::digest(test_data);
		let default_hash = DefaultSha256::digest(test_data);
		println!("sp_io_hash: {:?}", sp_io_hash);
		println!("default_hash: {:?}", default_hash);

		assert_eq!(sp_io_hash, default_hash, "Hash implementations should be identical");
	}
}
