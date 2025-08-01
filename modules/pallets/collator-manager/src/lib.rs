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

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
pub mod currency_adapter;

use polkadot_sdk::*;

use alloc::vec::Vec;
pub use pallet::*;

pub trait CandidateProvider<ValidatorId> {
	fn candidates() -> Vec<ValidatorId>;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		traits::{
			fungibles,
			Currency, Get,
		},
	};
	use sp_runtime::traits::Zero;
	use sp_staking::SessionIndex;
	use sp_std::vec::Vec;
	use pallet_session::SessionManager;
	use polkadot_sdk::frame_support::traits::fungibles::Mutate;


	type BalanceOf<T> =
	<<T as Config>::ReputationCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_session::Config {
		type ReputationCurrency: Currency<Self::AccountId>;
		type CandidateProvider: CandidateProvider<Self::ValidatorId>;
		#[pallet::constant]
		type ReputationAssetId: Get<<Self::ReputationAssets as fungibles::Inspect<Self::AccountId>>::AssetId>;
		type ReputationAssets: fungibles::Mutate<
			Self::AccountId,
			Balance = BalanceOf<Self>,
		>;
		#[pallet::constant]
		type DesiredCollators: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewCollatorSet(Vec<T::AccountId>),
		ReputationReset(T::AccountId),
	}

	impl<T: Config> SessionManager<T::AccountId> for Pallet<T>
	where T::ValidatorId: Into<T::AccountId> + Clone,
		  T::AccountId: From<T::ValidatorId>,
		  T::AccountId: Into<T::ValidatorId> + Clone,
		  T::ValidatorId: From<T::AccountId>
	{
		fn new_session(_new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
			println!("new session");
			let active_collators = <pallet_session::Pallet<T>>::validators();
			println!("active collators are {:?}", active_collators.len());
			let desired_collators = T::DesiredCollators::get() as usize;

			let mut new_set_validators: Vec<T::ValidatorId> = Vec::new();

			// select from registered candidates who are not in the current active set
			// with session keys and highes balances.
			let mut candidates = T::CandidateProvider::candidates();
			println!("candidates are {:?}", candidates.len());
			candidates.retain(|c| {
				!active_collators.contains(c) &&
					pallet_session::NextKeys::<T>::get(&c).is_some()
			});
			candidates.sort_by_key(|a| {
				let account_id: T::AccountId = a.clone().into();
				T::ReputationCurrency::total_balance(&account_id)
			});
			candidates.reverse();
			new_set_validators.extend(candidates.into_iter().take(desired_collators));
			println!("new_set_validators are {:?}", new_set_validators.len());

			// fill remaining slots with the best of the previous set.
			if new_set_validators.len() < desired_collators {
				let needed = desired_collators - new_set_validators.len();
				let mut reused_collators = active_collators.clone();

				reused_collators.retain(|c| !new_set_validators.contains(c));

				reused_collators.sort_by_key(|a| {
					let account_id: T::AccountId = a.clone().into();
					T::ReputationCurrency::total_balance(&account_id)
				});
				reused_collators.reverse();

				new_set_validators.extend(reused_collators.into_iter().take(needed));
			}

			let new_set: Vec<T::AccountId> = new_set_validators.iter().map(|v| v.clone().into()).collect();
			println!("new_set are {:?}", new_set.len());
			if new_set.is_empty() {
				return None;
			}


			let outgoing_collators: Vec<T::ValidatorId> = active_collators
				.into_iter()
				.filter(|c| !new_set_validators.contains(c))
				.collect();

			println!("outgoing collators are {:?}", outgoing_collators.len());

			for old_collator in outgoing_collators {
				let account_id: T::AccountId = old_collator.clone().into();
				println!("setting balance to zero for {:?}", &account_id);
				T::ReputationAssets::set_balance(
					T::ReputationAssetId::get(),
					&account_id,
					Zero::zero(),
				);
				Self::deposit_event(Event::ReputationReset(account_id));
			}

			Self::deposit_event(Event::NewCollatorSet(new_set.clone()));
			Some(new_set)
		}

		fn end_session(_: SessionIndex) {}

		fn start_session(_: SessionIndex) {}
	}
}
