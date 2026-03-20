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

//! Benchmarking setup for pallet-intents

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use alloc::vec;
use frame_benchmarking::v2::*;
use frame_support::{
	traits::{Currency, EnsureOrigin},
	BoundedVec,
};
use frame_system::RawOrigin;
use ismp::host::StateMachine;
use primitive_types::{H160, H256, U256};
use sp_runtime::traits::ConstU32;
use types::PriceInput;

#[benchmarks(
    where
		T::AccountId: From<[u8; 32]>
)]
mod benchmarks {
	use super::*;
	use frame_system::pallet_prelude::BlockNumberFor;

	#[benchmark]
	fn place_bid() {
		let caller: T::AccountId = whitelisted_caller();
		let commitment = H256::repeat_byte(0xff);
		let user_op: BoundedVec<u8, ConstU32<1_048_576>> =
			vec![1u8; 100].try_into().expect("user_op fits in bounds");

		// Fund the caller
		let deposit = Pallet::<T>::storage_deposit_fee();
		let balance = deposit * 10u32.into();
		<T as Config>::Currency::make_free_balance_be(&caller, balance);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), commitment, user_op);

		// Verify bid was placed
		assert!(Bids::<T>::contains_key(&commitment, &caller));
	}

	#[benchmark]
	fn retract_bid() {
		let caller: T::AccountId = whitelisted_caller();
		let commitment = H256::repeat_byte(0xff);
		let user_op: BoundedVec<u8, ConstU32<1_048_576>> =
			vec![1u8; 100].try_into().expect("user_op fits in bounds");

		// Fund the caller
		let deposit = Pallet::<T>::storage_deposit_fee();
		let balance = deposit * 10u32.into();
		<T as Config>::Currency::make_free_balance_be(&caller, balance);

		// Place a bid first
		let _ =
			Pallet::<T>::place_bid(RawOrigin::Signed(caller.clone()).into(), commitment, user_op);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), commitment);

		// Verify bid was removed
		assert!(!Bids::<T>::contains_key(&commitment, &caller));
	}

	#[benchmark]
	fn add_deployment() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let state_machine = StateMachine::Evm(1);
		let gateway = H160::default();
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine, gateway, params);

		// Verify gateway was added
		assert!(Gateways::<T>::contains_key(state_machine));
		Ok(())
	}

	#[benchmark]
	fn update_params() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let state_machine = StateMachine::Evm(1);
		let gateway = H160::default();
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};

		// Add gateway first
		let _ = Pallet::<T>::add_deployment(origin.clone(), state_machine, gateway, params.clone());

		let params_update = types::ParamsUpdate {
			protocol_fee_bps: Some(U256::from(150)),
			solver_selection: Some(false),
			..Default::default()
		};

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine, params_update);

		Ok(())
	}

	#[benchmark]
	fn sweep_dust() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let state_machine = StateMachine::Evm(1);
		let gateway = H160::default();
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};

		// Add gateway first
		let _ = Pallet::<T>::add_deployment(origin.clone(), state_machine, gateway, params.clone());

		let dust_params = types::SweepDust { beneficiary: H160::default(), outputs: vec![] };

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine, dust_params);

		Ok(())
	}

	#[benchmark]
	fn update_token_decimals() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let state_machine = StateMachine::Evm(1);
		let gateway = H160::default();
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};

		// Add gateway first
		let _ = Pallet::<T>::add_deployment(origin.clone(), state_machine, gateway, params);

		let updates = vec![types::TokenDecimalsUpdate {
			source_chain: vec![1u8; 10],
			tokens: vec![types::TokenDecimal { token: H160::default(), decimals: 18 }],
		}];

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine, updates);

		Ok(())
	}

	#[benchmark]
	fn set_storage_deposit_fee() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, 1000u32.into());

		assert_eq!(StorageDepositFee::<T>::get(), 1000u32.into());
		Ok(())
	}

	#[benchmark]
	fn submit_pair_price(n: Linear<1, 100>) {
		let caller: T::AccountId = whitelisted_caller();
		let pair_id = H256::repeat_byte(0xaa);

		// Use a large balance to cover existential deposit + price deposit on any runtime
		let balance = BalanceOf::<T>::from(u32::MAX);
		<T as Config>::Currency::make_free_balance_be(&caller, balance);

		let deposit_amount = <T as Config>::Currency::minimum_balance();
		PriceDepositAmount::<T>::put(deposit_amount);
		PriceDepositLockDuration::<T>::put(BlockNumberFor::<T>::from(10u32));
		PriceWindowDurationValue::<T>::put(86_400_000u64);

		let count = n.min(T::MaxPriceEntries::get());
		let mut entries_vec = vec![];
		for i in 0..count {
			entries_vec
				.push(PriceInput { amount: U256::from(i * 1000), price: U256::from(2000 + i) });
		}
		let entries: BoundedVec<PriceInput, T::MaxPriceEntries> =
			entries_vec.try_into().expect("entries fit in bounds");

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), pair_id, entries);

		assert!(!Prices::<T>::get(&pair_id).is_empty());
	}

	#[benchmark]
	fn set_price_window_duration() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, 172_800_000u64);

		assert_eq!(PriceWindowDurationValue::<T>::get(), 172_800_000u64);
		Ok(())
	}

	#[benchmark]
	fn set_price_deposit_amount() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, 2000u32.into());

		assert_eq!(PriceDepositAmount::<T>::get(), 2000u32.into());
		Ok(())
	}

	#[benchmark]
	fn set_price_deposit_lock_duration() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, 100u32.into());

		assert_eq!(PriceDepositLockDuration::<T>::get(), 100u32.into());
		Ok(())
	}

	#[benchmark]
	fn withdraw_price_deposit() {
		let caller: T::AccountId = whitelisted_caller();
		let pair_id = H256::repeat_byte(0xdd);

		// Use a large balance to cover existential deposit + price deposit on any runtime
		let balance = BalanceOf::<T>::from(u32::MAX);
		<T as Config>::Currency::make_free_balance_be(&caller, balance);

		let deposit_amount = <T as Config>::Currency::minimum_balance();
		PriceDepositAmount::<T>::put(deposit_amount);
		PriceDepositLockDuration::<T>::put(BlockNumberFor::<T>::from(10u32));
		PriceWindowDurationValue::<T>::put(86_400_000u64);

		let entries: BoundedVec<PriceInput, T::MaxPriceEntries> =
			vec![PriceInput { amount: U256::zero(), price: U256::from(2000) }]
				.try_into()
				.expect("single entry fits");
		let _ = Pallet::<T>::submit_pair_price(
			RawOrigin::Signed(caller.clone()).into(),
			pair_id,
			entries,
		);

		let _ =
			Pallet::<T>::withdraw_price_deposit(RawOrigin::Signed(caller.clone()).into(), pair_id);

		frame_system::Pallet::<T>::set_block_number(100u32.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), pair_id);

		assert!(PriceDeposits::<T>::get(&caller, &pair_id).is_none());
	}

	impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
