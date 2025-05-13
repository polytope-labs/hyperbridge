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

use alloc::{collections::BTreeMap, vec::Vec};
use codec::{Decode, Encode};
use frame_support::dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo};
use ismp::messaging::Message;
use pallet_ismp::fee_handler::FeeHandler;
use polkadot_sdk::*;
use scale_info::TypeInfo;

mod impls;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Currency type for balance operations
		type Currency: frame_support::traits::Currency<Self::AccountId>;

		/// Origin that can manage incentive parameters
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Conversion from a relayer address (Vec<u8>) to AccountId
		type RelayerLookup: RelayerAccountLookup<Self::AccountId>;

		/// Weight information for operations
		type WeightInfo: WeightInfo;
	}

	/// Relayer incentive distribution parameters
	#[pallet::storage]
	#[pallet::getter(fn incentive_params)]
	pub type IncentiveParams<T: Config> = StorageValue<_, IncentiveParameters, ValueQuery>;

	/// Mapping from relayer to their total rewards
	#[pallet::storage]
	#[pallet::getter(fn relayer_rewards)]
	pub type RelayerRewards<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	/// Message identifiers (e.g., request_id/response_id) that have already been rewarded
	#[pallet::storage]
	#[pallet::getter(fn processed_messages)]
	pub type ProcessedMessages<T: Config> =
		StorageMap<_, Blake2_128Concat, Vec<u8>, bool, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient funds for operation
		InsufficientFunds,
		/// Invalid parameter value
		InvalidParameter,
		/// Operation not allowed for this relayer
		NotAuthorized,
		/// Relayer account could not be resolved
		RelayerLookupFailed,
		/// Message has already been processed for rewards
		MessageAlreadyProcessed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Incentive parameters were updated
		IncentiveParametersUpdated {
			/// New parameters
			parameters: IncentiveParameters,
		},
		/// A relayer was rewarded
		RelayerRewarded {
			/// Relayer account that received the reward
			relayer: T::AccountId,
			/// Amount of the reward
			amount: BalanceOf<T>,
			/// Message identifier that was processed
			message_id: Vec<u8>,
		},
		/// A relayer claimed their rewards
		RewardsClaimed {
			/// Relayer account that claimed rewards
			relayer: T::AccountId,
			/// Amount claimed
			amount: BalanceOf<T>,
		},
		/// Batch of messages processed for rewards
		BatchProcessed {
			/// Number of messages processed
			count: u32,
			/// Total rewards distributed
			total_rewards: BalanceOf<T>,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Update incentive parameters
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::update_incentive_parameters())]
		pub fn update_incentive_parameters(
			origin: OriginFor<T>,
			parameters: IncentiveParameters,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			// Validate parameters
			ensure!(parameters.is_valid(), Error::<T>::InvalidParameter);

			// Update storage
			IncentiveParams::<T>::put(parameters.clone());

			// Emit event
			Self::deposit_event(Event::<T>::IncentiveParametersUpdated { parameters });

			Ok(())
		}
	}
}

/// Trait for converting between relayer addresses (Vec<u8>) and AccountId
pub trait RelayerAccountLookup<AccountId> {
	/// Convert a relayer address to an AccountId
	fn lookup_account(address: &[u8]) -> Option<AccountId>;
}

/// Type alias for currency balances
pub type BalanceOf<T> = <<T as Config>::Currency as frame_support::traits::Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

/// Parameters for the incentive mechanism
#[derive(Clone, Encode, Decode, PartialEq, Eq, TypeInfo, Debug, Default)]
pub struct IncentiveParameters {
	/// Base reward for relaying messages
	pub base_reward: u128,
	/// Multiplier for high priority messages
	pub priority_multiplier: u32,
	/// Minimum time between reward claims
	pub claim_interval: u32,
}

impl IncentiveParameters {
	/// Check if parameters are valid
	pub fn is_valid(&self) -> bool {
		self.priority_multiplier > 0 && self.claim_interval > 0
	}
}

/// Weight information for pallet operations
pub trait WeightInfo {
	fn update_incentive_parameters() -> Weight;
	fn reward_relayer() -> Weight;
	fn claim_rewards() -> Weight;
	fn on_message_execution() -> Weight;
}

/// Default weight implementation using sensible defaults
impl WeightInfo for () {
	fn update_incentive_parameters() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}

	fn reward_relayer() -> Weight {
		Weight::from_parts(20_000_000, 0)
	}

	fn claim_rewards() -> Weight {
		Weight::from_parts(50_000_000, 0)
	}

	fn on_message_execution() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
}
