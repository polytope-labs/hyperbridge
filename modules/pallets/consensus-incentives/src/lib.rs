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

//! The pallet-consensus-incentives allows for incentivizing relayers with rewards.
//!
//! This pallet implements the FeeHandler trait from pallet-ismp to process messages
//! and reward relayers who deliver them.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use frame_support::{
	dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo},
	pallet_prelude::*,
};
use frame_system::pallet_prelude::*;
use ismp::host::IsmpHost;
use polkadot_sdk::*;

mod impls;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{traits::fungible, PalletId};
	use ismp::consensus::{StateMachineHeight, StateMachineId};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + Default;

		/// The account id for the treasury
		type TreasuryAccount: Get<PalletId>;

		/// Weight information for operations
		type WeightInfo: WeightInfo;

		/// Origin for privileged actions
		type IncentivesOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The pallet-assets instance used for managing the reputation token.
		type ReputationAsset: fungible::Mutate<
			Self::AccountId,
			Balance = <Self as pallet_ismp::Config>::Balance,
		>;
	}

	// Mapping from state machineId to respective cost per block
	#[pallet::storage]
	#[pallet::getter(fn state_machines_cost_per_block)]
	pub type StateMachinesCostPerBlock<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		StateMachineId,
		<T as pallet_ismp::Config>::Balance,
		OptionQuery,
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
		/// Reputation mint failed
		ReputationMintFailed,
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
			///  Metadata about the state machine and height that was rewarded
			state_machine_height: StateMachineHeight,
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
			T::IncentivesOrigin::ensure_origin(origin)?;

			StateMachinesCostPerBlock::<T>::mutate(state_machine_id.clone(), |maybe_cost| {
				*maybe_cost = Some(cost_per_block);
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
