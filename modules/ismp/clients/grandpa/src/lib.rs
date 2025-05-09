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
use polkadot_sdk::*;

use ismp::{consensus::ConsensusStateId, host::StateMachine};
pub use pallet::*;
pub use weights::WeightInfo;

/// Update the state machine whitelist
#[derive(Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, Debug, PartialEq, Eq)]
pub struct StateMachineInfo {
	/// State machine to add
	pub state_machine: StateMachine,
	/// It's slot duration
	pub slot_duration: u64,
}

/// Update the state machine whitelist
#[derive(Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, Debug, PartialEq, Eq)]
pub enum AddStateMachines {
	Standard(StateMachineInfo),
	Alternative { consensus_state_id: ConsensusStateId, para_id: u32, info: StateMachineInfo },
}

impl AddStateMachines {
	fn state_machine(&self) -> StateMachine {
		match &self {
			Self::Standard(info) => info.state_machine.clone(),
			Self::Alternative { info, .. } => info.state_machine.clone(),
		}
	}
}

/// Update the state machine whitelist
#[derive(Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, Debug, PartialEq, Eq)]
pub enum RemoveStateMachines {
	Standard(StateMachine),
	Alternative { consensus_state_id: ConsensusStateId, para_id: u32 },
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec::Vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::host::IsmpHost;
	use polkadot_sdk::frame_support::Blake2_128Concat;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;

		/// IsmpHost implementation
		type IsmpHost: IsmpHost + Default;

		/// Weight information for dispatchable extrinsics
		type WeightInfo: WeightInfo;
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
			state_machines: Vec<RemoveStateMachines>,
		},
	}

	/// Registered state machines for the grandpa consensus client
	#[pallet::storage]
	#[pallet::getter(fn state_machines)]
	pub type SupportedStateMachines<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, u64, OptionQuery>;

	/// Registered state machines for the alternative relay chains
	#[pallet::storage]
	#[pallet::getter(fn alternative_solochains)]
	pub type AlternativeRelayChain<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ConsensusStateId,
		Blake2_128Concat,
		u32,
		StateMachineInfo,
		OptionQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add some a state machine to the list of supported state machines
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::add_state_machines(new_state_machines.len() as u32))]
		pub fn add_state_machines(
			origin: OriginFor<T>,
			new_state_machines: Vec<AddStateMachines>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			let state_machines = new_state_machines.iter().map(|a| a.state_machine()).collect();
			for add_state_machine in new_state_machines {
				match add_state_machine {
					AddStateMachines::Standard(StateMachineInfo {
						state_machine,
						slot_duration,
					}) => {
						SupportedStateMachines::<T>::insert(state_machine, slot_duration);
					},
					AddStateMachines::Alternative { consensus_state_id, para_id, info } => {
						AlternativeRelayChain::<T>::insert(
							consensus_state_id,
							para_id,
							info.clone(),
						);
						SupportedStateMachines::<T>::insert(info.state_machine, info.slot_duration);
					},
				}
			}

			Self::deposit_event(Event::StateMachineAdded { state_machines });

			Ok(())
		}

		/// Remove a state machine from the list of supported state machines
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::remove_state_machines(state_machines.len() as u32))]
		pub fn remove_state_machines(
			origin: OriginFor<T>,
			state_machines: Vec<RemoveStateMachines>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			for rm_state_machines in state_machines.clone() {
				match rm_state_machines {
					RemoveStateMachines::Standard(state_machine) => {
						SupportedStateMachines::<T>::remove(state_machine);
					},
					RemoveStateMachines::Alternative { consensus_state_id, para_id } => {
						let info = AlternativeRelayChain::<T>::get(consensus_state_id, para_id);
						AlternativeRelayChain::<T>::remove(consensus_state_id, para_id);
						if let Some(info) = info {
							SupportedStateMachines::<T>::remove(info.state_machine);
						}
					},
				}
			}

			Self::deposit_event(Event::StateMachineRemoved { state_machines });

			Ok(())
		}
	}
}
