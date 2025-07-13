pub use cometbft::{
	block::{signed_header::SignedHeader, Header, Height},
	chain::Id as ChainId,
	hash::Hash,
	time::Time,
	validator::{Info as Validator, Set as ValidatorSet},
};

pub mod account;
pub mod client;
pub mod error;
pub mod prover;
pub mod types;

#[cfg(test)]
mod tests;

pub use client::{Client, CometBFTClient, HeimdallClient};
pub use error::ProverError;
pub use prover::{prove_header_update, prove_misbehaviour_header};

pub use account::custom_account_id_from_pubkey;
pub use tendermint_verifier::{
	ConsensusProof, TrustedState, UpdatedTrustedState, VerificationOptions,
};
