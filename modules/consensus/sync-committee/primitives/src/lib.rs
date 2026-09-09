//! Primitive types for sync committee verifier
//! This crate contains code adapted from https://github.com/ralexstokes/ethereum-consensus
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(unused_imports)]
// The ssz containers take their type parameters from the spec constants, so the names are
// screaming snake case rather than camel case. Renaming them would move every use site.
#![allow(non_camel_case_types)]
#[warn(unused_variables)]
extern crate alloc;

pub mod beacon_state;
pub mod consensus_types;
pub mod constants;
pub mod deneb;
pub mod domains;
pub mod electra;
pub mod error;
// Always compiled: the verifier recovers the Gloas execution state root from this rlp header at
// runtime, so it is always compiled.
pub mod execution_header;
// Gloas `BeaconState`/`BeaconBlockBody` component types. Always compiled: the state shape is now
// chosen at runtime from the fork the beacon api reports, so these can no longer be gated.
pub mod gloas;
mod ssz;
pub mod types;
pub mod util;
