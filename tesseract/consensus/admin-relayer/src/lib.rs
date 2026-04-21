// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

//! `tesseract-admin-relayer` — an admin-driven consensus relayer that only
//! forwards mandatory (authority-set handover) BEEFY proofs from Hyperbridge to
//! counterparty EVM chains.
//!
//! Unlike [`tesseract_consensus`], this binary never relays finalized-message
//! proofs. Each consensus update is submitted as an ERC-7821 batch that
//! atomically unfreezes the ISMP host, applies the update, and re-freezes it.
//! The relayer EOA (which must be the ISMP host admin on each chain) is
//! EIP-7702 delegated to a per-chain ERC-7821 Executor on startup.

pub mod batch;
pub mod config;
pub mod delegation;
pub mod logging;
pub mod task;
