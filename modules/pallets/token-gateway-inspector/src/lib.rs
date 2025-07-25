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

//! The token governor handles asset registration as well as tracks the metadata of multi-chain
//! native tokens across all connected chains
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloy_sol_types::SolValue;
use ismp::router::PostRequest;
use polkadot_sdk::*;

use alloc::{format, vec};
use primitive_types::{H256, U256};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, Blake2_128Concat};

	use ismp::host::StateMachine;
	use pallet_token_gateway::types::Body;
	use pallet_token_governor::StandaloneChainAssets;
	use polkadot_sdk::{
		frame_support::dispatch::DispatchResult, frame_system::pallet_prelude::OriginFor,
	};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The pallet's configuration trait.
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config + pallet_ismp::Config + pallet_token_governor::Config
	{
		/// Origin for privileged actions
		type GatewayOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	/// Balances for net inflow of non native assets into a standalone chain
	#[pallet::storage]
	pub type InflowBalances<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, StateMachine, Twox64Concat, H256, U256, ValueQuery>;

	/// Pallet events that functions in this pallet can emit.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Illegal request has been intercepted
		IllegalRequest { source: StateMachine },
	}

	/// Errors that can be returned by this pallet.
	#[pallet::error]
	pub enum Error<T> {}

	// Hack for implementing the [`Default`] bound needed for
	// [`IsmpDispatcher`](ismp::dispatcher::IsmpDispatcher) and
	// [`IsmpModule`](ismp::module::IsmpModule)
	impl<T> Default for Pallet<T> {
		fn default() -> Self {
			Self(PhantomData)
		}
	}

	#[derive(
		Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
	)]
	pub struct NetInflow {
		asset: H256,
		chain: StateMachine,
		balance: U256,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the balance of a chain through a governance action
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(1))]
		pub fn set_chain_balance(origin: OriginFor<T>, inflow: NetInflow) -> DispatchResult {
			T::GatewayOrigin::ensure_origin(origin)?;
			InflowBalances::<T>::insert(inflow.chain, inflow.asset, inflow.balance);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn is_token_gateway_request(body: &[u8]) -> Option<Body> {
			Body::abi_decode(&mut &body[1..], true).ok()
		}

		pub fn inspect_request(post: &PostRequest) -> Result<(), ismp::Error> {
			let PostRequest { body, source, dest, .. } = post.clone();

			// Token Gateway contracts on EVM chains are immutable and non upgradeable
			// As long as the initial deployment is valid
			// it's impossible to send malicious requests
			if source.is_evm() && dest.is_evm() {
				return Ok(());
			}

			if let Some(body) = Self::is_token_gateway_request(&body) {
				// There's no need to record when the destination is EVM because we don't perform
				// the balance check when the source is EVM
				if !dest.is_evm() {
					InflowBalances::<T>::try_mutate(dest, H256::from(body.asset_id.0), |val| {
						let amount = U256::from_big_endian(&body.amount.to_be_bytes::<32>());
						*val += amount;
						Ok::<_, ()>(())
					})
					.map_err(|_| {
						ismp::Error::Custom(format!(
							"Failed to record inflow while inspecting packet"
						))
					})?;
				}

				let is_native =
					StandaloneChainAssets::<T>::get(source, H256::from(body.asset_id.0))
						.unwrap_or_default();
				// We don't check when the source is EVM because the contract issuing the request
				// cannot be malicious, And if there's a consensus fault, it will be caught by
				// fishermen during the challenge period
				if !is_native && !source.is_evm() {
					let balance = InflowBalances::<T>::get(source, H256::from(body.asset_id.0));
					let amount = U256::from_big_endian(&body.amount.to_be_bytes::<32>());
					if amount > balance {
						Err(ismp::Error::Custom(format!("Illegal Token Gateway request")))?;
						Pallet::<T>::deposit_event(Event::<T>::IllegalRequest { source })
					}

					InflowBalances::<T>::try_mutate(source, H256::from(body.asset_id.0), |val| {
						*val -= amount;
						Ok::<_, ()>(())
					})
					.map_err(|_| {
						ismp::Error::Custom(format!(
							"Failed to record inflow while inspecting packet"
						))
					})?;
				}
			}

			Ok(())
		}

		pub fn handle_timeout(post: &PostRequest) -> Result<(), ismp::Error> {
			let PostRequest { body, source, dest, .. } = post.clone();
			// Token Gateway contracts on EVM chains are immutable and non upgradeable
			// As long as the initial deployment is valid
			// it's impossible to send malicious requests
			if source.is_evm() && dest.is_evm() {
				return Ok(());
			}

			if let Some(body) = Self::is_token_gateway_request(&body) {
				let is_native =
					StandaloneChainAssets::<T>::get(source, H256::from(body.asset_id.0))
						.unwrap_or_default();
				if !is_native && !source.is_evm() {
					InflowBalances::<T>::try_mutate(source, H256::from(body.asset_id.0), |val| {
						let amount = U256::from_big_endian(&body.amount.to_be_bytes::<32>());
						*val += amount;
						Ok::<_, ()>(())
					})
					.map_err(|_| {
						ismp::Error::Custom(format!(
							"Failed to record inflow while inspecting packet"
						))
					})?;
				}

				if !dest.is_evm() {
					InflowBalances::<T>::try_mutate(dest, H256::from(body.asset_id.0), |val| {
						let amount = U256::from_big_endian(&body.amount.to_be_bytes::<32>());
						*val -= amount;
						Ok::<_, ()>(())
					})
					.map_err(|_| {
						ismp::Error::Custom(format!(
							"Failed to record inflow while inspecting packet"
						))
					})?;
				}
			}
			Ok(())
		}
	}
}
