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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::traits::EnsureOrigin;
use polkadot_sdk::*;

#[benchmarks(
	where
	T::AdminOrigin: EnsureOrigin<T::RuntimeOrigin>
)]
mod benchmarks {
	use super::*;
	use primitive_types::H256;

	/// Benchmark for add_parachain extrinsic
	/// The benchmark creates n parachains and measures the time to add them
	/// to the whitelist.
	///
	/// Parameters:
	/// - `n`: Number of parachains to add in a single call
	#[benchmark]
	fn add_parachain(n: Linear<1, 100>) -> Result<(), BenchmarkError> {
		let origin =
			T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let parachains: Vec<ParachainData> =
			(0..n).map(|i| ParachainData { id: i }).collect();

		#[block]
		{
			Pallet::<T>::add_parachain(origin, parachains)?;
		}

		Ok(())
	}

	/// Benchmark for remove_parachain extrinsic
	/// The benchmark first adds n parachains, then measures the time to remove them
	/// from the whitelist.
	///
	/// Parameters:
	/// - `n`: Number of parachains to remove in a single call
	#[benchmark]
	fn remove_parachain(n: Linear<1, 100>) -> Result<(), BenchmarkError> {
		let origin =
			T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let parachains: Vec<ParachainData> =
			(0..n).map(|i| ParachainData { id: i }).collect();

		Pallet::<T>::add_parachain(origin.clone(), parachains)?;

		#[block]
		{
			Pallet::<T>::remove_parachain(origin, vec![0, 1, 2, 3, 4])?;
		}

		Ok(())
	}

	/// Steady-state `on_finalize`: map already at the cap, evict the oldest
	/// then insert the new height. Mirrors the order used in the hook.
	#[benchmark]
	fn on_finalize_bound_relay_state_commitments() -> Result<(), BenchmarkError> {
		let oldest: u32 = 1;
		for i in 0..crate::MAX_RELAY_STATE_COMMITMENTS {
			let key = oldest + i;
			CurrentRelayChainStateRoots::<T>::insert(key, H256::repeat_byte(0xab));
			KnownRelayHeights::<T>::mutate(|heights| {
				let _ = heights.try_insert(key);
			});
		}

		let new_height = oldest + crate::MAX_RELAY_STATE_COMMITMENTS;

		#[block]
		{
			Pallet::<T>::evict_oldest_relay_commitment();
			CurrentRelayChainStateRoots::<T>::insert(new_height, H256::repeat_byte(0xcd));
			KnownRelayHeights::<T>::mutate(|heights| {
				let _ = heights.try_insert(new_height);
			});
		}

		assert_eq!(CurrentRelayChainStateRoots::<T>::count(), crate::MAX_RELAY_STATE_COMMITMENTS);
		let heights = KnownRelayHeights::<T>::get();
		assert_eq!(heights.len() as u32, crate::MAX_RELAY_STATE_COMMITMENTS);
		assert!(*heights.iter().next().expect("set is non-empty") > oldest);

		Ok(())
	}

	/// One v1 → v2 `SteppedMigration::step` that removes a single
	/// `RelayChainStateCommitments` entry.
	#[benchmark]
	fn migrate_relay_state_commitments_step() -> Result<(), BenchmarkError> {
		for i in 0u32..16u32 {
			RelayChainStateCommitments::<T>::insert(i, H256::repeat_byte(0xef));
		}

		#[block]
		{
			let _ = RelayChainStateCommitments::<T>::clear(1, None);
		}

		assert!(RelayChainStateCommitments::<T>::iter_values().count() < 16);

		Ok(())
	}
}
