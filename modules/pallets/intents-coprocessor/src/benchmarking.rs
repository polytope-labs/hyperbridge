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
use alloc::{string::ToString, vec};
use frame_benchmarking::v2::*;
use frame_support::{
	traits::{Currency, EnsureOrigin, Hooks},
	BoundedBTreeMap, BoundedVec,
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

	/// `count` distinct token pairs with disjoint unordered token sets, sized like real one-unit
	/// standard amounts.
	fn synthetic_pairs(
		count: u32,
	) -> Result<
		BoundedVec<types::PhantomTokenPair, ConstU32<MAX_PHANTOM_TOKEN_PAIRS>>,
		BenchmarkError,
	> {
		(0..count)
			.map(|i| types::PhantomTokenPair {
				token_a: H160::from_low_u64_be(i.into()),
				token_b: H160::from_low_u64_be((i + MAX_PHANTOM_TOKEN_PAIRS).into()),
				standard_amount: 1_000_000_000_000_000_000u128,
				standard_amount_b: 1_000_000_000_000_000_000u128,
			})
			.collect::<Vec<_>>()
			.try_into()
			.map_err(|_| BenchmarkError::Stop("pair list exceeds MAX_PHANTOM_TOKEN_PAIRS"))
	}

	/// A synthetic phantom chain id, distinct per index.
	fn phantom_chain_id(index: u32) -> StateMachineId {
		StateMachineId { state_id: StateMachine::Evm(index), consensus_state_id: *b"ETH0" }
	}

	/// Configures `count` chains — a map entry, set membership and an active order each — so the
	/// governance calls are measured against a full-sized chain set and order batch.
	fn configure_chains<T: Config>(count: u32) -> Result<(), BenchmarkError> {
		let mut chains = PhantomChains::<T>::get();
		let mut batch = CurrentPhantomOrder::<T>::get().unwrap_or_default();

		for index in 0..count {
			let chain = phantom_chain_id(index);
			PhantomOrderConfig::<T>::insert(chain, synthetic_pairs(1)?);
			chains
				.try_insert(chain)
				.map_err(|_| BenchmarkError::Stop("chain set exceeds MAX_PHANTOM_CHAINS"))?;
			let info = PhantomOrderInfo {
				created_at_block: 1u32.into(),
				chain: chain.state_id.to_string().into_bytes(),
			};
			let _ = batch.try_push((H256::repeat_byte(index as u8), info));
		}

		PhantomChains::<T>::put(chains);
		CurrentPhantomOrder::<T>::put(batch);
		Ok(())
	}

	/// `c` chains configured in one call, each carrying a full pair list: the validation walks
	/// every pair of every chain and each chain costs its own map write, which is what the
	/// linear term pays for.
	#[benchmark]
	fn set_phantom_order_config(c: Linear<1, MAX_PHANTOM_CHAINS>) -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let mut chains = BoundedBTreeMap::new();
		for index in 0..c {
			chains
				.try_insert(phantom_chain_id(index), synthetic_pairs(MAX_PHANTOM_TOKEN_PAIRS)?)
				.map_err(|_| BenchmarkError::Stop("chain map exceeds MAX_PHANTOM_CHAINS"))?;
		}

		let config = types::PhantomOrderConfiguration {
			chains,
			// Must stay strictly above every fallback bid window this benchmark runs against, or
			// the call is rejected with PhantomBidWindowNotShorterThanInterval. The bound is the
			// mock's 100 (gargantua uses 5, nexus 25), and the check is `window < interval`, so
			// 100 itself is not enough.
			interval_blocks: 1_000,
		};

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, config);

		assert_eq!(PhantomChains::<T>::get().len() as u32, c);
		Ok(())
	}

	/// Removing the last chain of a full set: the set write is the worst case.
	#[benchmark]
	fn remove_phantom_order_config() -> Result<(), BenchmarkError> {
		let origin =
			T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		configure_chains::<T>(MAX_PHANTOM_CHAINS)?;
		let chain = phantom_chain_id(MAX_PHANTOM_CHAINS - 1);

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, chain);

		assert!(PhantomOrderConfig::<T>::get(chain).is_none());
		Ok(())
	}

	/// The on_initialize generation path, measured for a single chain: the hook sums this
	/// weight per configured chain. Every pair expands into both directions of the chain's
	/// order, so both the ABI encoding it hashes and the event it deposits grow with the pair
	/// count (two legs per pair) while the storage writes stay fixed. Hence the linear
	/// component.
	#[benchmark]
	fn generate_phantom_order(p: Linear<1, MAX_PHANTOM_TOKEN_PAIRS>) -> Result<(), BenchmarkError> {
		let chain =
			StateMachineId { state_id: StateMachine::Evm(8453), consensus_state_id: *b"ETH0" };
		let token_pairs = synthetic_pairs(p)?;

		// Written straight to storage rather than through set_phantom_order_config, so the interval
		// is not checked against the bid window here and any value does. The hook reads it only to
		// decide whether an interval has elapsed, which the kill below settles anyway.
		PhantomOrderConfig::<T>::insert(chain, token_pairs);
		PhantomOrderInterval::<T>::put(100u32);
		let mut chains = PhantomChains::<T>::get();
		chains
			.try_insert(chain)
			.map_err(|_| BenchmarkError::Stop("chain set exceeds MAX_PHANTOM_CHAINS"))?;
		PhantomChains::<T>::put(chains);
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
