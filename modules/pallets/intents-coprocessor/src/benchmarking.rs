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

#[benchmarks(
    where
		T::AccountId: From<[u8; 32]>
)]
mod benchmarks {
	use super::*;

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

		// The worst case: the caller already holds one bid short of the bound, so the new one
		// is counted against a full walk of its bids on the order.
		for index in 1..T::MaxBidsPerFiller::get() {
			OrderBids::<T>::insert(
				(&commitment, &caller, H256::from_low_u64_be(index.into())),
				deposit,
			);
		}

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), commitment, H256::zero(), user_op);

		// Verify bid was placed
		assert!(OrderBids::<T>::contains_key((&commitment, &caller, H256::zero())));
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
		let _ = Pallet::<T>::place_bid(
			RawOrigin::Signed(caller.clone()).into(),
			commitment,
			H256::zero(),
			user_op,
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), commitment, H256::zero());

		// Verify bid was removed
		assert!(!OrderBids::<T>::contains_key((&commitment, &caller, H256::zero())));
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
	fn upgrade_gateway() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let state_machine = StateMachine::Evm(1);
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};
		let _ = Pallet::<T>::add_deployment(origin.clone(), state_machine, H160::default(), params);

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine, H160::repeat_byte(0x42), vec![0u8; 32]);

		Ok(())
	}

	fn registered_paymaster<T: Config>() -> Result<(T::RuntimeOrigin, StateMachine), BenchmarkError>
	where
		T::AccountId: From<[u8; 32]>,
	{
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let state_machine = StateMachine::Evm(1);
		Pallet::<T>::add_paymaster_deployment(
			origin.clone(),
			state_machine,
			H160::repeat_byte(0xAA),
		)
		.map_err(|_| BenchmarkError::Stop("paymaster registration failed"))?;
		Ok((origin, state_machine))
	}

	#[benchmark]
	fn add_paymaster_deployment() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let state_machine = StateMachine::Evm(1);

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine, H160::repeat_byte(0xAA));

		assert!(Paymasters::<T>::contains_key(state_machine));
		Ok(())
	}

	#[benchmark]
	fn upgrade_paymaster() -> Result<(), BenchmarkError> {
		let (origin, state_machine) = registered_paymaster::<T>()?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine, H160::repeat_byte(0x42), vec![0u8; 32]);

		Ok(())
	}

	#[benchmark]
	fn update_paymaster_params() -> Result<(), BenchmarkError> {
		let (origin, state_machine) = registered_paymaster::<T>()?;
		let params = types::PaymasterParams {
			native_oracle: H160::repeat_byte(0x01),
			markup_bps: U256::from(200),
			treasury: H160::repeat_byte(0x02),
			max_oracle_age: U256::from(90_000),
			swap_slippage_bps: U256::from(200),
		};

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine, params);

		Ok(())
	}

	#[benchmark]
	fn register_paymaster_token() -> Result<(), BenchmarkError> {
		let (origin, state_machine) = registered_paymaster::<T>()?;

		#[extrinsic_call]
		_(
			origin as T::RuntimeOrigin,
			state_machine,
			H160::repeat_byte(0x11),
			H160::repeat_byte(0x22),
		);

		Ok(())
	}

	#[benchmark]
	fn deactivate_paymaster_token() -> Result<(), BenchmarkError> {
		let (origin, state_machine) = registered_paymaster::<T>()?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine, H160::repeat_byte(0x11));

		Ok(())
	}

	#[benchmark]
	fn withdraw_paymaster_assets() -> Result<(), BenchmarkError> {
		let (origin, state_machine) = registered_paymaster::<T>()?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine, H160::zero(), U256::from(1_000_000));

		Ok(())
	}

	#[benchmark]
	fn unlock_paymaster_stake() -> Result<(), BenchmarkError> {
		let (origin, state_machine) = registered_paymaster::<T>()?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine);

		Ok(())
	}

	#[benchmark]
	fn withdraw_paymaster_stake() -> Result<(), BenchmarkError> {
		let (origin, state_machine) = registered_paymaster::<T>()?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, state_machine);

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

	impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
