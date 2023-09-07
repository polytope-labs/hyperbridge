// Implementation of the ethereum beacon consensus client for ISMP
// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

pub mod prelude {
    pub use alloc::{boxed::Box, vec, vec::Vec};
}

pub mod arbitrum;
pub mod beacon_client;
pub mod optimism;
pub mod presets;
#[cfg(test)]
mod tests;
pub mod types;
pub mod utils;

pub use beacon_client::*;
