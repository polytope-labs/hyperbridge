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
use alloc::vec;
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use pallet::{
	BEEFY_CONSENSUS_ID, CurrentEpoch, ProvenHeights, Sp1VkeyHash,
};

#[benchmarks(
	where
		T::AccountId: From<[u8; 32]>,
		<T::Currency as frame_support::traits::fungible::Inspect<T::AccountId>>::Balance: From<u128>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn submit_proof() {
		let caller: T::AccountId = whitelisted_caller();

		CurrentEpoch::<T>::put(0u64);
		pallet_ismp::ConsensusStates::<T>::insert(BEEFY_CONSENSUS_ID, vec![0u8; 32]);

		let proof: BoundedVec<u8, T::MaxProofSize> =
			vec![0u8; 100].try_into().expect("fits in bounds");

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), proof, 1000u64, 500u64, 1u64);

		assert!(ProvenHeights::<T>::contains_key(1000u64));
		assert_eq!(CurrentEpoch::<T>::get(), 1u64);
	}

	#[benchmark]
	fn set_proof_reward() {
		let reward: <T::Currency as frame_support::traits::fungible::Inspect<T::AccountId>>::Balance = 1000u128.into();
		#[extrinsic_call]
		_(RawOrigin::Root, reward);

		assert_eq!(pallet::ProofReward::<T>::get(), reward);
	}

	#[benchmark]
	fn set_sp1_vkey_hash() {
		let vkey = vec![0xabu8; 32];
		#[extrinsic_call]
		_(RawOrigin::Root, vkey.clone());

		assert_eq!(Sp1VkeyHash::<T>::get(), vkey);
	}
}
