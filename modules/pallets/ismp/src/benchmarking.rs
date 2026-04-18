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
// See the License for the specific language governing permissions and
// limitations under the License.

//! Benchmarks for pallet-ismp migration operations.

#![cfg(feature = "runtime-benchmarks")]

use crate::{
	child_trie::{CHILD_TRIE_PREFIX, STATE_COMMITMENTS_KEY},
	Config, Pallet, StateCommitments, StateMachineUpdateTime,
};
use codec::Encode;
use frame_benchmarking::v2::*;
use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	host::StateMachine,
};
use polkadot_sdk::*;
use sp_core::{storage::ChildInfo, H256};

fn dummy_sm_height(i: u64) -> StateMachineHeight {
	StateMachineHeight {
		id: StateMachineId { state_id: StateMachine::Evm(1), consensus_state_id: *b"ETH0" },
		height: i,
	}
}

fn dummy_commitment() -> StateCommitment {
	StateCommitment {
		timestamp: 1000,
		overlay_root: Some(H256::repeat_byte(0xAA)),
		state_root: H256::repeat_byte(0xBB),
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	/// Benchmark clearing `StateCommitments` entries.
	/// Pre-fills N entries, then calls `clear(N, None)`.
	#[benchmark]
	fn drain_state_commitments_step(n: Linear<1, 1_000>) -> Result<(), BenchmarkError> {
		for i in 0..n as u64 {
			StateCommitments::<T>::insert(dummy_sm_height(i), dummy_commitment());
		}

		#[block]
		{
			let _ = StateCommitments::<T>::clear(n, None);
		}

		assert_eq!(StateCommitments::<T>::iter_keys().count(), 0);
		Ok(())
	}

	/// Benchmark clearing `StateMachineUpdateTime` entries.
	/// Pre-fills N entries, then calls `clear(N, None)`.
	#[benchmark]
	fn drain_state_machine_update_time_step(n: Linear<1, 1_000>) -> Result<(), BenchmarkError> {
		for i in 0..n as u64 {
			StateMachineUpdateTime::<T>::insert(dummy_sm_height(i), 1000 + i);
		}

		#[block]
		{
			let _ = StateMachineUpdateTime::<T>::clear(n, None);
		}

		assert_eq!(StateMachineUpdateTime::<T>::iter_keys().count(), 0);
		Ok(())
	}

	/// Benchmark clearing child trie state commitment entries.
	/// Pre-fills N entries, then removes them one by one.
	#[benchmark]
	fn drain_child_trie_state_commitments_step(n: Linear<1, 500>) -> Result<(), BenchmarkError> {
		let child_info = ChildInfo::new_default(CHILD_TRIE_PREFIX);
		for i in 0..n {
			let height = dummy_sm_height(i as u64);
			let key = [
				STATE_COMMITMENTS_KEY.to_vec(),
				sp_io::hashing::keccak_256(&height.encode()).to_vec(),
			]
			.concat();
			sp_io::default_child_storage::set(
				child_info.storage_key(),
				&key,
				&dummy_commitment().encode(),
			);
		}

		#[block]
		{
			for _ in 0..n {
				if let Some(key) = sp_io::default_child_storage::next_key(
					child_info.storage_key(),
					STATE_COMMITMENTS_KEY,
				) {
					if key.starts_with(STATE_COMMITMENTS_KEY) {
						sp_io::default_child_storage::clear(child_info.storage_key(), &key);
					}
				}
			}
		}

		Ok(())
	}
}
