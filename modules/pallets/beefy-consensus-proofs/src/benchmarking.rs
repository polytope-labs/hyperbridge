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
use polkadot_sdk::*;
use sp_core::{Get, H256};

/// SCALE-encoded `beefy_verifier_primitives::ConsensusState` for the SP1 Groth16 fixture
/// produced by `zk-beefy::tests::test_sp1_beefy` and committed under
/// `evm/tests/foundry/fixtures/sp1_beefy_fixture.json`. The first 4 bytes
/// (`latest_beefy_height` LE) decode to 31_213_551 = 0x01dc47ef, which is below the
/// fixture proof's `blockNumber = 0x01dc47f7`. Used as the pre-proof snapshot in
/// `ProofContext` so `settle_uncle_proof`'s SP1 verifier sees a valid trusted state.
const TRUSTED_STATE_SCALE: [u8; 128] = hex_literal::hex!("ef47dc0118532a01000000000000000000000000000000000000000000000000000000000000000014130000000000005802000080af94e4aabe6b11819d8e50059b73693140c4e781a3380311ffd1334d36858015130000000000005802000080af94e4aabe6b11819d8e50059b73693140c4e781a3380311ffd1334d368580");

/// Same fixture as `TRUSTED_STATE_SCALE` but with `latest_beefy_height` bumped to
/// 31_213_559 = 0x01dc47f7, which equals the fixture proof's `blockNumber`.
/// Used as the live consensus state so the SP1 verifier inside
/// `BeefyConsensusClient::verify_consensus` returns `StaleHeight` cheaply (its own
/// upfront check, before any cryptographic work). The pallet maps that to `StaleProof`,
/// dispatch routes to `settle_uncle_proof`, and SP1 runs once there. Net cost on the
/// measured path: one SP1 verification + uncle storage writes.
const LIVE_STATE_SCALE: [u8; 128] = hex_literal::hex!("f747dc0118532a01000000000000000000000000000000000000000000000000000000000000000014130000000000005802000080af94e4aabe6b11819d8e50059b73693140c4e781a3380311ffd1334d36858015130000000000005802000080af94e4aabe6b11819d8e50059b73693140c4e781a3380311ffd1334d368580");

/// Wire-format proof: `[PROOF_TYPE_SP1] ++ abi_encode_params(SP1BeefyProof)`.
/// ABI bytes lifted verbatim from `evm/tests/foundry/fixtures/sp1_beefy_fixture.json`,
/// which is the same fixture consumed by `SP1BeefyForkTest`.
const WIRE_PROOF: [u8; 1249] = hex_literal::hex!("010000000000000000000000000000000000000000000000000000000001dc47f7000000000000000000000000000000000000000000000000000000000000131400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001dc47f626de324920a139adbcfec37592c3d5d4f1c5d47be3c962da23f54de266b6b7af0000000000000000000000000000000000000000000000000000000000001315000000000000000000000000000000000000000000000000000000000000025880af94e4aabe6b11819d8e50059b73693140c4e781a3380311ffd1334d368580dd392bcf43ae0ec709e1b092c8104422611665975c6cd579c30dd08e9b087b8700000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000340000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000d2700000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000139371a10acc61519dfb7f41332f5b20fe72f320506c47091aaaf4210d4f078dec7069b6d028d87933c5237ba98c8dca60d753a1053494e6c11df7c23cba900e0cf54797f2a008138fa61940fae7e4f1ec58907ccc57eced453b3d00b29c7f7eaa999e03e09140661757261208caed508000000000452505352906934e5099eb4a44dfb23d258d0510adb4e9a427fc7499b8870130457d826ad10ce1f71070449534d500101db5a2025cacc6a30cb8359a594d7d5cd3b001b060efc658775c43dd9febee19f9b04637257e1a28fb87a12795e7c1f4bb4f43c5c03f904dbda1b6dcbd63883ae044953544d20902e046a0000000005617572610101367b70f2da76391f7f810ab08a393987460cff2d2e290fc689ff6fccc30807568f8e57226898192ad284ee82e6187e07bd3864988c29a905d4f108323d38f18b0000000000000000000000000000000000000000000000000000000000000000000000000001644388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f2535200000000000000000000000000000000000000000000000000000000000000001e42a064240ad4f396a1db358a97274a6b43b143323c70ccb064463c51f620ae18cdd61384fb99d6440bde7472eac01b5441502a7ae704ad89bb1708640b138e228e12adc1afa827f2fa3de02243430dec1f4c9eb55cf7c7d1a0e764ac7b76540f8544daaa4e46e81227c73b6dbb99e60bc412100b588b6d00178b37943a43b2262ecd4e750cf44b41b95b306a24e408cc1b1549b3087717f57660b23eca45de121714e2e0045462d3ce7aa03700daf7842ca59126f6b30819523e3eabc8ad9c0db273840c5d6bf2468b8925dcd6e8aee857e3197ef280dcf0a8d5369be964822b4b0f8508f24277738b1f4c9ca06d2f2166f42f9f9b470bc5968526d55bbbc300000000000000000000000000000000000000000000000000000000");

