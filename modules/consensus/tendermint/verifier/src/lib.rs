#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use cometbft::{
	block::{signed_header::SignedHeader, Header, Height},
	chain::Id as ChainId,
	hash::Hash,
	time::Time,
	validator::{Info as Validator, Set as ValidatorSet},
};

pub mod hashing;
pub mod sp_io_verifier;
pub mod verifier;

pub use sp_io_verifier::{validate_validator_set_hash, SpIoVerifier};
pub use verifier::{verify_header_update, verify_misbehaviour_header};
