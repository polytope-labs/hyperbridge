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

/// Client implementations for different Tendermint variants
pub mod client;
/// Chain-specific key layouts and builders
pub mod keys;
/// Core proof generation functionality
pub mod prover;

#[cfg(test)]
mod tests;

/// Client trait and implementations
pub use client::CometBFTClient;
/// Functions for generating header update and misbehaviour proofs
pub use prover::{prove_header_update, prove_misbehaviour_header};
