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

use crate::types::*;
use alloc::vec::Vec;
use frame_support::{
	dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo},
	pallet_prelude::*,
	traits::Get,
};
use frame_system::pallet_prelude::*;
use ismp::dispatcher::IsmpDispatcher;
use ismp::host::{IsmpHost, StateMachine};
use polkadot_sdk::{
	sp_runtime::{traits::OpaqueKeys, KeyTypeId},
	*,
};

mod impls;
pub mod types;

pub use pallet::*;

pub const MODULE_ID: &'static [u8] = b"ISMP-RLYR";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::PalletId;
	use polkadot_sdk::sp_core::H256;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config
		+ pallet_ismp::Config
		+ pallet_ismp_host_executive::Config
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;

		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance> + Default;

		/// The account id for the treasury
		type TreasuryAccount: Get<PalletId>;

		/// Origin for privileged actions
		type IncentivesOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Price oracle for usd price conversion to bridge tokens
		type PriceOracle: PriceOracle<Self::Balance>;

		/// The target message size
		#[pallet::constant]
		type TargetMessageSize: Get<u32>;

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
	StorageMap<_, Twox64Concat, (StateMachine, StateMachine), bool, OptionQuery>;

	/// A map of request commitment to fees associated with them
	#[pallet::storage]
	#[pallet::getter(fn commitment_fees)]
	pub type CommitmentFees<T: Config> = StorageMap<_, Blake2_128Concat, H256, T::Balance, OptionQuery>;

	/// Accumulated fees for a relayer
	#[pallet::storage]
	#[pallet::getter(fn accumulated_fees)]
	pub type AccumulatedFees<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StateMachine,
		Blake2_128Concat,
		T::AccountId,
		T::Balance,
		ValueQuery,
	>;

	/// Latest nonce for each address and the state machine they want to withdraw from
	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	pub type Nonce<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		StateMachine, // source Chain
		u64,
		ValueQuery,
	>;


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
		ErrorCompletingCall
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// State machine Routes supported for incentives
		RouteSupported { source_chain: StateMachine, destination_chain: StateMachine },
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
		/// Resetting of Incentives has occurred
		IncentivesReset,
		FeeAccumulated {
			relayer: T::AccountId,
			source_chain: StateMachine,
			amount: <T as pallet_ismp::Config>::Balance,
		},
		Withdrawn {
			relayer: T::AccountId,
			source_chain: StateMachine,
			amount: <T as pallet_ismp::Config>::Balance,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		u128: From<<T as pallet_ismp::Config>::Balance>,
		T::AccountId: AsRef<[u8]>,{
		/// Whitelists a route for messaging fees.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_supported_route())]
		pub fn set_supported_route(
			origin: OriginFor<T>,
			source: StateMachine,
			destination: StateMachine,
		) -> DispatchResult {
			<T as Config>::IncentivesOrigin::ensure_origin(origin)?;

			IncentivizedRoutes::<T>::insert((source, destination), true);

			Self::deposit_event(Event::<T>::RouteSupported {
				source_chain: source,
				destination_chain: destination,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::withdraw_fees())]
		pub fn withdraw_fees(
			origin: OriginFor<T>,
			source_chain: StateMachine
		) -> DispatchResult {
			let relayer = ensure_signed(origin)?;
			Self::do_withdraw_fees(relayer, source_chain)?;
			Ok(())
		}
	}
}

impl<T: Config> pallet_session::SessionHandler<T::AccountId> for Pallet<T> {
	const KEY_TYPE_IDS: &'static [KeyTypeId] = &[];

	fn on_genesis_session<Ks: OpaqueKeys>(_validators: &[(T::AccountId, Ks)]) {}

	fn on_new_session<Ks: OpaqueKeys>(
		changed: bool,
		_validators: &[(T::AccountId, Ks)],
		_queued_validators: &[(T::AccountId, Ks)],
	) {
		if !changed {
			return;
		}

		TotalBytesProcessed::<T>::kill();

		Self::deposit_event(Event::IncentivesReset);
	}

	fn on_disabled(_validator_index: u32) {}
}
