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

//! Benchmarking setup for `pallet-hyper-fungible-token`.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::types::{BenchmarkHelper, ChainConfig, SendParams, TokenRegistration, TokenUpdate};
use alloc::collections::BTreeMap;
use frame_benchmarking::v2::*;
use frame_support::{
	traits::{Currency, EnsureOrigin},
	BoundedVec,
};
use frame_system::RawOrigin;
use ismp::host::StateMachine;
use sp_core::H160;

type BalanceOf<T> = <<T as Config>::NativeCurrency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

/// Decimals of the asset the benchmarks bridge. `register_token` rejects an evm side with fewer
/// decimals than the local asset, so this must stay at or below [`EVM_DECIMALS`].
const LOCAL_DECIMALS: u8 = 12;
/// Decimals of the token contract on the evm side.
const EVM_DECIMALS: u8 = 18;

/// The most chains a single `register_token`/`update_token` is benchmarked over.
const MAX_CHAINS: u32 = 100;

/// Creates the bridged asset, funding `who` with enough native currency to cover the asset
/// creation and metadata deposits, and mints them a balance of it.
fn setup_asset<T: Config>(who: &T::AccountId) -> AssetId<T>
where
	BalanceOf<T>: From<u128>,
{
	let unit = 10u128.pow(LOCAL_DECIMALS as u32);
	<T as Config>::NativeCurrency::make_free_balance_be(who, (1_000_000 * unit).into());
	T::BenchmarkHelper::create_asset(LOCAL_DECIMALS, who, 1_000 * unit)
}

/// `count` distinct evm chains, each configured with its own token contract.
fn chain_configs(range: core::ops::Range<u32>) -> BTreeMap<StateMachine, ChainConfig> {
	range
		.map(|i| {
			let mut contract = [0u8; 20];
			contract[..4].copy_from_slice(&i.to_be_bytes());
			(
				StateMachine::Evm(i),
				ChainConfig { token_contract: H160::from(contract), decimals: EVM_DECIMALS },
			)
		})
		.collect()
}

#[benchmarks(
	where
		T::AccountId: From<[u8; 32]>,
		[u8; 32]: From<T::AccountId>,
		u128: From<BalanceOf<T>>,
		BalanceOf<T>: From<u128>,
		<T as pallet_ismp::Config>::Balance: From<BalanceOf<T>>,
		<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::Balance:
			From<BalanceOf<T>> + From<u128>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn send() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let asset_id = setup_asset::<T>(&caller);

		let create_origin =
			T::CreateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		Pallet::<T>::register_token(
			create_origin,
			TokenRegistration {
				local_id: asset_id.clone(),
				native: false,
				chains: chain_configs(0..1),
			},
		)?;

		let unit = 10u128.pow(LOCAL_DECIMALS as u32);
		let balance_before =
			<T::Assets as fungibles::Inspect<T::AccountId>>::balance(asset_id.clone(), &caller);
		let params = SendParams {
			asset_id: asset_id.clone(),
			destination: StateMachine::Evm(0),
			recipient: BoundedVec::truncate_from([1u8; 32].to_vec()),
			amount: unit.into(),
			timeout: 0,
			relayer_fee: 0u128.into(),
			call_data: None,
		};

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), params);

		// the sent amount was burnt from the caller
		let balance_after =
			<T::Assets as fungibles::Inspect<T::AccountId>>::balance(asset_id, &caller);
		assert_eq!(balance_before - balance_after, unit.into());
		Ok(())
	}

	#[benchmark]
	fn register_token(c: Linear<1, MAX_CHAINS>) -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let asset_id = setup_asset::<T>(&caller);
		let origin =
			T::CreateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let registration = TokenRegistration {
			local_id: asset_id.clone(),
			native: false,
			chains: chain_configs(0..c),
		};

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, registration);

		for i in 0..c {
			assert!(TokenContracts::<T>::contains_key(
				StateMachine::Evm(i),
				asset_id.clone()
			));
		}
		Ok(())
	}

	#[benchmark]
	fn update_token(
		a: Linear<0, MAX_CHAINS>,
		r: Linear<0, MAX_CHAINS>,
	) -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let asset_id = setup_asset::<T>(&caller);
		let origin =
			T::CreateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		Pallet::<T>::register_token(
			origin.clone(),
			TokenRegistration {
				local_id: asset_id.clone(),
				native: false,
				chains: chain_configs(0..(a + r)),
			},
		)?;

		// re-point the first `a` chains at fresh contracts, and drop the remaining `r`
		let add_chains = chain_configs(0..a)
			.into_iter()
			.map(|(chain, config)| {
				(chain, ChainConfig { token_contract: H160::repeat_byte(0xff), ..config })
			})
			.collect();
		let remove_chains = (a..(a + r)).map(StateMachine::Evm).collect::<Vec<_>>();
		let update = TokenUpdate { asset_id: asset_id.clone(), add_chains, remove_chains };

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, update);

		for i in 0..a {
			assert_eq!(
				TokenContracts::<T>::get(StateMachine::Evm(i), asset_id.clone()),
				Some(H160::repeat_byte(0xff).0.to_vec())
			);
		}
		for i in a..(a + r) {
			assert!(!TokenContracts::<T>::contains_key(
				StateMachine::Evm(i),
				asset_id.clone()
			));
		}
		Ok(())
	}
}
