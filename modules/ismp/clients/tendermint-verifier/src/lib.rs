pub use tendermint::{
	block::{signed_header::SignedHeader, Header, Height},
	chain::Id as ChainId,
	hash::Hash,
	time::Time,
	validator::{Info as Validator, Set as ValidatorSet},
};

pub mod types;
pub use types::{ConsensusProof, TrustedState, UpdatedTrustedState, VerificationOptions};
pub mod error;
pub mod verifier;
pub use error::VerificationError;


pub use verifier::{
	verify_header_update,
	verify_misbehaviour_header,
	verify_validator_sets,
	verify_commit,
	verify_commit_against_trusted,
	validate_trusted_state,
	validate_consensus_proof_against_trusted,
};
