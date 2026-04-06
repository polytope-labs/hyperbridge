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
use frame_system::RawOrigin;

#[benchmarks(
	where
		T::AccountId: From<[u8; 32]>,
		<<T as pallet::Config>::Currency as frame_support::traits::fungible::Inspect<T::AccountId>>::Balance: From<u128>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn set_proof_reward() {
		let reward: <<T as pallet::Config>::Currency as frame_support::traits::fungible::Inspect<T::AccountId>>::Balance = 1000u128.into();
		#[extrinsic_call]
		_(RawOrigin::Root, reward);

		assert_eq!(pallet::ProofReward::<T>::get(), reward);
	}
}
