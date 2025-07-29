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

//! Enables fishermen keep hyperbridge safe by vetoing fraudulent state commitments.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
pub use pallet::*;
use polkadot_sdk::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::{
		consensus::{StateCommitment, StateMachineHeight},
		events::StateCommitmentVetoed,
		host::IsmpHost,
	};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + Default;

		/// Origin for privileged actions
		type FishermenOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	/// Set of whitelisted fishermen accounts
	#[pallet::storage]
	#[pallet::getter(fn whitelist)]
	pub type Fishermen<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, (), OptionQuery>;

	/// Set of whitelisted fishermen accounts
	#[pallet::storage]
	#[pallet::getter(fn pending_vetoes)]
	pub type PendingVetoes<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachineHeight, T::AccountId, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Account Already Whitelisted
		AlreadyAdded,
		/// Account wasn't found in the set.
		NotInSet,
		/// An account not in the fishermen set attempted to execute a veto
		UnauthorizedAction,
		/// State commitment was not found
		VetoFailed,
		/// Invalid veto request
		InvalidVeto,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account `account` has been added to the fishermen set.
		Added { account: T::AccountId },
		/// An account `account` has been removed from the fishermen set.
		Removed { account: T::AccountId },
		/// The provided state commitment was vetoed `state_machine` is by account
		StateCommitmentVetoed { height: StateMachineHeight, commitment: StateCommitment },
		/// The vetoe has been noted by the runtime
		VetoNoted { height: StateMachineHeight, fisherman: T::AccountId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: AsRef<[u8]>,
	{
		/// Adds a new fisherman to the set
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn add(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			T::FishermenOrigin::ensure_origin(origin)?;

			ensure!(!Fishermen::<T>::contains_key(&account), Error::<T>::AlreadyAdded);
			Fishermen::<T>::insert(&account, ());

			Self::deposit_event(Event::Added { account });
			Ok(())
		}

		/// Removes a fisherman from the set
		#[pallet::call_index(1)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn remove(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			T::FishermenOrigin::ensure_origin(origin)?;

			ensure!(Fishermen::<T>::contains_key(&account), Error::<T>::NotInSet);
			Fishermen::<T>::remove(&account);

			Self::deposit_event(Event::Removed { account });
			Ok(())
		}

		/// A fisherman has determined that some [`StateCommitment`] (which is ideally still in it's
		/// challenge period) is infact fraudulent and misrepresentative of the state
		/// changes at the provided height. This allows them to veto the state commitment.
		/// They aren't required to provide any proofs for this.
		/// Successful veto requires two fishermen
		#[pallet::call_index(2)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(2, 3))]
		pub fn veto_state_commitment(
			origin: OriginFor<T>,
			height: StateMachineHeight,
		) -> DispatchResult {
			let account = ensure_signed(origin.clone())?;
			ensure!(Fishermen::<T>::contains_key(&account), Error::<T>::UnauthorizedAction);

			if let Some(prev_veto) = PendingVetoes::<T>::get(height) {
				if account == prev_veto {
					Err(Error::<T>::InvalidVeto)?
				}
				let ismp_host = <T as Config>::IsmpHost::default();
				let commitment = ismp_host
					.state_machine_commitment(height)
					.map_err(|_| Error::<T>::VetoFailed)?;
				ismp_host.delete_state_commitment(height).map_err(|_| Error::<T>::VetoFailed)?;
				PendingVetoes::<T>::remove(height);

				Self::deposit_event(Event::StateCommitmentVetoed { height, commitment });
				pallet_ismp::Pallet::<T>::deposit_event(
					ismp::events::Event::StateCommitmentVetoed(StateCommitmentVetoed {
						height,
						fisherman: account.as_ref().to_vec(),
					})
					.into(),
				);
			} else {
				PendingVetoes::<T>::insert(height, account.clone());
				Self::deposit_event(Event::VetoNoted { height, fisherman: account });
			}

			Ok(())
		}
	}
}
