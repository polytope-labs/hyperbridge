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

//! The pallet-collator-balances is a wrapper around pallet-balances which allows intending collator
//! candidates to use their locked tokens(vesting tokens) as a candidacy bond for
//! pallet-collator-selection
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use polkadot_sdk::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use codec::{Codec, HasCompact};
	use core::fmt::Debug;
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{
			Currency, ExistenceRequirement, LockIdentifier, LockableCurrency, ReservableCurrency,
			SignedImbalance, WithdrawReasons,
		},
	};
	use scale_info::TypeInfo;
	use sp_runtime::{
		DispatchError, FixedPointOperand,
		traits::{AtLeast32BitUnsigned, Saturating, Zero},
	};

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
	pub trait Config: polkadot_sdk::frame_system::Config {
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
		type NativeCurrency: ReservableCurrency<Self::AccountId, Balance = Self::Balance>
			+ LockableCurrency<Self::AccountId, Balance = Self::Balance>;

		/// The identifier for the locks placed by this pallet.
		#[pallet::constant]
		type LockId: Get<LockIdentifier>;
	}

	/// Tracks the total amount an account has bonded through this pallet.
	#[pallet::storage]
	pub type Bonded<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::Balance, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The account does not have enough unreserved funds to bond the requested amount.
		InsufficientBalance,
	}

	/// Makes use of the underlying implementation provided by the `NativeCurrency` i.e
	/// pallet-balances
	impl<T: Config> Currency<T::AccountId> for Pallet<T> {
		type Balance = T::Balance;
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

	/// The custom implementation of the `ReservableCurrency` trait.
	impl<T: Config> ReservableCurrency<T::AccountId> for Pallet<T> {
		fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
			/// Checks total amount of funds that are not already reserved
			/// total includes locked/vested tokens
			let unreserved_balance = T::NativeCurrency::total_balance(who)
				.saturating_sub(T::NativeCurrency::reserved_balance(who));
			unreserved_balance >= value
		}

		/// Reserves by placing a lock on the `NativeCurrency`.
		/// Checks if the funds are available via `can_reserve`. If so, it updates the
		/// `Bonded` storage and then places a lock on `pallet-balances` for the new total
		/// bonded amount.
		fn reserve(who: &T::AccountId, value: T::Balance) -> DispatchResult {
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
		fn unreserve(who: &T::AccountId, value: T::Balance) -> T::Balance {
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

		fn reserved_balance(who: &T::AccountId) -> T::Balance {
			Bonded::<T>::get(who)
		}

		fn slash_reserved(
			who: &T::AccountId,
			value: T::Balance,
		) -> (NegativeImbalanceOf<T>, T::Balance) {
			Bonded::<T>::mutate(who, |bonded| {
				let to_slash = (*bonded).min(value);
				*bonded = bonded.saturating_sub(to_slash);
				to_slash
			});

			T::NativeCurrency::set_lock(
				T::LockId::get(),
				who,
				Bonded::<T>::get(who),
				WithdrawReasons::all(),
			);

			let (imbalance, remainder) = T::NativeCurrency::slash(who, value);
			(imbalance, remainder)
		}

		fn repatriate_reserved(
			slashed: &T::AccountId,
			beneficiary: &T::AccountId,
			value: T::Balance,
			status: frame_support::traits::BalanceStatus,
		) -> Result<T::Balance, DispatchError> {
			T::NativeCurrency::repatriate_reserved(slashed, beneficiary, value, status)
		}
	}
}
