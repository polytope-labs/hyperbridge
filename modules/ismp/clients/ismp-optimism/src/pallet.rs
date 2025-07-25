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

pub use pallet::*;
use polkadot_sdk::*;

extern crate alloc;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec::Vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::{
		consensus::StateMachineId,
		host::{IsmpHost, StateMachine},
	};
	use primitive_types::H160;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// Origin allowed for admin privileges
		type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// IsmpHost implementation
		type IsmpHost: IsmpHost + Default;
	}

	#[pallet::error]
	pub enum Error<T> {}

	// Mapping from state machineId to respective oracle addresses
	#[pallet::storage]
	#[pallet::getter(fn state_machines_oracle_addresses)]
	pub type StateMachinesOracleAddresses<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachineId, H160, OptionQuery>;

	// Mapping from state machineId to respective dispute game addresses and respected game types
	#[pallet::storage]
	#[pallet::getter(fn state_machines_dispute_game_factories_types)]
	pub type StateMachinesDisputeGameFactoriesTypes<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachineId, (H160, Vec<u32>), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn supported_state_machines)]
	pub type SupportedStateMachines<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, bool, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// State Machine Oracle Address updated
		StateMachinesOracleAddress { state_machine_id: StateMachineId, oracle_address: H160 },
		/// State Machine Dispute Game Factory Types updated
		StateMachinesDisputeGameFactoryTypes {
			/// State Machine Identifier for the chain
			state_machine_id: StateMachineId,
			/// The dispute game factory contract address
			dispute_game_factory: H160,
			/// The respected dispute game types
			respected_game_types: Vec<u32>,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Sets the new oracle address
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
		pub fn set_oracle_address(
			origin: OriginFor<T>,
			state_machine_id: StateMachineId,
			oracle_address: H160,
		) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;

			StateMachinesOracleAddresses::<T>::mutate(state_machine_id.clone(), |maybe_address| {
				*maybe_address = Some(oracle_address);
			});

			SupportedStateMachines::<T>::insert(state_machine_id.state_id, true);

			Self::deposit_event(Event::<T>::StateMachinesOracleAddress {
				state_machine_id,
				oracle_address,
			});

			Ok(())
		}

		/// Sets the new dispute game factory with respected game types for a state machine
		#[pallet::call_index(2)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
		pub fn set_dispute_game_factories(
			origin: OriginFor<T>,
			state_machine_id: StateMachineId,
			dispute_game_factory: H160,
			respected_game_types: Vec<u32>,
		) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;

			StateMachinesDisputeGameFactoriesTypes::<T>::mutate(state_machine_id, |maybe_entry| {
				if let Some((factory, game_types)) = maybe_entry {
					*factory = dispute_game_factory;
					*game_types = respected_game_types.clone();
				} else {
					*maybe_entry = Some((dispute_game_factory, respected_game_types.clone()));
				}
			});

			SupportedStateMachines::<T>::insert(state_machine_id.state_id, true);

			Self::deposit_event(Event::<T>::StateMachinesDisputeGameFactoryTypes {
				state_machine_id,
				dispute_game_factory,
				respected_game_types,
			});

			Ok(())
		}
	}
}
