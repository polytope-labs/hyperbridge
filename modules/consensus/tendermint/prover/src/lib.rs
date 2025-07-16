#![deny(missing_docs)]

//! Tendermint consensus prover for generating light client proofs
//!
//! This crate provides functionality for generating and verifying Tendermint consensus proofs,
//! supporting both standard CometBFT and Heimdall clients.

/// Re-exported CometBFT types for convenience
pub use cometbft::{
	block::{signed_header::SignedHeader, Header, Height},
	chain::Id as ChainId,
	hash::Hash,
	time::Time,
	validator::{Info as Validator, Set as ValidatorSet},
};

/// Account-related functionality
pub mod account;
/// Client implementations for different Tendermint variants
pub mod client;
/// Error types for the prover
pub mod error;
/// Core proof generation functionality
pub mod prover;
/// Type definitions for RPC responses
pub mod types;

#[cfg(test)]
mod tests;

/// Client trait and implementations
pub use client::{Client, CometBFTClient, HeimdallClient};
/// Error type for proof generation operations
pub use error::ProverError;
/// Functions for generating header update and misbehaviour proofs
pub use prover::{prove_header_update, prove_misbehaviour_header};

/// Utility function for creating custom account IDs from public keys
pub use account::custom_account_id_from_pubkey;
