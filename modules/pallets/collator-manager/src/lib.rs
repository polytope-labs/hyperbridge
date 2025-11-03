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
//! It uses a reputation score held in `pallet-assets` to rank and select collators for each new
//! session.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
extern crate alloc;

use pallet_messaging_fees::IncentivesManager;
use polkadot_sdk::{sp_runtime::Weight, *};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use codec::{Codec, HasCompact};
	use core::fmt::Debug;
	use frame_support::{
		PalletId,
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{
			Currency, ExistenceRequirement, Get, LockIdentifier, LockableCurrency,
			ReservableCurrency, SignedImbalance, WithdrawReasons,
			fungible::{self, Inspect, Mutate},
			tokens::{Fortitude, Precision, Preservation},
		},
	};
	use frame_system::pallet_prelude::{BlockNumberFor, OriginFor};
	use pallet_session::SessionManager;
	use scale_info::TypeInfo;
	use sp_runtime::{
		DispatchError, FixedPointOperand,
		traits::{AccountIdConversion, AtLeast32BitUnsigned, Saturating, Zero},
	};
	use sp_staking::SessionIndex;
	use sp_std::vec::Vec;

	/// Positive imbalance type of the wrapped `NativeCurrency`.
	type PositiveImbalanceOf<T> = <<T as Config>::NativeCurrency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::PositiveImbalance;
	/// Negative imbalance type of the wrapped `NativeCurrency`.
	type NegativeImbalanceOf<T> = <<T as Config>::NativeCurrency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::NegativeImbalance;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config
		+ pallet_session::Config
		+ pallet_collator_selection::Config<
			ValidatorId = <Self as pallet_session::Config>::ValidatorId,
		> + pallet_authorship::Config
		+ pallet_ismp::Config
	{
		/// The pallet-assets instance that manages the reputation token.
		type ReputationAsset: fungible::Mutate<Self::AccountId, Balance = <Self as pallet::Config>::Balance>;

		/// The Native balance type
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Codec
			+ HasCompact<Type: DecodeWithMemTracking>
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Debug
			+ MaxEncodedLen
			+ TypeInfo
			+ FixedPointOperand;

		/// This is meant to `pallet-balances` which is the underlying native currency pallet that
		/// this pallet wraps around.
		type NativeCurrency: ReservableCurrency<Self::AccountId, Balance = <Self as pallet::Config>::Balance>
			+ LockableCurrency<Self::AccountId, Balance = <Self as pallet::Config>::Balance>;

		/// The PalletId of the Treasury pallet
		type TreasuryAccount: Get<PalletId>;

		/// Admin origin for privileged actions
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The identifier for the locks placed by this pallet.
		#[pallet::constant]
		type LockId: Get<LockIdentifier>;

		/// Trait implementation for resetting messaging incentives.
		type IncentivesManager: pallet_messaging_fees::IncentivesManager;

		/// Weight information for operations
		type WeightInfo: WeightInfo;
	}

	/// Tracks the total amount an account has bonded through this pallet.
	#[pallet::storage]
	pub type Bonded<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, <T as pallet::Config>::Balance, ValueQuery>;

	/// The reward value for collators
	#[pallet::storage]
	#[pallet::getter(fn collator_reward)]
	pub type CollatorReward<T: Config> =
		StorageValue<_, <T as pallet::Config>::Balance, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The account does not have enough unreserved funds to bond the requested amount.
		InsufficientBalance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new set of collators has been selected for the upcoming session.
		NewCollatorSet(Vec<T::AccountId>),
		/// The reputation score/balance of a collator has been reset.
		ReputationReset {
			/// The account of the collator whose reputation was reset.
			who: T::AccountId,
			/// The amount of reputation that was reset.
			amount: <T as pallet::Config>::Balance,
		},
		/// The collator reward amount has been updated.
		CollatorRewardAmountUpdated {
			/// The new reward amount
			new_reward: <T as pallet::Config>::Balance,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Sets the collator reward amount.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_collator_reward())]
		pub fn set_collator_reward(
			origin: OriginFor<T>,
			new_reward: <T as pallet::Config>::Balance,
		) -> DispatchResult {
			<T as pallet::Config>::AdminOrigin::ensure_origin(origin)?;
			CollatorReward::<T>::put(new_reward);
			Self::deposit_event(Event::CollatorRewardAmountUpdated { new_reward });
			Ok(())
		}
	}

	impl<T: Config> pallet_authorship::EventHandler<T::AccountId, BlockNumberFor<T>> for Pallet<T> {
		fn note_author(author: T::AccountId) {
			let reward = CollatorReward::<T>::get();

			if reward > Zero::zero() {
				let treasury_account = T::TreasuryAccount::get().into_account_truncating();

				let _ = T::NativeCurrency::transfer(
					&treasury_account,
					&author,
					reward,
					frame_support::traits::ExistenceRequirement::KeepAlive,
				);
			}
		}
	}

	impl<T: Config> SessionManager<T::AccountId> for Pallet<T>
	where
		<T as pallet_session::Config>::ValidatorId: Into<T::AccountId> + Clone,
		T::AccountId: From<<T as pallet_session::Config>::ValidatorId>,
		T::AccountId: Into<<T as pallet_session::Config>::ValidatorId> + Clone,
		<T as pallet_session::Config>::ValidatorId: From<T::AccountId>,
	{
		fn new_session(_new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
			T::IncentivesManager::reset_incentives();

			let active_collators = <pallet_session::Pallet<T>>::validators();
			let desired_collators = core::cmp::max(
				pallet_collator_selection::DesiredCandidates::<T>::get(),
				<T as pallet_collator_selection::Config>::MinEligibleCollators::get(),
			) as usize;

			let mut new_set_validators: Vec<<T as pallet_session::Config>::ValidatorId> =
				Vec::new();

			// select from registered candidates who are not in the current active set
			// with session keys and highes balances.
			let mut candidates = pallet_collator_selection::CandidateList::<T>::get()
				.into_iter()
				.map(|info| info.who)
				.filter(|c| {
					!active_collators.contains(&c.clone().into()) &&
						pallet_session::NextKeys::<T>::get(c.clone().into()).is_some()
				})
				.filter_map(|account_id| {
					let balance = T::ReputationAsset::balance(&account_id);
					if balance.is_zero() { None } else { Some((balance, account_id)) }
				})
				.collect::<Vec<_>>();

			candidates.sort_by_key(|(balance, _)| *balance);

			new_set_validators.extend(
				candidates
					.into_iter()
					.rev()
					.take(desired_collators)
					.map(|(_, c)| c.clone().into()),
			);

			// fill remaining slots with the best of the previous set.
			if new_set_validators.len() < desired_collators {
				let needed = desired_collators - new_set_validators.len();
				let mut reused_collators = active_collators.clone();

				reused_collators.sort_by_key(|a| {
					let account_id: T::AccountId = a.clone().into();
					T::ReputationAsset::balance(&account_id)
				});
				new_set_validators.extend(reused_collators.into_iter().rev().take(needed));
			}

			let new_set: Vec<T::AccountId> =
				new_set_validators.iter().map(|v| v.clone().into()).collect();
			if new_set.is_empty() {
				return None;
			}

			for account_id in &new_set {
				let balance = T::ReputationAsset::balance(&account_id);
				let result = T::ReputationAsset::burn_from(
					&account_id,
					balance,
					Preservation::Expendable,
					Precision::Exact,
					Fortitude::Polite,
				);

				if result.is_ok() {
					Self::deposit_event(Event::ReputationReset {
						who: account_id.clone(),
						amount: balance,
					});
				}
			}

			Self::deposit_event(Event::NewCollatorSet(new_set.clone()));
			Some(new_set)
		}

		fn end_session(_: SessionIndex) {}

		fn start_session(_: SessionIndex) {}
	}

	/// The custom implementation of the `ReservableCurrency` trait.
	impl<T: Config> ReservableCurrency<T::AccountId> for Pallet<T> {
		/// Checks total amount of funds that are not already reserved
		/// total includes locked/vested tokens
		fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
			let unreserved_balance = T::NativeCurrency::total_balance(who)
				.saturating_sub(T::NativeCurrency::reserved_balance(who));
			unreserved_balance >= value
		}

		/// Reserves by placing a lock on the `NativeCurrency`.
		/// Checks if the funds are available via `can_reserve`. If so, it updates the
		/// `Bonded` storage and then places a lock on `pallet-balances` for the new total
		/// bonded amount.
		fn reserve(who: &T::AccountId, value: <T as pallet::Config>::Balance) -> DispatchResult {
			ensure!(Self::can_reserve(who, value), Error::<T>::InsufficientBalance);

			let new_total_bonded = Bonded::<T>::mutate(who, |bonded| {
				*bonded = bonded.saturating_add(value);
				*bonded
			});

			T::NativeCurrency::set_lock(
				T::LockId::get(),
				who,
				new_total_bonded,
				WithdrawReasons::all(),
			);

			Ok(())
		}

		/// Reverses the reserve by updating or removing the lock.
		/// It reduces the amount in its internal `Bonded` ledger and then updates the lock on
		/// `pallet-balances` to match. If the bonded amount becomes zero, the lock is removed
		/// entirely.
		fn unreserve(
			who: &T::AccountId,
			value: <T as pallet::Config>::Balance,
		) -> <T as pallet::Config>::Balance {
			let unreserved_amount = Bonded::<T>::mutate(who, |bonded| {
				let to_unreserve = (*bonded).min(value);
				*bonded = bonded.saturating_sub(to_unreserve);
				to_unreserve
			});

			let new_total_bonded = Bonded::<T>::get(who);
			if new_total_bonded.is_zero() {
				T::NativeCurrency::remove_lock(T::LockId::get(), who);
			} else {
				T::NativeCurrency::set_lock(
					T::LockId::get(),
					who,
					new_total_bonded,
					WithdrawReasons::all(),
				);
			}

			unreserved_amount
		}

		fn reserved_balance(who: &T::AccountId) -> <T as pallet::Config>::Balance {
			Bonded::<T>::get(who)
		}

		fn slash_reserved(
			who: &T::AccountId,
			value: <T as pallet::Config>::Balance,
		) -> (NegativeImbalanceOf<T>, <T as pallet::Config>::Balance) {
			let amount = Bonded::<T>::mutate(who, |bonded| {
				let to_slash = (*bonded).min(value);
				*bonded = bonded.saturating_sub(to_slash);
				to_slash
			});

			T::NativeCurrency::set_lock(T::LockId::get(), who, amount, WithdrawReasons::all());

			let (imbalance, remainder) = T::NativeCurrency::slash(who, value);
			(imbalance, remainder)
		}

		fn repatriate_reserved(
			slashed: &T::AccountId,
			beneficiary: &T::AccountId,
			value: <T as pallet::Config>::Balance,
			status: frame_support::traits::BalanceStatus,
		) -> Result<<T as pallet::Config>::Balance, DispatchError> {
			T::NativeCurrency::repatriate_reserved(slashed, beneficiary, value, status)
		}
	}

	/// Makes use of the underlying implementation provided by the `NativeCurrency` i.e
	/// pallet-balances
	impl<T: Config> Currency<T::AccountId> for Pallet<T> {
		type Balance = <T as pallet::Config>::Balance;
		type PositiveImbalance = PositiveImbalanceOf<T>;
		type NegativeImbalance = NegativeImbalanceOf<T>;

		fn total_balance(who: &T::AccountId) -> Self::Balance {
			T::NativeCurrency::total_balance(who)
		}

		fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
			T::NativeCurrency::can_slash(who, value)
		}

		fn total_issuance() -> Self::Balance {
			T::NativeCurrency::total_issuance()
		}

		fn minimum_balance() -> Self::Balance {
			T::NativeCurrency::minimum_balance()
		}

		fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
			T::NativeCurrency::burn(amount)
		}

		fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
			T::NativeCurrency::issue(amount)
		}

		fn free_balance(who: &T::AccountId) -> Self::Balance {
			T::NativeCurrency::free_balance(who)
		}

		fn slash(
			who: &T::AccountId,
			value: Self::Balance,
		) -> (Self::NegativeImbalance, Self::Balance) {
			T::NativeCurrency::slash(who, value)
		}

		fn transfer(
			source: &T::AccountId,
			dest: &T::AccountId,
			value: Self::Balance,
			existence_requirement: frame_support::traits::ExistenceRequirement,
		) -> DispatchResult {
			T::NativeCurrency::transfer(source, dest, value, existence_requirement)
		}

		fn ensure_can_withdraw(
			who: &T::AccountId,
			amount: Self::Balance,
			reasons: WithdrawReasons,
			new_balance: Self::Balance,
		) -> DispatchResult {
			T::NativeCurrency::ensure_can_withdraw(who, amount, reasons, new_balance)
		}

		fn deposit_into_existing(
			who: &T::AccountId,
			value: Self::Balance,
		) -> Result<Self::PositiveImbalance, DispatchError> {
			T::NativeCurrency::deposit_into_existing(who, value)
		}

		fn deposit_creating(who: &T::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
			T::NativeCurrency::deposit_creating(who, value)
		}

		fn withdraw(
			who: &T::AccountId,
			value: Self::Balance,
			reasons: WithdrawReasons,
			liveness: ExistenceRequirement,
		) -> Result<Self::NegativeImbalance, DispatchError> {
			T::NativeCurrency::withdraw(who, value, reasons, liveness)
		}

		fn make_free_balance_be(
			who: &T::AccountId,
			balance: Self::Balance,
		) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
			T::NativeCurrency::make_free_balance_be(who, balance)
		}
	}
}

/// Weight information for pallet operations
pub trait WeightInfo {
	/// sets collator reward
	fn set_collator_reward() -> Weight;
}

/// Default weight implementation using sensible defaults
impl WeightInfo for () {
	fn set_collator_reward() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
}
