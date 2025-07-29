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
// See the License for the specific lang

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub mod consensus;
pub mod messages;
pub mod weights;
use codec::DecodeWithMemTracking;
use polkadot_sdk::*;

use ismp::host::StateMachine;
pub use pallet::*;
pub use weights::WeightInfo;

/// Update the state machine whitelist
#[derive(
	Clone,
	codec::Encode,
	codec::Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	Debug,
	PartialEq,
	Eq,
)]
pub struct AddStateMachine {
	/// State machine to add
	pub state_machine: StateMachine,
	/// It's slot duration
	pub slot_duration: u64,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec::Vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::host::IsmpHost;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// IsmpHost implementation
		type IsmpHost: IsmpHost + Default;

		/// Weight information for dispatchable extrinsics
		type WeightInfo: WeightInfo;

		/// Origin for privileged actions
		type RootOrigin: EnsureOrigin<Self::RuntimeOrigin>;
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

	/// Registered state machines for the grandpa consensus client
	#[pallet::storage]
	#[pallet::getter(fn state_machines)]
	pub type SupportedStateMachines<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, u64, OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add some a state machine to the list of supported state machines
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::add_state_machines(new_state_machines.len() as u32))]
		pub fn add_state_machines(
			origin: OriginFor<T>,
			new_state_machines: Vec<AddStateMachine>,
		) -> DispatchResult {
			T::RootOrigin::ensure_origin(origin)?;

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
		#[pallet::weight(T::WeightInfo::remove_state_machines(state_machines.len() as u32))]
		pub fn remove_state_machines(
			origin: OriginFor<T>,
			state_machines: Vec<StateMachine>,
		) -> DispatchResult {
			T::RootOrigin::ensure_origin(origin)?;

			for state_machine in state_machines.clone() {
				SupportedStateMachines::<T>::remove(state_machine)
			}

			Self::deposit_event(Event::StateMachineRemoved { state_machines });

			Ok(())
		}
	}
}
