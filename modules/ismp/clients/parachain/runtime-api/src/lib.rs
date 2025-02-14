// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Runtime API for the parachain consensus client.

#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;
use cumulus_pallet_parachain_system::RelayChainState;
use polkadot_sdk::*;

sp_api::decl_runtime_apis! {
	/// Ismp Parachain consensus client runtime APIs
	pub trait IsmpParachainApi {
		/// Return all the para_ids this runtime is interested in. Used by the inherent provider
		fn para_ids() -> Vec<u32>;

		/// Return the current relay chain state.
		fn current_relay_chain_state() -> RelayChainState;
	}
}
