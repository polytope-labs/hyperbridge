#![allow(clippy::all)]
#![allow(missing_docs)]
//! This module contains sol! macro generated bindings for solidity contracts.
//! These bindings are generated using alloy-sol-macro from compiled ABI JSON files.
//!
//! Each binding compiles under both `std` and `no_std`: the `std` variant adds
//! `#[sol(rpc)]` to emit provider-backed contract call bindings (which depend on
//! `alloy-contract`, `alloy-provider`, `alloy-network` and `alloy-transport`, all of
//! which are std-only). Under `no_std` only the ABI types + codec impls are emitted,
//! which is what substrate pallets consume.

pub mod ecdsa_beefy;
pub mod erc20;
pub mod evm_host;
pub mod handler;
pub mod host_manager;
pub mod ping_module;
pub mod sp1_beefy;
