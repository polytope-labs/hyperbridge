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
use frame_support::pallet_prelude::Weight;
use ismp::router::PostRequest;

use alloc::{format, string::ToString, vec, vec::Vec};
use primitive_types::{H256, U256};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::collections::{BTreeMap, BTreeSet};
	use frame_support::{pallet_prelude::*, Blake2_128Concat};
	use frame_system::pallet_prelude::*;
	use ismp::{events::Meta, host::StateMachine};
	use pallet_token_gateway::Body;
	use pallet_token_governor::TokenGatewayParams;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The pallet's configuration trait.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_ismp::Config + pallet_token_governor::Config
	{
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	/// Native asset ids for standalone chains connected to token gateway.
	#[pallet::storage]
	pub type StandaloneChainAssets<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, BTreeSet<H256>, OptionQuery>;

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
		/// Native asset IDs have been registered
		NativeAssetsRegistered { assets: BTreeMap<StateMachine, BTreeSet<H256>> },
		/// Native asset IDs have been deregistered
		NativeAssetsDeregistered { assets: BTreeMap<StateMachine, BTreeSet<H256>> },
	}

	/// Errors that can be returned by this pallet.
	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		<T as pallet_ismp::Config>::Balance: Default,
	{
		/// Register the native token asset ids for standalone chains
		#[pallet::call_index(0)]
		#[pallet::weight(weight())]
		pub fn register_standalone_chain_native_assets(
			origin: OriginFor<T>,
			assets: BTreeMap<StateMachine, BTreeSet<H256>>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			for (state_machine, mut new_asset_ids) in assets.clone() {
				let _ = StandaloneChainAssets::<T>::try_mutate(state_machine, |asset_ids| {
					if let Some(set) = asset_ids {
						set.append(&mut new_asset_ids);
					} else {
						*asset_ids = Some(new_asset_ids);
					};

					Ok::<(), ()>(())
				});
			}

			Self::deposit_event(Event::<T>::NativeAssetsRegistered { assets });

			Ok(())
		}

		/// Deregister the native token asset ids for standalone chains
		#[pallet::call_index(1)]
		#[pallet::weight(weight())]
		pub fn deregister_standalone_chain_native_assets(
			origin: OriginFor<T>,
			assets: BTreeMap<StateMachine, BTreeSet<H256>>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			for (state_machine, new_asset_ids) in assets.clone() {
				let _ = StandaloneChainAssets::<T>::try_mutate(state_machine, |asset_ids| {
					if let Some(set) = asset_ids {
						for id in new_asset_ids {
							set.remove(&id);
						}
						if set.is_empty() {
							*asset_ids = None;
						}
					}
					Ok::<(), ()>(())
				});
			}

			Self::deposit_event(Event::<T>::NativeAssetsDeregistered { assets });

			Ok(())
		}
	}

	// Hack for implementing the [`Default`] bound needed for
	// [`IsmpDispatcher`](ismp::dispatcher::IsmpDispatcher) and
	// [`IsmpModule`](ismp::module::IsmpModule)
	impl<T> Default for Pallet<T> {
		fn default() -> Self {
			Self(PhantomData)
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn is_token_gateway_request(
			from: Vec<u8>,
			to: Vec<u8>,
			source: StateMachine,
			dest: StateMachine,
		) -> bool {
			from == pallet_token_gateway::impls::module_id().0.to_vec() ||
				TokenGatewayParams::<T>::get(source)
					.map(|params| params.address.0.to_vec() == from)
					.unwrap_or_default() ||
				to == pallet_token_gateway::impls::module_id().0.to_vec() ||
				TokenGatewayParams::<T>::get(dest)
					.map(|params| params.address.0.to_vec() == to)
					.unwrap_or_default()
		}

		pub fn inspect_request(post: &PostRequest) -> Result<(), ismp::Error> {
			let PostRequest { body, from, to, source, dest, nonce, .. } = post.clone();

			// Token Gateway contracts on EVM chains are immutable and non upgradeable
			// As long as the initial deployment is valid
			// it's impossible to send malicious requests
			if source.is_evm() && dest.is_evm() {
				return Ok(())
			}

			if Self::is_token_gateway_request(from.clone(), to.clone(), source, dest) {
				let body = Body::abi_decode(&mut &body[1..], true).map_err(|_| {
					ismp::error::Error::ModuleDispatchError {
						msg: "Token Gateway: Failed to decode request body".to_string(),
						meta: Meta { source, dest, nonce },
					}
				})?;

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

				let native_asset_ids = StandaloneChainAssets::<T>::get(source).unwrap_or_default();
				if !native_asset_ids.contains(&H256::from(body.asset_id.0)) && !source.is_evm() {
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
			let PostRequest { body, from, to, source, dest, nonce, .. } = post.clone();
			// Token Gateway contracts on EVM chains are immutable and non upgradeable
			// As long as the initial deployment is valid
			// it's impossible to send malicious requests
			if source.is_evm() && dest.is_evm() {
				return Ok(())
			}

			if Self::is_token_gateway_request(from.clone(), to.clone(), source, dest) {
				let body = Body::abi_decode(&mut &body[1..], true).map_err(|_| {
					ismp::error::Error::ModuleDispatchError {
						msg: "Token Gateway: Failed to decode request body".to_string(),
						meta: Meta { source, dest, nonce },
					}
				})?;

				let native_asset_ids = StandaloneChainAssets::<T>::get(source).unwrap_or_default();
				if !native_asset_ids.contains(&H256::from(body.asset_id.0)) && !source.is_evm() {
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

/// Static weights because benchmarks suck, and we'll be getting PolkaVM soon anyways
fn weight() -> Weight {
	Weight::from_parts(300_000_000, 0)
}
