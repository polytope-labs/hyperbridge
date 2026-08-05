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
use crate::types::MAX_PHANTOM_TOKEN_PAIRS;
use alloc::vec;
use frame_benchmarking::v2::*;
use frame_support::{
	traits::{Currency, EnsureOrigin, Hooks},
	BoundedVec,
};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
use ismp::{consensus::StateMachineId, host::StateMachine};
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
	fn set_phantom_order_config() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		// A full pair list is the worst case: the duplicate check builds a set over every pair.
		let token_pairs = (0..MAX_PHANTOM_TOKEN_PAIRS)
			.map(|i| types::PhantomTokenPair {
				token_a: H160::from_low_u64_be(i.into()),
				token_b: H160::from_low_u64_be((i + MAX_PHANTOM_TOKEN_PAIRS).into()),
				standard_amount: 1_000_000_000_000_000_000u128,
				standard_amount_b: 1_000_000_000_000_000_000u128,
			})
			.collect::<Vec<_>>()
			.try_into()
			.map_err(|_| BenchmarkError::Stop("pair list exceeds MAX_PHANTOM_TOKEN_PAIRS"))?;

		let config = types::PhantomOrderConfiguration {
			chain: StateMachineId {
				state_id: StateMachine::Evm(8453),
				consensus_state_id: *b"ETH0",
			},
			token_pairs,
			// Must stay strictly above every fallback bid window this benchmark runs against, or
			// the call is rejected with PhantomBidWindowNotShorterThanInterval. The bound is the
			// mock's 100 (gargantua uses 5, nexus 25), and the check is `window < interval`, so
			// 100 itself is not enough.
			interval_blocks: 1_000,
		};

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, config);

		assert!(PhantomOrderConfig::<T>::get().is_some());
		Ok(())
	}

	/// The on_initialize generation path. Every configured pair expands into both directions of
	/// one order, so both the ABI encoding it hashes and the event it deposits grow with the pair
	/// count (two legs per pair) while the storage writes stay fixed. Hence the linear component.
	#[benchmark]
	fn generate_phantom_order(p: Linear<1, MAX_PHANTOM_TOKEN_PAIRS>) -> Result<(), BenchmarkError> {
		let chain =
			StateMachineId { state_id: StateMachine::Evm(8453), consensus_state_id: *b"ETH0" };
		let token_pairs = (0..p)
			.map(|i| types::PhantomTokenPair {
				token_a: H160::from_low_u64_be(i.into()),
				token_b: H160::from_low_u64_be((i + MAX_PHANTOM_TOKEN_PAIRS).into()),
				standard_amount: 1_000_000_000_000_000_000u128,
				standard_amount_b: 1_000_000_000_000_000_000u128,
			})
			.collect::<Vec<_>>()
			.try_into()
			.map_err(|_| BenchmarkError::Stop("pair list exceeds MAX_PHANTOM_TOKEN_PAIRS"))?;

		// Written straight to storage rather than through set_phantom_order_config, so the interval
		// is not checked against the bid window here and any value does. The hook reads it only to
		// decide whether an interval has elapsed, which the kill below settles anyway.
		PhantomOrderConfig::<T>::put(types::PhantomOrderConfiguration {
			chain,
			token_pairs,
			interval_blocks: 100,
		});
		// The hook needs a confirmed height on the destination chain for the deadline, and no
		// prior generation so the interval check passes on the first block.
		pallet_ismp::LatestStateMachineHeight::<T>::insert(chain, 42u64);
		LastPhantomGeneration::<T>::kill();

		let n: BlockNumberFor<T> = 1u32.into();

		#[block]
		{
			<Pallet<T> as Hooks<BlockNumberFor<T>>>::on_initialize(n);
		}

		assert!(CurrentPhantomOrder::<T>::get().is_some());
		Ok(())
	}

	#[benchmark]
	fn set_phantom_bid_window() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let window: u32 = 100;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, window);

		// Verify the window was stored
		assert_eq!(PhantomBidWindow::<T>::get(), window);

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
