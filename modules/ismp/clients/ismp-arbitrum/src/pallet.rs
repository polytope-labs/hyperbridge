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

	// Mapping from state machineId to respective roll up core addresses
	#[pallet::storage]
	#[pallet::getter(fn state_machines_rollup_core_addresses)]
	pub type StateMachinesRollupCoreAddresses<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachineId, H160, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn supported_state_machines)]
	pub type SupportedStateMachines<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, bool, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// State Machine Rollup Core Address updated
		StateMachinesRollupCoreAddress {
			/// State Machine Identifier for the chain
			state_machine_id: StateMachineId,
			/// The address for the rollup core
			rollup_core_address: H160,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Sets the new roll up core address
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
		pub fn set_rollup_core_address(
			origin: OriginFor<T>,
			state_machine_id: StateMachineId,
			rollup_core_address: H160,
		) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;

			StateMachinesRollupCoreAddresses::<T>::mutate(
				state_machine_id.clone(),
				|maybe_address| {
					*maybe_address = Some(rollup_core_address);
				},
			);

			SupportedStateMachines::<T>::insert(state_machine_id.state_id, true);

			Self::deposit_event(Event::<T>::StateMachinesRollupCoreAddress {
				state_machine_id,
				rollup_core_address,
			});

			Ok(())
		}
	}
}
