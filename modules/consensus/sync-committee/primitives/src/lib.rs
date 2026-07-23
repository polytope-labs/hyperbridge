//! Primitive types for sync committee verifier
//! This crate contains code adapted from https://github.com/ralexstokes/ethereum-consensus
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(unused_imports)]
#[warn(unused_variables)]
extern crate alloc;

pub mod consensus_types;
pub mod constants;
pub mod deneb;
pub mod domains;
pub mod electra;
pub mod error;
// Always compiled: the verifier recovers the Gloas execution state root from this rlp header at
// runtime, so it can no longer live behind the `glamsterdam` feature.
pub mod execution_header;
// Gloas `BeaconState`/`BeaconBlockBody` component types, needed only by the prover's compile-time
// ssz layout.
#[cfg(feature = "glamsterdam")]
pub mod gloas;
mod ssz;
pub mod types;
pub mod util;
