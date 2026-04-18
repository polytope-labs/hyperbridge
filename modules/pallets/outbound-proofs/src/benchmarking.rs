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
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use polkadot_sdk::*;

// SCALE-encoded `beefy_verifier_primitives::ConsensusState` and wire-format proof
// (`[PROOF_TYPE_SP1] ++ SCALE(Sp1BeefyProof)`) for the SP1 Groth16 fixture used in
// `evm/test/SP1BeefyTest.sol::testVerifySp1Optional`. Produced by the ignored helper
// `beefy_verifier::test::dump_sp1_fixture_scale_bytes`.
//
// Fixture encodes a mandatory update (next authority set id advances), so the
// pallet's mandatory branch runs without needing a matching host parachain id.
const TRUSTED_STATE_SCALE: [u8; 128] = hex_literal::hex!("2279d60118532a010000000000000000000000000000000000000000000000000000000000000000751200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49761200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49");

const WIRE_PROOF: [u8; 808] = hex_literal::hex!("012a79d6017512000000000000002979d601e1dbc67b9da4b90227fb3dc2e7ffdce4e120d583502399e4bd083c02651ca5eb761200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f4963bc2eb07f9c83afe64eb8815b626cd0a7d2a1bbb4630a44a1896af297d0135d04e504739e9bd7f1addf87db9b6a762bd0e1713baa895c3b82b4595080e5ba02fb5b3cf2915702b49122c32b822e6a11384074d8902d5ea5f79c7cb0d7804e49501b8b532298f49e38d3f7140ce1ba61c243152e4e380b37eb628e08d5270d8b2c5e4ebedd84bb14066175726120fbc4d208000000000452505352902a869d4e00b3bb93f1e88e41a2b5f51fc637626b4ce1da15749ef2d79de4797a9ae459070449534d50010118a13886ac93d163a1d22cdef94e018eba5189424a66b7bd03a5ac232beb46bf08b0f9d2b979fff833d7e21a64a5183c61e2630c0b452236baba3c1b4ff41821044953544d20ca3be169000000000561757261010152d45dea4dcf058b0610e12981e0e4c97ad153f26481510c0b78beedf1848b4dd2abd37b8c6b800b72fa12199898eca7651471b49e38d6167a84fb6e2df7c78400000000270d000091054388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f2535200000000000000000000000000000000000000000000000000000000000000002ac5e596c552ee76353c176f0870e47a0aa765ceafc4c65b03dbf434e27fa9062f185bdc40f7aae982c1c8c6b766dd491a1e1cd60128efbc58da965e5be96320287f4ce1b04538f0c8287c8eff096c36df67dc17970032546c9b3d4dd5510c5c25e880e13469e1e1aca1b41c367f2ecf04da65f7602fb53ec212b03d0148157b2cd9a79a9779f350d240e6d4c980848302fca8c7447c5fa7ac8d3c6eefcd0c640acff8b27ea316db978652553e3d054765094cf0dab6085a616489cdb973c42b258e22f346ac3ceb3e2e6750c37dad1f98f6ca15d1f70659343caa52dbbcad150b75dd2dcf0ba0a664ea4605b291df54ab1aa5b4c55034b9425ba29cc87eca7b");

const FIXTURE_VKEY: &[u8] =
	b"0x0059fd0bff44da77999bb7974cbcf2ac7dc89e5869352f20a2f3cd46c9f53d5c";

#[benchmarks(
	where
		T::AccountId: From<[u8; 32]>,
		<<T as pallet::Config>::Currency as frame_support::traits::fungible::Inspect<T::AccountId>>::Balance: From<u128>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn submit_proof() {
		pallet_ismp::ConsensusStates::<T>::insert(
			pallet::BEEFY_CONSENSUS_ID,
			TRUSTED_STATE_SCALE.to_vec(),
		);
		pallet::Sp1VkeyHash::<T>::put(FIXTURE_VKEY.to_vec());

		let bounded: BoundedVec<u8, <T as pallet::Config>::MaxProofSize> =
			WIRE_PROOF.to_vec().try_into().expect("wire proof fits in MaxProofSize");
		let prover: T::AccountId = [1u8; 32].into();

		#[extrinsic_call]
		_(RawOrigin::Signed(prover), bounded);

		assert_eq!(pallet::RecentProofs::<T>::get().len(), 1);
	}

	#[benchmark]
	fn set_proof_reward() {
		let reward: <<T as pallet::Config>::Currency as frame_support::traits::fungible::Inspect<
			T::AccountId,
		>>::Balance = 1000u128.into();
		#[extrinsic_call]
		_(RawOrigin::Root, reward);

		assert_eq!(pallet::ProofReward::<T>::get(), reward);
	}

	#[benchmark]
	fn set_sp1_vkey_hash() {
		let vkey = FIXTURE_VKEY.to_vec();
		#[extrinsic_call]
		_(RawOrigin::Root, vkey.clone());

		assert_eq!(pallet::Sp1VkeyHash::<T>::get(), vkey);
	}
}
