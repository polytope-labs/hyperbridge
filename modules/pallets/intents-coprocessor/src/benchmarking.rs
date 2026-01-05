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
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use ismp::host::StateMachine;
use primitive_types::{H160, H256, U256};

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn place_bid() {
		let caller: T::AccountId = whitelisted_caller();
		let commitment = H256::random();
		let user_op = vec![1u8; 100];

		// Fund the caller
		let deposit = T::StorageDepositFee::get();
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
		let commitment = H256::random();
		let user_op = vec![1u8; 100];

		// Fund the caller
		let deposit = T::StorageDepositFee::get();
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
	fn add_gateway_deployment() {
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
		_(RawOrigin::Root, state_machine, gateway, params);

		// Verify gateway was added
		assert!(Gateways::<T>::contains_key(state_machine));
	}

	#[benchmark]
	fn update_gateway_params() {
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
		let _ = Pallet::<T>::add_gateway_deployment(
			RawOrigin::Root.into(),
			state_machine,
			gateway,
			params.clone(),
		);

		let params_update = types::ParamsUpdate {
			protocol_fee_bps: Some(U256::from(150)),
			solver_selection: Some(false),
			..Default::default()
		};

		#[extrinsic_call]
		_(RawOrigin::Root, state_machine, params_update);
	}

	#[benchmark]
	fn sweep_gateway_dust() {
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
		let _ = Pallet::<T>::add_gateway_deployment(
			RawOrigin::Root.into(),
			state_machine,
			gateway,
			params.clone(),
		);

		let sweep_dust = types::SweepDust { beneficiary: H160::default(), outputs: vec![] };

		#[extrinsic_call]
		_(RawOrigin::Root, state_machine, sweep_dust);
	}

	#[benchmark]
	fn update_oracle_token_decimals() {
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
		let _ = Pallet::<T>::add_gateway_deployment(
			RawOrigin::Root.into(),
			state_machine,
			gateway,
			params,
		);

		let updates = vec![types::TokenDecimalsUpdate {
			source_chain: vec![1u8; 10],
			tokens: vec![types::TokenDecimal { token: H160::default(), decimals: 18 }],
		}];

		#[extrinsic_call]
		_(RawOrigin::Root, state_machine, updates);
	}

	impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
