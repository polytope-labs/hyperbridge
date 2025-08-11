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

//! The pallet-collator-manager is a session manager for selecting collators based on reputation.
//! It uses a reputation score held in `pallet-assets` to rank and select collators for each new session.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
extern crate alloc;

use polkadot_sdk::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::traits::{
		Get,
		fungibles::{self, Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
	};
	use pallet_session::SessionManager;
	use sp_runtime::traits::Zero;
	use sp_staking::SessionIndex;
	use sp_std::vec::Vec;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config
		+ pallet_session::Config
		+ pallet_collator_selection::Config<
			ValidatorId = <Self as pallet_session::Config>::ValidatorId,
		> + pallet_ismp::Config
	{
		/// A constant that provides the ID of the asset used for reputation.
		#[pallet::constant]
		type ReputationAssetId: Get<
			<Self::ReputationAssets as fungibles::Inspect<Self::AccountId>>::AssetId,
		>;
		/// The pallet-assets instance that manages the reputation token.
		type ReputationAssets: fungibles::Mutate<Self::AccountId, Balance = <Self as pallet_ismp::Config>::Balance>;
		/// A constant that defines the target number of collators for the active set.
		#[pallet::constant]
		type DesiredCollators: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new set of collators has been selected for the upcoming session.
		NewCollatorSet(Vec<T::AccountId>),
		/// The reputation score/balance of a collator has been reset.
		ReputationReset(T::AccountId),
	}

	impl<T: Config> SessionManager<T::AccountId> for Pallet<T>
	where
		<T as pallet_session::Config>::ValidatorId: Into<T::AccountId> + Clone,
		T::AccountId: From<<T as pallet_session::Config>::ValidatorId>,
		T::AccountId: Into<<T as pallet_session::Config>::ValidatorId> + Clone,
		<T as pallet_session::Config>::ValidatorId: From<T::AccountId>,
	{
		fn new_session(_new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
			let active_collators = <pallet_session::Pallet<T>>::validators();
			let desired_collators = T::DesiredCollators::get() as usize;

			let mut new_set_validators: Vec<<T as pallet_session::Config>::ValidatorId> =
				Vec::new();

			// select from registered candidates who are not in the current active set
			// with session keys and highes balances.
			let mut candidates = pallet_collator_selection::CandidateList::<T>::get()
				.into_iter()
				.map(|info| info.who.into())
				.collect::<Vec<_>>();
			candidates.retain(|c| {
				!active_collators.contains(c) && pallet_session::NextKeys::<T>::get(&c).is_some()
			});
			candidates.sort_by_key(|a| {
				let account_id: T::AccountId = a.clone().into();
				T::ReputationAssets::balance(T::ReputationAssetId::get(), &account_id)
			});
			candidates.reverse();
			new_set_validators.extend(candidates.into_iter().take(desired_collators));

			// fill remaining slots with the best of the previous set.
			if new_set_validators.len() < desired_collators {
				let needed = desired_collators - new_set_validators.len();
				let mut reused_collators = active_collators.clone();

				reused_collators.sort_by_key(|a| {
					let account_id: T::AccountId = a.clone().into();
					T::ReputationAssets::balance(T::ReputationAssetId::get(), &account_id)
				});
				reused_collators.reverse();

				new_set_validators.extend(reused_collators.into_iter().take(needed));
			}

			let new_set: Vec<T::AccountId> =
				new_set_validators.iter().map(|v| v.clone().into()).collect();
			if new_set.is_empty() {
				return None;
			}

			let newly_joined_collators: Vec<<T as pallet_session::Config>::ValidatorId> =
				new_set_validators
					.iter()
					.filter(|c| !active_collators.contains(c))
					.cloned()
					.collect();

			for new_collator in newly_joined_collators {
				let account_id: T::AccountId = new_collator.into();
				let balance =
					T::ReputationAssets::balance(T::ReputationAssetId::get(), &account_id);
				if !balance.is_zero() {
					let result = T::ReputationAssets::burn_from(
						T::ReputationAssetId::get(),
						&account_id,
						balance,
						Preservation::Expendable,
						Precision::Exact,
						Fortitude::Polite,
					);

					if result.is_ok() {
						Self::deposit_event(Event::ReputationReset(account_id));
					}
				}
			}

			Self::deposit_event(Event::NewCollatorSet(new_set.clone()));
			Some(new_set)
		}

		fn end_session(_: SessionIndex) {}

		fn start_session(_: SessionIndex) {}
	}
}
