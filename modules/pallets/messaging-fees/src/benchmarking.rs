// Copyright (c) 2025 Polytope Labs.
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
// See the License for the specific lang

use super::*;

use alloc::vec;
use codec::{Decode, Encode};
use frame_benchmarking::v2::*;
use frame_support::{
	migrations::SteppedMigration,
	storage::{storage_prefix, unhashed},
	weights::WeightMeter,
};
use ismp::host::StateMachine;
use polkadot_sdk::{sp_core::U256, *};

#[benchmarks(where T: pallet_migrations::Config)]
mod benchmarks {
	use super::*;
	use crate::migrations::v1::Migration;

	#[benchmark]
	fn migrate_evm_fees() {
		let fee_storage_prefix = storage_prefix(b"Relayer", b"Fees");

		let evm_chain = StateMachine::Evm(1);
		let relayer_addr_32 = vec![1u8; 32];

		let fee_decimals = 6u8;
		let fee_18_decimals = U256::from(100_000_000_000_000_000_000u128);
		let scaling_power = 18u32.saturating_sub(fee_decimals as u32);
		let divisor = U256::from(10u128).pow(U256::from(scaling_power));
		let expected_fee = fee_18_decimals.checked_div(divisor).unwrap();

		let key1_hash = sp_io::hashing::blake2_128(&evm_chain.encode());
		let key2_hash = sp_io::hashing::blake2_128(&relayer_addr_32.encode());

		let key_suffix = [
			key1_hash.as_slice(),
			&evm_chain.encode(),
			key2_hash.as_slice(),
			&relayer_addr_32.encode(),
		]
		.concat();
		let full_key = [fee_storage_prefix.as_slice(), key_suffix.as_slice()].concat();

		unhashed::put(&full_key, &fee_18_decimals);
		pallet_ismp_host_executive::FeeTokenDecimals::<T>::insert(&evm_chain, fee_decimals);

		assert_eq!(unhashed::get::<U256>(&full_key), Some(fee_18_decimals));
		let mut meter = WeightMeter::new();

		#[block]
		{
			Migration::<T>::step(None, &mut meter).unwrap();
		}

		assert_eq!(unhashed::get::<U256>(&full_key), Some(expected_fee));
	}
}
