#![cfg_attr(not(feature = "std"), no_std)]
pub use cometbft::{
	block::{signed_header::SignedHeader, Header, Height},
	chain::Id as ChainId,
	hash::Hash,
	time::Time,
	validator::{Info as Validator, Set as ValidatorSet},
};

pub mod types;
pub use types::{ConsensusProof, TrustedState, UpdatedTrustedState, VerificationOptions};
pub mod error;
pub mod hashing;
pub mod sp_io_verifier;
pub mod verifier;
pub use error::VerificationError;

pub use sp_io_verifier::SpIoVerifier;
pub use verifier::{verify_header_update, verify_misbehaviour_header};
