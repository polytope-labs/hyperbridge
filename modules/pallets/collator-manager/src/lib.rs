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

use pallet_messaging_incentives::IncentivesManager;
use polkadot_sdk::{sp_runtime::Weight, *};

// `storage_alias` for the legacy ledger generates an undocumented prefix struct, which trips
// the crate-wide `deny(missing_docs)`; relax it for this one module.
#[allow(missing_docs)]
pub mod migrations;

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
			Currency, Get, LockableCurrency, ReservableCurrency, ValidatorRegistration,
			fungible::{self, Inspect, Mutate},
			tokens::{Fortitude, Precision, Preservation},
		},
	};
	use frame_system::{
		ensure_signed,
		pallet_prelude::{BlockNumberFor, OriginFor},
	};
	use pallet_session::SessionManager;
	use scale_info::TypeInfo;
	use sp_runtime::{
		FixedPointOperand,
		traits::{AccountIdConversion, AtLeast32BitUnsigned, Zero},
	};
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

		/// The native currency (pallet-balances). Used to pay collator rewards, and by the
		/// one-off migration that moves legacy bond locks into collator-selection reserves.
		type NativeCurrency: ReservableCurrency<Self::AccountId, Balance = <Self as pallet::Config>::Balance>
			+ LockableCurrency<Self::AccountId, Balance = <Self as pallet::Config>::Balance>;

		/// The PalletId of the Treasury pallet
		type TreasuryAccount: Get<PalletId>;

		/// Admin origin for privileged actions
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Trait implementation for resetting messaging incentives.
		type IncentivesManager: pallet_messaging_incentives::IncentivesManager;

		/// Weight information for operations
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	pub type Controller<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

	#[pallet::storage]
	pub type Stash<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

	/// Pending controller-side approvals authorising a specific stash to bind
	/// the controller. A non-empty entry at `(stash, controller)` is the
	/// controller's signed consent to be paired by that stash.
	///
	/// Cleared on consumption by `register` / `set_controller`, or explicitly
	/// via `revoke_controller_approval`.
	#[pallet::storage]
	pub type ControllerApprovals<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId, // stash
		Blake2_128Concat,
		T::AccountId, // controller
		(),
		OptionQuery,
	>;

	/// The reward value for collators
	#[pallet::storage]
	#[pallet::getter(fn collator_reward)]
	pub type CollatorReward<T: Config> =
		StorageValue<_, <T as pallet::Config>::Balance, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The specified account is not a stash account.
		AlreadyRegistered,
		/// The stash account has no bond associated with it.
		NoBond,
		/// The specified account is not a stash account.
		NotStash,
		/// The specified account is not a controller account.
		NoController,
		/// The specified controller account is already paired with another stash.
		AlreadyPaired,
		/// The controller has not approved being paired with the calling stash.
		ControllerApprovalMissing,
		/// There is no pending controller approval to revoke for the given pair.
		NoPendingApproval,
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
		/// A new stash account has been registered with a controller.
		Registered {
			/// The stash account that was registered.
			stash: T::AccountId,
			/// The controller account set for the stash.
			controller: T::AccountId,
		},
		/// A stash account has been deregistered, removing it's controller.
		Deregistered {
			/// The stash account that was deregistered/
			stash: T::AccountId,
		},
		/// A stash account has changed it's controller account.
		ControllerSet {
			/// The stash account that was affected.
			stash: T::AccountId,
			/// The old controller account.
			old_controller: T::AccountId,
			/// The new controller account.
			new_controller: T::AccountId,
		},
		/// A collator was rewarded for authoring a block.
		CollatorRewarded {
			/// The collator who authored the block.
			collator: T::AccountId,
			/// The reward amount.
			amount: <T as pallet::Config>::Balance,
		},
		/// A controller account has approved being paired with a specific stash.
		ControllerApprovalGranted {
			/// The controller account granting approval.
			controller: T::AccountId,
			/// The stash account the controller has authorised.
			stash: T::AccountId,
		},
		/// A previously-granted controller approval was revoked.
		ControllerApprovalRevoked {
			/// The controller account that revoked approval.
			controller: T::AccountId,
			/// The stash account whose approval was revoked.
			stash: T::AccountId,
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

		/// Registers a controller account for a bonded stash.
		///
		/// The origin must be a stash account, which must have already bonded funds
		/// via `pallet-collator-selection`. The supplied `controller` must have
		/// previously authorised the pairing by calling `approve_controller` from
		/// the controller's own origin — without this two-step consent, an
		/// arbitrary stash could squat any unpaired controller address, blocking
		/// the legitimate operator and (if the controller carried session keys
		/// and reputation) consuming that reputation on selection.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register())]
		pub fn register(origin: OriginFor<T>, controller: T::AccountId) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			ensure!(!Controller::<T>::contains_key(&stash), Error::<T>::AlreadyRegistered);
			ensure!(!Stash::<T>::contains_key(&controller), Error::<T>::AlreadyPaired);
			// Controller must have signed an approval for this specific stash.
			ensure!(
				ControllerApprovals::<T>::take(&stash, &controller).is_some(),
				Error::<T>::ControllerApprovalMissing
			);

			Controller::<T>::insert(&stash, &controller);
			Stash::<T>::insert(&controller, &stash);

			Self::deposit_event(Event::Registered { stash, controller });
			Ok(())
		}

		/// Change the controller account for a registered stash.
		///
		/// The origin must be the stash account, and the proposed `new_controller`
		/// must have previously authorised the rotation by calling
		/// `approve_controller` from its own origin (mirroring the consent flow
		/// required by `register`).
		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_controller())]
		pub fn set_controller(
			origin: OriginFor<T>,
			new_controller: T::AccountId,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			let old_controller = Controller::<T>::get(&stash).ok_or(Error::<T>::NotStash)?;
			ensure!(!Stash::<T>::contains_key(&new_controller), Error::<T>::AlreadyPaired);
			ensure!(
				ControllerApprovals::<T>::take(&stash, &new_controller).is_some(),
				Error::<T>::ControllerApprovalMissing
			);

			Controller::<T>::insert(&stash, &new_controller);
			Stash::<T>::remove(&old_controller);
			Stash::<T>::insert(&new_controller, &stash);

			Self::deposit_event(Event::ControllerSet { stash, old_controller, new_controller });
			Ok(())
		}

		/// Deregister a stash account, unbinding it's controller.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::deregister())]
		pub fn deregister(origin: OriginFor<T>) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			let controller = Controller::<T>::take(&stash).ok_or(Error::<T>::NotStash)?;
			Stash::<T>::remove(&controller);
			Self::deposit_event(Event::Deregistered { stash });
			Ok(())
		}

		/// Authorise `stash` to bind the caller as its controller.
		///
		/// The origin is the controller account granting consent. A subsequent
		/// `register(controller)` or `set_controller(controller)` call from
		/// `stash` consumes this approval and completes the pairing. The
		/// approval is single-use and per-(stash, controller).
		///
		/// Approvals may be retracted before consumption via
		/// `revoke_controller_approval`.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register())]
		pub fn approve_controller(origin: OriginFor<T>, stash: T::AccountId) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			ControllerApprovals::<T>::insert(&stash, &controller, ());
			Self::deposit_event(Event::ControllerApprovalGranted { controller, stash });
			Ok(())
		}

		/// Revoke a previously-granted controller approval for the given stash.
		/// The origin must be the controller that issued the approval.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::deregister())]
		pub fn revoke_controller_approval(
			origin: OriginFor<T>,
			stash: T::AccountId,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			ensure!(
				ControllerApprovals::<T>::take(&stash, &controller).is_some(),
				Error::<T>::NoPendingApproval,
			);
			Self::deposit_event(Event::ControllerApprovalRevoked { controller, stash });
			Ok(())
		}
	}

	impl<T: Config> pallet_authorship::EventHandler<T::AccountId, BlockNumberFor<T>> for Pallet<T> {
		fn note_author(author: T::AccountId) {
			let reward = CollatorReward::<T>::get();

			if reward > Zero::zero() {
				let treasury_account = T::TreasuryAccount::get().into_account_truncating();

				let result = T::NativeCurrency::transfer(
					&treasury_account,
					&author,
					reward,
					frame_support::traits::ExistenceRequirement::KeepAlive,
				);

				if result.is_ok() {
					Self::deposit_event(Event::CollatorRewarded {
						collator: author,
						amount: reward,
					});
				}
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

			let desired_collators = core::cmp::max(
				pallet_collator_selection::DesiredCandidates::<T>::get(),
				<T as pallet_collator_selection::Config>::MinEligibleCollators::get(),
			) as usize;

			// select registered candidates that have session keys, ranked by reputation.
			let mut candidates = pallet_collator_selection::CandidateList::<T>::get()
				.into_iter()
				.map(|info| info.who)
				.filter_map(|stash_account| Controller::<T>::get(&stash_account))
				.filter(|controller_account| {
					pallet_session::NextKeys::<T>::get(controller_account.clone().into()).is_some()
				})
				.filter_map(|controller_account| {
					let balance = T::ReputationAsset::balance(&controller_account);
					if balance.is_zero() { None } else { Some((balance, controller_account)) }
				})
				.collect::<Vec<_>>();

			candidates.sort_by_key(|(balance, _)| *balance);

			let new_set_validators: Vec<<T as pallet_session::Config>::ValidatorId> = candidates
				.into_iter()
				.rev()
				.take(desired_collators)
				.map(|(_, c)| c.clone().into())
				.collect();

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

	/// Implementation of `ValidatorRegistration` that checks if a stash account
	/// has a registered controller with valid session keys.
	impl<T: Config> ValidatorRegistration<T::AccountId> for Pallet<T>
	where
		T::AccountId: Into<<T as pallet_session::Config>::ValidatorId> + Clone,
	{
		fn is_registered(stash: &T::AccountId) -> bool {
			if let Some(controller) = Controller::<T>::get(stash) {
				let validator_id: <T as pallet_session::Config>::ValidatorId = controller.into();
				pallet_session::NextKeys::<T>::get(&validator_id).is_some()
			} else {
				false
			}
		}
	}
}

/// Weight information for pallet operations
pub trait WeightInfo {
	/// sets collator reward
	fn set_collator_reward() -> Weight;
	/// registers a controller account
	fn register() -> Weight;
	/// change the controller account for a stash
	fn set_controller() -> Weight;
	/// deregisters a stash account, unbinding it's controller.
	fn deregister() -> Weight;
}

/// Default weight implementation using sensible defaults
impl WeightInfo for () {
	fn set_collator_reward() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
	fn register() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
	fn set_controller() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
	fn deregister() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
}
