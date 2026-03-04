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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use polkadot_sdk::*;

sp_api::decl_runtime_apis! {
	pub trait IntentsCoprocessorApi {
		/// Returns `(commitment, filler_bytes, user_op)` for `place_bid` extrinsics,
		/// or `(commitment, filler_bytes, empty)` for `retract_bid`. `None` otherwise.
		fn extract_bid(extrinsic: Vec<u8>) -> Option<(sp_core::H256, Vec<u8>, Vec<u8>)>;

		/// Returns all confirmed bids for a given order commitment as
		/// `Vec<(filler_bytes, user_op)>`.
		///
		/// Requires `OffchainDbExt` to be registered by the caller.
		fn get_bids_for_commitment(commitment: sp_core::H256) -> Vec<(Vec<u8>, Vec<u8>)>;
	}
}
