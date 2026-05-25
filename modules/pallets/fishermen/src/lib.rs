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

//! Enables collators keep hyperbridge safe by vetoing fraudulent state
//! commitments. The set of accounts allowed to veto is the active collator
//! set, sourced from the runtime's `IsCollator` predicate.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
pub use extension::PrioritizeVeto;
pub use pallet::*;
use polkadot_sdk::*;

mod extension;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::Pays, pallet_prelude::*, traits::Contains};
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

		/// Predicate that returns true when an account is currently entitled to
		/// submit a veto. Runtimes wire this to the active collator set
		/// (e.g. session validators intersected with collator-manager
		/// controllers).
		type IsCollator: Contains<Self::AccountId>;
	}

	/// Heights with a single recorded veto, awaiting a second distinct collator
	/// to finalize the veto.
	#[pallet::storage]
	#[pallet::getter(fn pending_vetoes)]
	pub type PendingVetoes<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachineHeight, T::AccountId, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Caller is not in the active collator set.
		UnauthorizedAction,
		/// State commitment was not found.
		VetoFailed,
		/// Invalid veto request (e.g. same collator submitted twice).
		InvalidVeto,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The provided state commitment was vetoed at `height`.
		StateCommitmentVetoed { height: StateMachineHeight, commitment: StateCommitment },
		/// A first veto for `height` has been noted, awaiting a second distinct
		/// collator to finalize.
		VetoNoted { height: StateMachineHeight, fisherman: T::AccountId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: AsRef<[u8]>,
	{
		/// A collator has determined that some [`StateCommitment`] (which is ideally still in its
		/// challenge period) is in fact fraudulent and misrepresentative of the state changes at
		/// the provided height. This allows them to veto the state commitment. They aren't
		/// required to provide any proofs for this. Successful veto requires two distinct
		/// collators.
		///
		/// Dispatches with `Pays::No`. The on-chain `IsCollator` check is the DOS guard, so the
		/// signer does not need to hold a balance.
		#[pallet::call_index(0)]
		#[pallet::weight((<T as frame_system::Config>::DbWeight::get().reads_writes(2, 3), Pays::No))]
		pub fn veto_state_commitment(
			origin: OriginFor<T>,
			height: StateMachineHeight,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			ensure!(T::IsCollator::contains(&account), Error::<T>::UnauthorizedAction);

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