const FIXTURE_VKEY: H256 =
	H256(hex_literal::hex!("009ce9c86546ac790c9e694519e16e59ff34b633c309fe4d6a4f850b886cddcf"));

#[benchmarks(
	where
		T::AccountId: From<[u8; 32]>,
		<<T as pallet::Config>::Currency as frame_support::traits::fungible::Inspect<T::AccountId>>::Balance: From<u128>,
)]
mod benchmarks {
	use super::*;

	/// Benches the uncle path of `submit_proof` along the single-SP1 worst case. Setup
	/// seeds the live consensus state with `latest_beefy_height` equal to the fixture
	/// proof's `blockNumber`. When dispatch reaches `BeefyConsensusClient::verify_consensus`,
	/// the inner SP1 verifier's own stale check (`beefy_verifier::error::Error::StaleHeight`)
	/// returns immediately — before any cryptographic work — and the pallet maps that to
	/// `StaleProof`. Dispatch then routes to `settle_uncle_proof`, which runs
	/// `verify_sp1_consensus` exactly once against the pre-seeded snapshot in
	/// `ProofContext`. The resulting weight covers one SP1 verification plus uncle storage
	/// writes — also the right bound for the first-proof path, which runs SP1 once inside
	/// `verify_and_apply`.
	#[benchmark]
	fn submit_proof() {
		// Live consensus state is "ahead" of the proof so the verifier's own stale check
		// exits before running SP1. `create_consensus_client` also writes
		// `ConsensusStateClient`, `UnbondingPeriod`, and `ConsensusClientUpdateTime` so
		// the BEEFY client is fully wired up.
		pallet_ismp::Pallet::<T>::create_consensus_client(
			frame_system::RawOrigin::Root.into(),
			ismp::messaging::CreateConsensusState {
				consensus_state: LIVE_STATE_SCALE.to_vec(),
				consensus_client_id: ismp_beefy::BEEFY_CONSENSUS_ID,
				consensus_state_id: ismp_beefy::BEEFY_CONSENSUS_ID,
				unbonding_period: T::UnbondingPeriod::get(),
				challenge_periods: Default::default(),
				state_machine_commitments: Default::default(),
			},
		)
		.expect("create_consensus_client succeeds in benchmark setup");
		pallet::Sp1VkeyHash::<T>::put(FIXTURE_VKEY);

		// Pre-seed the uncle snapshot at `Self::latest_height()` (0 with no parachain
		// commitments stored). The snapshot's `latest_beefy_height` is below the proof's
		// `blockNumber` so the SP1 verifier accepts the proof here.
		pallet::ProofContext::<T>::insert(0u64, TRUSTED_STATE_SCALE.to_vec());

		// Any 32-byte AccountId works. The signed origin doesn't need a keystore entry
		// for the actual signature, just a usable AccountId for reward payout.
		let submitter: T::AccountId = [1u8; 32].into();

		let proof =
			frame_support::BoundedVec::<u8, T::MaxProofSize>::truncate_from(WIRE_PROOF.to_vec());

		#[extrinsic_call]
		_(RawOrigin::Signed(submitter), proof);

		// Uncle accepted at position 0; one hash and one submitter recorded under height 0.
		assert_eq!(pallet::ProverCount::<T>::get(0u64), 1);
		assert_eq!(pallet::AcceptedProofHashes::<T>::get(0u64).len(), 1);
		assert_eq!(pallet::RewardedSubmitters::<T>::get(0u64).len(), 1);
	}

	#[benchmark]
	fn set_proof_reward() {
		let reward: <<T as pallet::Config>::Currency as frame_support::traits::fungible::Inspect<
			T::AccountId,
		>>::Balance = 1_000u128.into();
		#[extrinsic_call]
		_(RawOrigin::Root, reward);

		assert_eq!(pallet::ProofReward::<T>::get(), reward);
	}

	#[benchmark]
	fn set_sp1_vkey_hash() {
		#[extrinsic_call]
		_(RawOrigin::Root, FIXTURE_VKEY);

		assert_eq!(pallet::Sp1VkeyHash::<T>::get(), FIXTURE_VKEY);
	}

	#[benchmark]
	fn set_reward_curve() {
		// Suggested mainnet defaults: 100%, 80%, 60%, 40%, 20%. The curve is bounded by
		// `MaxStoredProvers` (`MaxUncleProvers + 1`), covering position 0 plus every
		// uncle slot.
		let curve: frame_support::BoundedVec<(u32, u32), pallet::MaxStoredProvers<T>> =
			frame_support::BoundedVec::truncate_from(alloc::vec![
				(1u32, 1u32),
				(4, 5),
				(3, 5),
				(2, 5),
				(1, 5),
			]);

		#[extrinsic_call]
		_(RawOrigin::Root, curve.clone());

		assert_eq!(pallet::RewardCurve::<T>::get(), curve);
	}

	// NOTE: `initialize_state` still has no benchmark because it requires an ABI-encoded
	// solidity `BeefyConsensusState` fixture. Add alongside the SP1 fixture once we need
	// its weight to be accurate.
}
