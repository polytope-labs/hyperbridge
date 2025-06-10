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

//! Pallet event conversions to the core ISMP event
use polkadot_sdk::*;

use crate::{Config, Event as PalletEvent, Pallet};
use frame_support::BoundedVec;
use ismp::{
	events::{StateCommitmentVetoed, StateMachineUpdated},
	router::{Request, Response},
};

impl<T: Config> TryFrom<PalletEvent<T>> for ismp::events::Event {
	type Error = ();

	fn try_from(event: PalletEvent<T>) -> Result<Self, Self::Error> {
		match event {
			PalletEvent::StateMachineUpdated { state_machine_id, latest_height } =>
				Ok(ismp::events::Event::StateMachineUpdated(StateMachineUpdated {
					state_machine_id,
					latest_height,
				})),
			PalletEvent::Response { commitment, .. } => {
				let event = match Pallet::<T>::response(commitment).ok_or_else(|| ())? {
					Response::Post(response) => ismp::events::Event::PostResponse(response),
					Response::Get(response) => ismp::events::Event::GetResponse(response),
				};

				Ok(event)
			},
			PalletEvent::Request { commitment, .. } => {
				let event = match Pallet::<T>::request(commitment).ok_or_else(|| ())? {
					Request::Post(post) => ismp::events::Event::PostRequest(post),
					Request::Get(get) => ismp::events::Event::GetRequest(get),
				};

				Ok(event)
			},
			PalletEvent::GetRequestTimeoutHandled(handled) =>
				Ok(ismp::events::Event::GetRequestTimeoutHandled(handled)),
			PalletEvent::GetRequestHandled(handled) =>
				Ok(ismp::events::Event::GetRequestHandled(handled)),
			PalletEvent::PostRequestHandled(handled) =>
				Ok(ismp::events::Event::PostRequestHandled(handled)),
			PalletEvent::PostResponseHandled(handled) =>
				Ok(ismp::events::Event::PostResponseHandled(handled)),
			PalletEvent::PostRequestTimeoutHandled(handled) =>
				Ok(ismp::events::Event::PostRequestTimeoutHandled(handled)),
			PalletEvent::PostResponseTimeoutHandled(handled) =>
				Ok(ismp::events::Event::PostResponseTimeoutHandled(handled)),
			PalletEvent::StateCommitmentVetoed { fisherman, height } =>
				Ok(ismp::events::Event::StateCommitmentVetoed(StateCommitmentVetoed {
					fisherman: fisherman.into_inner(),
					height,
				})),
			// We are only converting events useful relayers and applications
			PalletEvent::ConsensusClientCreated { .. } |
			PalletEvent::ConsensusClientFrozen { .. } |
			PalletEvent::Errors { .. } |
			PalletEvent::__Ignore(_, _) => Err(()),
		}
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
			ismp::events::Event::GetRequest(_) |
			ismp::events::Event::GetResponse(_) => {
				unimplemented!("These events should not originate from handler")
			},
		}
	}
}
