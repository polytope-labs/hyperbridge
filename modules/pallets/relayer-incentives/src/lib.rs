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

//! The pallet-relayer-incentives allows for incentivizing relayers with rewards.
//!
//! This pallet implements the FeeHandler trait from pallet-ismp to process messages
//! and reward relayers who deliver them.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use frame_support::dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use ismp::host::IsmpHost;
use polkadot_sdk::*;

mod impls;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::PalletId;
	use ismp::consensus::StateMachineId;
	use polkadot_sdk::sp_core::H256;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;

		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + Default;

		/// The account id for the treasury
		type TreasuryAccount: Get<PalletId>;

		/// Weight information for operations
		type WeightInfo: WeightInfo;
	}

	/// Mapping from relayer to their total rewards
	#[pallet::storage]
	#[pallet::getter(fn relayer_rewards)]
	pub type RelayerRewards<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		<T as pallet_ismp::Config>::Balance,
		ValueQuery,
	>;

	/// Message identifiers (e.g., request_id/response_id) that have already been rewarded
	#[pallet::storage]
	#[pallet::getter(fn processed_messages)]
	pub type ProcessedMessages<T: Config> = StorageMap<_, Blake2_128Concat, H256, bool, ValueQuery>;

	// Mapping from state machineId to respective cost per block
	#[pallet::storage]
	#[pallet::getter(fn state_machines_cost_per_block)]
	pub type StateMachinesCostPerBlock<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		StateMachineId,
		<T as pallet_ismp::Config>::Balance,
		ValueQuery,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// Reward transfer
		RewardTransferFailed,
		/// Invalid address
		InvalidAddress,
		/// Message has already been processed for rewards
		MessageAlreadyProcessed,
		/// Could not get State machine height
		CouldNotGetStateMachineHeight,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A relayer was rewarded
		RelayerRewarded {
			/// Relayer account that received the reward
			relayer: T::AccountId,
			/// Amount of the reward
			amount: <T as pallet_ismp::Config>::Balance,
			/// Message identifier that was processed
			message_id: H256,
		},
		/// State Machine cost per block updated
		StateMachineCostPerBlockUpdated {
			/// Number of messages processed
			state_machine_id: StateMachineId,
			/// Cost per block
			cost_per_block: <T as pallet_ismp::Config>::Balance,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Update cost per block for a state machine
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::update_cost_per_block())]
		pub fn update_cost_per_block(
			origin: OriginFor<T>,
			state_machine_id: StateMachineId,
			cost_per_block: <T as pallet_ismp::Config>::Balance,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			StateMachinesCostPerBlock::<T>::mutate(state_machine_id.clone(), |block_cost| {
				*block_cost = cost_per_block;
			});

			Self::deposit_event(Event::<T>::StateMachineCostPerBlockUpdated {
				state_machine_id,
				cost_per_block,
			});

			Ok(())
		}
	}
}

/// Weight information for pallet operations
pub trait WeightInfo {
	fn update_cost_per_block() -> Weight;
}

/// Default weight implementation using sensible defaults
impl WeightInfo for () {
	fn update_cost_per_block() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
}
