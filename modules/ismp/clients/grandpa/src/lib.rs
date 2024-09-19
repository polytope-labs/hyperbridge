// Copyright (c) 2024 Polytope Labs.
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
// See the License for the specific lang

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod consensus;
pub mod messages;

use alloc::vec::Vec;
use ismp::host::StateMachine;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::host::IsmpHost;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_ismp::Config {
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// IsmpHost implementation
		type IsmpHost: IsmpHost + Default;
	}

	/// Events emitted by this pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// State machines have been added to whitelist
		StateMachineAdded {
			/// The state machines in question
			state_machines: Vec<StateMachine>,
		},
		/// State machines have been removed from the whitelist
		StateMachineRemoved {
			/// The state machines in question
			state_machines: Vec<StateMachine>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Standalone Consensus State Already Exists
		StandaloneConsensusStateAlreadyExists,
		/// Standalone Consensus Does not Exist
		StandaloneConsensusStateDontExists,
		/// Error fetching consensus state
		ErrorFetchingConsensusState,
		/// Error decoding consensus state
		ErrorDecodingConsensusState,
		/// Incorrect consensus state id length
		IncorrectConsensusStateIdLength,
		/// Error storing consensus state
		ErrorStoringConsensusState,
	}

	/// Registered Standalone chains for the grandpa consensus client
	#[pallet::storage]
	#[pallet::getter(fn state_machines)]
	pub type SupportedStateMachines<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, u64, OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add some a state machine to the list of supported state machines
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn add_state_machines(
			origin: OriginFor<T>,
			new_state_machines: Vec<AddStateMachine>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			let state_machines =
				new_state_machines.iter().map(|a| a.state_machine.clone()).collect();
			for AddStateMachine { state_machine, slot_duration } in new_state_machines {
				SupportedStateMachines::<T>::insert(state_machine, slot_duration);
			}

			Self::deposit_event(Event::StateMachineAdded { state_machines });

			Ok(())
		}

		/// Remove a state machine from the list of supported state machines
		#[pallet::call_index(1)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn remove_state_machines(
			origin: OriginFor<T>,
			state_machines: Vec<StateMachine>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			for state_machine in state_machines.clone() {
				SupportedStateMachines::<T>::remove(state_machine)
			}

			Self::deposit_event(Event::StateMachineRemoved { state_machines });

			Ok(())
		}
	}
}

/// Update the state machine whitelist
#[derive(Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, Debug, PartialEq, Eq)]
pub struct AddStateMachine {
	/// State machine to add
	pub state_machine: StateMachine,
	/// It's slot duration
	pub slot_duration: u64,
}
