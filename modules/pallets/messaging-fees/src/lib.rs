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
	traits::{Get, OneSessionHandler},
};
use frame_system::pallet_prelude::*;
use ismp::host::{IsmpHost, StateMachine};
use polkadot_sdk::*;

mod impls;
pub mod types;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::PalletId;

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
		type IsmpHost: IsmpHost + Default;

		/// The account id for the treasury
		type TreasuryAccount: Get<PalletId>;

		/// Origin for privileged actions
		type IncentivesOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Price oracle for price conversion to bridge tokens
		type PriceOracle: PriceOracle<Self::Balance>;

		/// The epoch length to reward messages in blocks
		#[pallet::constant]
		type EpochLength: Get<BlockNumberFor<Self>>;

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
	pub type IncentivizedRoutes<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		StateMachine,
		Twox64Concat,
		StateMachine,
		bool,
		OptionQuery,
	>;

	/// Current active Epoch for incentivization
	#[pallet::storage]
	#[pallet::getter(fn epoch)]
	pub type Epoch<T: Config> = StorageValue<_, EpochInfo<BlockNumberFor<T>>, ValueQuery>;

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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// State machine Routes supported for incentives
		RouteSupported { source_chain: StateMachine, destination_chain: StateMachine },
		/// A relayer was rewarded
		RelayerRewarded {
			/// Relayer account that received the reward
			relayer: T::AccountId,
			/// Amount of the reward
			amount: <T as pallet_ismp::Config>::Balance,
		},
		/// A relayer was charged a fee
		RelayerCharged {
			/// Relayer account that was charged
			relayer: T::AccountId,
			/// Amount of the fee
			amount: <T as pallet_ismp::Config>::Balance,
		},
		/// A new epoch has started
		NewEpoch {
			/// The index of the new epoch
			index: u64,
		},
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			let mut epoch = Epoch::<T>::get();
			if n - epoch.start_block >= T::EpochLength::get() {
				epoch.index += 1;
				epoch.start_block = n;
				Epoch::<T>::put(epoch.clone());
				TotalBytesProcessed::<T>::kill();
				Self::deposit_event(Event::NewEpoch { index: epoch.index });
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Whitelists a route for messaging incentives.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_supported_route())]
		pub fn set_supported_route(
			origin: OriginFor<T>,
			source: StateMachine,
			destination: StateMachine,
		) -> DispatchResult {
			<T as Config>::IncentivesOrigin::ensure_origin(origin)?;

			IncentivizedRoutes::<T>::insert(source, destination, true);

			Self::deposit_event(Event::<T>::RouteSupported {
				source_chain: source,
				destination_chain: destination,
			});

			Ok(())
		}
	}
}
