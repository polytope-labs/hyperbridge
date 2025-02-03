// Copyright (c) 2025 Polytope Labs.
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

#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;
use ismp::{
	consensus::{ConsensusClientId, StateMachineHeight, StateMachineId},
	host::StateMachine,
	router::{Request, Response},
};
use polkadot_sdk::*;
use primitive_types::H256;

sp_api::decl_runtime_apis! {
	/// Required runtime APIs needed for client subsystems like the RPC
	pub trait IsmpRuntimeApi<Hash: codec::Codec> {
		/// Should return the host's state machine identifier
		fn host_state_machine() -> StateMachine;

		/// Fetch all ISMP events
		fn block_events() -> Vec<ismp::events::Event>;

		/// Fetch all ISMP events and their extrinsic metadata
		fn block_events_with_metadata() -> Vec<(ismp::events::Event, Option<u32>)>;

		/// Return the scale encoded consensus state
		fn consensus_state(id: ConsensusClientId) -> Option<Vec<u8>>;

		/// Return the timestamp this client was last updated in seconds
		fn state_machine_update_time(id: StateMachineHeight) -> Option<u64>;

		/// Return the challenge period timestamp
		fn challenge_period(id: StateMachineId) -> Option<u64>;

		/// Return the latest height of the state machine
		fn latest_state_machine_height(id: StateMachineId) -> Option<u64>;

		/// Fetch the requests for the given commitments.
		fn requests(request_commitments: Vec<H256>) -> Vec<Request>;

		/// Fetch the responses for the given commitments.
		fn responses(response_commitments: Vec<H256>) -> Vec<Response>;
	}
}
