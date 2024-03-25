// Copyright (C) 2023 Polytope Labs.
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
//! Pallet-ismp runtime Apis

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![deny(missing_docs)]

use ismp::{
    consensus::{ConsensusClientId, StateMachineId},
    router::{Request, Response},
};
use pallet_ismp::{
    mmr_primitives::{Leaf, LeafIndex},
    primitives::{Error, Proof},
    ProofKeys,
};
use sp_core::H256;
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    /// ISMP Runtime Apis
    pub trait IsmpRuntimeApi<Hash: codec::Codec> {
        /// Return the number of MMR leaves.
        fn mmr_leaf_count() -> Result<LeafIndex, Error>;

        /// Return the on-chain MMR root hash.
        fn mmr_root() -> Result<Hash, Error>;

        /// Generate a proof for the provided leaf indices
        fn generate_proof(
            commitments: ProofKeys
        ) -> Result<(Vec<Leaf>, Proof<Hash>), Error>;

        /// Fetch all ISMP events
        fn block_events() -> Vec<pallet_ismp::events::Event>;

        /// Fetch all ISMP events and their extrinsic metadata
        fn block_events_with_metadata() -> Vec<(pallet_ismp::events::Event, u32)>;

        /// Return the scale encoded consensus state
        fn consensus_state(id: ConsensusClientId) -> Option<Vec<u8>>;

        /// Return the timestamp this client was last updated in seconds
        fn consensus_update_time(id: ConsensusClientId) -> Option<u64>;

        /// Return the challenge period timestamp
        fn challenge_period(id: ConsensusClientId) -> Option<u64>;

        /// Return the latest height of the state machine
        fn latest_state_machine_height(id: StateMachineId) -> Option<u64>;

        /// Get actual requests
        fn get_requests(leaf_positions: Vec<H256>) -> Vec<Request>;

        /// Get actual responses
        fn get_responses(leaf_positions: Vec<H256>) -> Vec<Response>;
    }
}
