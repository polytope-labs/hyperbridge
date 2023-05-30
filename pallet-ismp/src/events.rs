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
//! Core ISMP events

use crate::{Config, Event as PalletEvent};
use alloc::collections::BTreeSet;
use ismp_rs::{
    consensus::{ConsensusClientId, StateMachineHeight, StateMachineId},
    host::StateMachine,
};

/// Ismp Core Protocol Events
#[derive(Clone, codec::Encode, codec::Decode, Debug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Event {
    /// Emitted when a state machine is successfully updated to a new height
    StateMachineUpdated {
        /// State machine id
        state_machine_id: StateMachineId,
        /// Latest height
        latest_height: u64,
    },
    /// Emitted when a challenge period has begun for a consensus client
    ChallengePeriodStarted {
        /// Consensus client id
        consensus_client_id: ConsensusClientId,
        /// Tuple of previous height and latest height
        state_machines: BTreeSet<(StateMachineHeight, StateMachineHeight)>,
    },
    /// Emitted for an outgoing response
    Response {
        /// Chain that this response will be routed to
        dest_chain: StateMachine,
        /// Source Chain for this response
        source_chain: StateMachine,
        /// Nonce for the request which this response is for
        request_nonce: u64,
    },
    /// Emitted for an outgoing request
    Request {
        /// Chain that this request will be routed to
        dest_chain: StateMachine,
        /// Source Chain for request
        source_chain: StateMachine,
        /// Request nonce
        request_nonce: u64,
    },
}

/// Convert from pallet event to Ismp event
pub fn to_core_protocol_event<T: Config>(event: PalletEvent<T>) -> Option<Event> {
    match event {
        PalletEvent::StateMachineUpdated { state_machine_id, latest_height } => {
            Some(Event::StateMachineUpdated { state_machine_id, latest_height })
        }
        PalletEvent::Response { dest_chain, source_chain, request_nonce } => {
            Some(Event::Response { dest_chain, source_chain, request_nonce })
        }
        PalletEvent::Request { dest_chain, source_chain, request_nonce } => {
            Some(Event::Request { dest_chain, source_chain, request_nonce })
        }
        PalletEvent::ChallengePeriodStarted { consensus_client_id, state_machines } => {
            Some(Event::ChallengePeriodStarted { consensus_client_id, state_machines })
        }
        _ => None,
    }
}
