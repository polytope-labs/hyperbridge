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

//! Pallet event definitions

use crate::{errors::HandlingError, Config, Event as PalletEvent, Pallet};
use alloc::vec::Vec;
use frame_support::BoundedVec;
use ismp::{
    consensus::StateMachineId,
    error::Error,
    events::{RequestResponseHandled, TimeoutHandled},
    host::StateMachine,
};
use sp_core::H256;

/// Ismp Handler Events
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
    /// Emitted for an outgoing response
    Response {
        /// Chain that this response will be routed to
        dest_chain: StateMachine,
        /// Source Chain for this response
        source_chain: StateMachine,
        /// Nonce for the request which this response is for
        request_nonce: u64,
        /// Commitment
        commitment: H256,
    },
    /// Emitted for an outgoing request
    Request {
        /// Chain that this request will be routed to
        dest_chain: StateMachine,
        /// Source Chain for request
        source_chain: StateMachine,
        /// Request nonce
        request_nonce: u64,
        /// Commitment
        commitment: H256,
    },
    /// Post Request Handled
    PostRequestHandled(RequestResponseHandled),
    /// Post Response Handled
    PostResponseHandled(RequestResponseHandled),
    /// Get Response Handled
    GetRequestHandled(RequestResponseHandled),
    /// Post request timeout handled
    PostRequestTimeoutHandled(TimeoutHandled),
    /// Post response timeout handled
    PostResponseTimeoutHandled(TimeoutHandled),
    /// Get request timeout handled
    GetRequestTimeoutHandled(TimeoutHandled),
}

/// Convert from pallet event to Ismp event
pub fn to_handler_events<T: Config>(event: PalletEvent<T>) -> Option<Event> {
    match event {
        PalletEvent::StateMachineUpdated { state_machine_id, latest_height } =>
            Some(Event::StateMachineUpdated { state_machine_id, latest_height }),
        PalletEvent::Response { dest_chain, source_chain, request_nonce, commitment } =>
            Some(Event::Response { dest_chain, source_chain, request_nonce, commitment }),
        PalletEvent::Request { dest_chain, source_chain, request_nonce, commitment } =>
            Some(Event::Request { dest_chain, source_chain, request_nonce, commitment }),
        PalletEvent::GetRequestTimeoutHandled(handled) =>
            Some(Event::GetRequestTimeoutHandled(handled)),
        PalletEvent::GetRequestHandled(handled) => Some(Event::GetRequestHandled(handled)),
        PalletEvent::PostRequestHandled(handled) => Some(Event::PostRequestHandled(handled)),
        PalletEvent::PostResponseHandled(handled) => Some(Event::PostResponseHandled(handled)),
        PalletEvent::PostRequestTimeoutHandled(handled) =>
            Some(Event::PostRequestTimeoutHandled(handled)),
        PalletEvent::PostResponseTimeoutHandled(handled) =>
            Some(Event::PostResponseTimeoutHandled(handled)),
        // We are only converting events useful relayers and applications
        PalletEvent::ConsensusClientCreated { .. } |
        PalletEvent::ConsensusClientFrozen { .. } |
        PalletEvent::Errors { .. } |
        PalletEvent::__Ignore(_, _) |
        PalletEvent::StateCommitmentVetoed { .. } => None,
    }
}

impl<T: Config> From<ismp::events::Event> for PalletEvent<T> {
    fn from(event: ismp::events::Event) -> Self {
        match event {
            ismp::events::Event::PostRequestHandled(handled) =>
                PalletEvent::<T>::PostRequestHandled(handled),
            ismp::events::Event::PostResponseHandled(handled) =>
                PalletEvent::<T>::PostResponseHandled(handled),
            ismp::events::Event::PostRequestTimeoutHandled(handled) =>
                PalletEvent::<T>::PostRequestTimeoutHandled(handled),
            ismp::events::Event::PostResponseTimeoutHandled(handled) =>
                PalletEvent::<T>::PostResponseTimeoutHandled(handled),
            ismp::events::Event::GetRequestHandled(handled) =>
                PalletEvent::<T>::GetRequestHandled(handled),
            ismp::events::Event::GetRequestTimeoutHandled(handled) =>
                PalletEvent::<T>::GetRequestTimeoutHandled(handled),
            ismp::events::Event::StateMachineUpdated(ev) => PalletEvent::<T>::StateMachineUpdated {
                state_machine_id: ev.state_machine_id,
                latest_height: ev.latest_height,
            },
            ismp::events::Event::StateCommitmentVetoed(ev) =>
                PalletEvent::<T>::StateCommitmentVetoed {
                    height: ev.height,
                    fisherman: BoundedVec::truncate_from(ev.fisherman),
                },
            // These events are only deposited when messages are dispatched, they should never
            // be deposited when a message is handled
            ismp::events::Event::PostRequest(_) |
            ismp::events::Event::PostResponse(_) |
            ismp::events::Event::GetRequest(_) => {
                unimplemented!("Event should not originate from handler")
            },
        }
    }
}

/// Deposit some ismp events
/// We only want to deposit Request Handled and time out events at this point
pub fn deposit_ismp_events<T: Config>(
    results: Vec<Result<ismp::events::Event, Error>>,
    errors: &mut Vec<HandlingError>,
) {
    for result in results {
        match result {
            Ok(event) => match event {
                ismp::events::Event::PostRequestHandled(handled) =>
                    Pallet::<T>::deposit_event(PalletEvent::<T>::PostRequestHandled(handled)),
                ismp::events::Event::PostResponseHandled(handled) =>
                    Pallet::<T>::deposit_event(PalletEvent::<T>::PostResponseHandled(handled)),
                ismp::events::Event::PostRequestTimeoutHandled(handled) =>
                    Pallet::<T>::deposit_event(PalletEvent::<T>::PostRequestTimeoutHandled(handled)),
                ismp::events::Event::PostResponseTimeoutHandled(handled) =>
                    Pallet::<T>::deposit_event(PalletEvent::<T>::PostResponseTimeoutHandled(
                        handled,
                    )),
                ismp::events::Event::GetRequestHandled(handled) =>
                    Pallet::<T>::deposit_event(PalletEvent::<T>::GetRequestHandled(handled)),
                ismp::events::Event::GetRequestTimeoutHandled(handled) =>
                    Pallet::<T>::deposit_event(PalletEvent::<T>::GetRequestTimeoutHandled(handled)),
                ismp::events::Event::StateMachineUpdated(ev) =>
                    Pallet::<T>::deposit_event(PalletEvent::<T>::StateMachineUpdated {
                        state_machine_id: ev.state_machine_id,
                        latest_height: ev.latest_height,
                    }),
                ismp::events::Event::StateCommitmentVetoed(ev) =>
                    Pallet::<T>::deposit_event(PalletEvent::<T>::StateCommitmentVetoed {
                        height: ev.height,
                        fisherman: BoundedVec::truncate_from(ev.fisherman),
                    }),
                // These events are only deposited when messages are dispatched, they should never
                // be deposited when a message is handled
                ismp::events::Event::PostRequest(_) => {},
                ismp::events::Event::PostResponse(_) => {},
                ismp::events::Event::GetRequest(_) => {},
            },
            Err(err) => errors.push(err.into()),
        }
    }
}
