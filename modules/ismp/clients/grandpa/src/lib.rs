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
use ismp::consensus::ConsensusStateId;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::{
		consensus::StateMachineId,
		host::{IsmpHost, StateMachine},
	};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_ismp::Config {
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type IsmpHost: IsmpHost + Default;
	}

	/// Events emitted by this pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Parachains with the `para_ids` have been added to the whitelist
		ParachainsAdded {
			/// The parachains in question
			para_ids: Vec<(u32, u64)>,
		},
		/// Parachains with the `para_ids` have been removed from the whitelist
		ParachainsRemoved {
			/// The parachains in question
			para_ids: Vec<u32>,
		},
		/// Standalone Chain Added to whitelist
		StandaloneChainsAdded {
			/// The state machines in question
			state_machines: Vec<StateMachine>,
		},
		/// Standalone have been removed from the whitelist
		StandaloneChainsRemoved {
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
		StorageMap<_, Twox64Concat, StateMachine, bool, OptionQuery>;

	/// List of parachains that this state machine is interested in.
	#[pallet::storage]
	pub type Parachains<T: Config> = StorageMap<_, Identity, u32, u64>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add some a state machine to the list of supported state machines
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn add_state_machines(
			origin: OriginFor<T>,
			state_machines: Vec<StateMachine>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			for state_machine in state_machines {
				SupportedStateMachines::<T>::insert(state_machine, true)
			}

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

			Self::deposit_event(Event::StandaloneChainsRemoved { state_machines });

			Ok(())
		}

		/// Add some new parachains to the parachains whitelist
		#[pallet::call_index(2)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(data.parachains.len() as u64))]
		pub fn add_parachains(origin: OriginFor<T>, data: ParachainData) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let host = <T::IsmpHost>::default();
			for (para, slot_duration) in &data.parachains {
				let state_id = match <T as pallet_ismp::Config>::Coprocessor::get() {
					Some(StateMachine::Kusama(_)) => StateMachine::Kusama(*para),
					Some(StateMachine::Polkadot(_)) => StateMachine::Polkadot(*para),
					_ => continue,
				};
				Parachains::<T>::insert(*para, slot_duration);
				let _ = host.store_challenge_period(
					StateMachineId { state_id, consensus_state_id: data.consensus_state_id },
					data.challenge_period,
				);
			}

			Self::deposit_event(Event::ParachainsAdded { para_ids: data.parachains });

			Ok(())
		}

		/// Removes some parachains from the parachains whitelist
		#[pallet::call_index(3)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(para_ids.len() as u64))]
		pub fn remove_parachains(origin: OriginFor<T>, para_ids: Vec<u32>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			for id in &para_ids {
				Parachains::<T>::remove(id);
			}

			Self::deposit_event(Event::ParachainsRemoved { para_ids });

			Ok(())
		}
	}
}

/// Update the parachain whitelist
#[derive(Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, Debug, PartialEq, Eq)]
pub struct ParachainData {
	/// Consensus state id for the parachains
	pub consensus_state_id: ConsensusStateId,
	/// A list of parachain ids and slot duration
	pub parachains: Vec<(u32, u64)>,
	/// Challenge period for the parachains
	pub challenge_period: u64,
}
