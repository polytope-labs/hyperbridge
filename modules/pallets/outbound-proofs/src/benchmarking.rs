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
	ConsensusState, CurrentEpoch, LatestMessageBlock, LatestProvenParachainHeight, ProvenHeights,
	Sp1VkeyHash,
};
use types::BeefyConsensusState;

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
		ConsensusState::<T>::put(BeefyConsensusState::default());

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

// Minimal test runtime for benchmark tests
#[cfg(test)]
use polkadot_sdk::*;

#[cfg(test)]
type Block = frame_system::mocking::MockBlock<Test>;

#[cfg(test)]
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		OutboundProofs: crate::pallet,
	}
);

#[cfg(test)]
#[frame_support::derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u128>;
}

#[cfg(test)]
#[frame_support::derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

#[cfg(test)]
pub struct DummyVerifier;
#[cfg(test)]
impl ProofVerifier for DummyVerifier {
	fn verify(
		trusted_state: &BeefyConsensusState,
		_proof: &[u8],
	) -> Result<BeefyConsensusState, frame_support::pallet_prelude::DispatchError> {
		Ok(trusted_state.clone())
	}
}

#[cfg(test)]
pub struct DummyOnDispatch;
#[cfg(test)]
impl pallet_ismp::OnDispatch for DummyOnDispatch {
	fn on_dispatch() {}
}

#[cfg(test)]
frame_support::parameter_types! {
	pub const TreasuryId: frame_support::PalletId = frame_support::PalletId(*b"hb/trsry");
}

#[cfg(test)]
pub struct TestWeights;
#[cfg(test)]
impl pallet::WeightInfo for TestWeights {
	fn submit_proof() -> frame_support::weights::Weight {
		frame_support::weights::Weight::zero()
	}
	fn set_proof_reward() -> frame_support::weights::Weight {
		frame_support::weights::Weight::zero()
	}
	fn set_sp1_vkey_hash() -> frame_support::weights::Weight {
		frame_support::weights::Weight::zero()
	}
}

#[cfg(test)]
impl crate::pallet::Config for Test {
	type AdminOrigin = frame_system::EnsureRoot<u64>;
	type ProofVerifier = DummyVerifier;
	type Currency = Balances;
	type TreasuryPalletId = TreasuryId;
	type MaxProofSize = frame_support::traits::ConstU32<100_000>;
	type MaxStoredProofs = frame_support::traits::ConstU32<100>;
	type WeightInfo = TestWeights;
}

#[cfg(test)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	use sp_runtime::BuildStorage;
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| frame_system::Pallet::<Test>::set_block_number(1));
	ext
}
