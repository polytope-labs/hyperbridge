// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0.

//! Runtime API for `pallet-beefy-consensus-proofs`.
//!
//! Exposes a single method that returns the `ProofAccepted` events emitted by
//! the pallet in the queried block. Paired with a range-scanning RPC so the
//! relayer can replace its per-block event polling loop with a single server-
//! side range query.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;
use pallet_beefy_consensus_proofs::types::ProofAcceptedEvent;
use polkadot_sdk::*;

sp_api::decl_runtime_apis! {
	/// Runtime API surface for `pallet-beefy-consensus-proofs`.
	pub trait BeefyConsensusProofsRuntimeApi {
		/// Return every `Event::ProofAccepted` deposited by the pallet in the
		/// current block.
		fn proof_accepted_events() -> Vec<ProofAcceptedEvent>;
	}
}
