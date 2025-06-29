pub use tendermint::{
	block::{signed_header::SignedHeader, Header, Height},
	chain::Id as ChainId,
	hash::Hash,
	time::Time,
	validator::{Info as Validator, Set as ValidatorSet},
};

pub use tendermint_testgen::Validator as TestValidator;

pub mod types;
pub use types::{ConsensusProof, TrustedState, UpdatedTrustedState, VerificationOptions};
pub mod error;
pub mod verifier;
pub use error::VerificationError;

pub use verifier::{verify_header_update, verify_misbehaviour_header};
