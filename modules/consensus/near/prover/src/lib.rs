#![deny(missing_docs)]

//! NEAR consensus prover for generating light client proofs
//!
//! This crate provides functionality for generating and verifying NEAR consensus proofs
//! using the NEAR light client protocol.

/// Re-exported NEAR types for convenience
pub use near_primitives::{
	hash::CryptoHash,
	types::{BlockHeight, BlockReference, TransactionOrReceiptId},
	views::{
		validator_stake_view::ValidatorStakeView, BlockHeaderView, LightClientBlockLiteView,
		LightClientBlockView,
	},
};

pub use near_primitives_ismp::prover::{
	Client, ConsensusProof, ProverError, TrustedState, UpdatedTrustedState,
};

/// Client implementations for NEAR RPC
pub mod client;
/// Core proof generation functionality
pub mod prover;

/// Client trait and implementations
pub use client::{Config, NearRpcClient, Network};
/// Functions for generating header update and misbehaviour proofs
pub use prover::{create_initial_trusted_state, prove_header_update, prove_misbehaviour_header};
