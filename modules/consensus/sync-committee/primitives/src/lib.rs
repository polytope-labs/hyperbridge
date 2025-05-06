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
mod ssz;
pub mod types;
pub mod util;
