// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Pallet for configuring supported Tendermint state machines

pub use pallet::*;
use polkadot_sdk::*;

extern crate alloc;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::host::StateMachine;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// Origin allowed for admin privileges
		type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::storage]
	#[pallet::getter(fn supported_state_machines)]
	pub type SupportedStateMachines<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, bool, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// State Machine support toggled
		StateMachineSupportUpdated {
			/// The Tendermint `StateMachine` whose support flag changed
			state_machine: StateMachine,
			/// Whether this `state_machine` is now supported
			supported: bool,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a Tendermint state machine support entry
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
		pub fn set_supported_state_machine(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			supported: bool,
		) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;

			SupportedStateMachines::<T>::insert(state_machine, supported);
			Self::deposit_event(Event::<T>::StateMachineSupportUpdated {
				state_machine,
				supported,
			});
			Ok(())
		}

		/// Remove a Tendermint state machine support entry
		#[pallet::call_index(1)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
		pub fn remove_supported_state_machine(
			origin: OriginFor<T>,
			state_machine: StateMachine,
		) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;

			SupportedStateMachines::<T>::remove(state_machine);
			Self::deposit_event(Event::<T>::StateMachineSupportUpdated {
				state_machine,
				supported: false,
			});
			Ok(())
		}
	}
}
