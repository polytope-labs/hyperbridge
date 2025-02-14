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

//! Pallet-mmr runtime Apis

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;
use pallet_ismp::offchain::{Proof, ProofKeys};
use polkadot_sdk::*;
use sp_mmr_primitives::{Error, LeafIndex};

sp_api::decl_runtime_apis! {
	/// MmrRuntimeApi
	pub trait MmrRuntimeApi<Hash: codec::Codec, BlockNumber: codec::Codec, Leaf: codec::Codec> {
		/// Return Block number where pallet-mmr was added to the runtime
		fn pallet_genesis() -> Result<Option<BlockNumber>, Error>;

		/// Return the number of MMR leaves.
		fn mmr_leaf_count() -> Result<LeafIndex, Error>;

		/// Return the on-chain MMR root hash.
		fn mmr_root() -> Result<Hash, Error>;

		/// Return the unique hash used as the offchain prefix at a particular block
		fn fork_identifier() -> Result<Hash, Error>;

		/// Generate a proof for the provided leaf indices
		fn generate_proof(
			commitments: ProofKeys
		) -> Result<(Vec<Leaf>, Proof<Hash>), Error>;
	}
}
