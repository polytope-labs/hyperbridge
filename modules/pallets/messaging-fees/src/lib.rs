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

//! The pallet-messaging-relayer-incentives allows for incentivizing messaging relayers with
//! rewards.
//!
//! This pallet implements the FeeHandler trait from pallet-ismp to process messages
//! and reward messaging relayers who deliver request and response messages.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use frame_support::{
	dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo},
	pallet_prelude::*,
	traits::Get,
};
use frame_system::pallet_prelude::*;
use polkadot_sdk::*;

use ismp::{
	dispatcher::IsmpDispatcher,
	host::{IsmpHost, StateMachine},
};
pub use pallet::*;

use crate::types::*;

mod impls;
pub mod types;

/// A trait for managing messaging incentives, primarily for resetting them.
pub trait IncentivesManager {
	/// Resets any accumulated incentive data, called at the start of a new session.
	fn reset_incentives();
}

#[frame_support::pallet]
pub mod pallet {
	use crate::frame_support::traits::fungible;
	use frame_support::PalletId;
	use polkadot_sdk::sp_core::H256;

	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config
		+ pallet_ismp::Config
		+ pallet_ismp_host_executive::Config
		+ pallet_ismp_relayer::Config
	{
		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost
			+ IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>
			+ Default;

		/// The account id for the treasury
		type TreasuryAccount: Get<PalletId>;

		/// Origin for privileged actions
		type IncentivesOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Price oracle for usd price conversion to bridge tokens
		type PriceOracle: PriceOracle;

		/// The pallet-assets instance used for managing the reputation token.
		type ReputationAsset: fungible::Mutate<
			Self::AccountId,
			Balance = <Self as pallet_ismp::Config>::Balance,
		>;
		/// Weight information for operations
		type WeightInfo: WeightInfo;
	}

	/// Total bytes processed in the current epoch for all chains
	#[pallet::storage]
	#[pallet::getter(fn total_bytes_processed)]
	pub type TotalBytesProcessed<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Stores whitelisted routes for incentives. The key is a tuple of (source, destination).
	#[pallet::storage]
	#[pallet::getter(fn incentivized_routes)]
	pub type IncentivizedRoutes<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, bool, OptionQuery>;

	/// A map of request commitment to fees associated with them
	#[pallet::storage]
	#[pallet::getter(fn commitment_fees)]
	pub type CommitmentFees<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, T::Balance, OptionQuery>;

	/// Stores the Target Message Size value
	#[pallet::storage]
	#[pallet::getter(fn target_message_size)]
	pub type TargetMessageSize<T: Config> = StorageValue<_, u32, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Reward transfer failed
		RewardTransferFailed,
		/// Calculation overflow
		CalculationOverflow,
		/// Oracle Price conversion error
		ErrorInPriceConversion,
		/// State machine per byte fee not found
		PerByteFeeNotFound,
		/// Not enough balance for withdrawal
		NotEnoughBalance,
		/// Dispactch request failed
		DispatchFailed,
		/// Error
		ErrorCompletingCall,
		/// Reputation mint failed
		ReputationMintFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// State machine Routes supported for incentives
		RouteSupported { state_machine: StateMachine },
		/// A relayer was rewarded
		FeeRewarded {
			/// Relayer account that received the reward
			relayer: T::AccountId,
			/// Amount of the reward
			amount: <T as pallet_ismp::Config>::Balance,
		},
		/// A relayer was charged a fee
		FeePaid {
			/// Relayer account that was charged
			relayer: T::AccountId,
			/// Amount of the fee
			amount: <T as pallet_ismp::Config>::Balance,
		},
		/// Target Message Size updated
		TargetMessageSizeUpdated { new_size: u32 },
		/// Resetting of Incentives has occurred
		IncentivesReset,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		u128: From<<T as pallet_ismp::Config>::Balance>,
		T::AccountId: AsRef<[u8]>,
	{
		/// Whitelists a route for messaging fees.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_supported_route())]
		pub fn set_supported_route(
			origin: OriginFor<T>,
			state_machine: StateMachine,
		) -> DispatchResult {
			<T as Config>::IncentivesOrigin::ensure_origin(origin)?;

			IncentivizedRoutes::<T>::insert(&state_machine, true);

			Self::deposit_event(Event::<T>::RouteSupported { state_machine });

			Ok(())
		}

		/// Sets the Target Message Size
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_target_message_size())]
		pub fn set_target_message_size(origin: OriginFor<T>, new_size: u32) -> DispatchResult {
			<T as Config>::IncentivesOrigin::ensure_origin(origin)?;
			TargetMessageSize::<T>::put(new_size);
			Self::deposit_event(Event::<T>::TargetMessageSizeUpdated { new_size });
			Ok(())
		}
	}
}

impl<T: Config> IncentivesManager for Pallet<T> {
	fn reset_incentives() {
		TotalBytesProcessed::<T>::kill();
		Self::deposit_event(Event::IncentivesReset);
	}
}
