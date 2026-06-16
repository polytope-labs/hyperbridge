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
/// (`latest_beefy_height` LE) decode to 31_419_345 = 0x01df6bd1, which is below the
/// fixture proof's `blockNumber = 0x01df6bd9`. Used as the pre-proof snapshot in
/// `ProofContext` so `settle_uncle_proof`'s SP1 verifier sees a valid trusted state.
const TRUSTED_STATE_SCALE: [u8; 128] = hex_literal::hex!("d16bdf0118532a0100000000000000000000000000000000000000000000000000000000000000006a13000000000000580200002cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc7416b13000000000000580200002cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc741");

/// Same fixture as `TRUSTED_STATE_SCALE` but with `latest_beefy_height` bumped to
/// 31_419_353 = 0x01df6bd9, which equals the fixture proof's `blockNumber`.
/// Used as the live consensus state so the SP1 verifier inside
/// `BeefyConsensusClient::verify_consensus` returns `StaleHeight` cheaply (its own
/// upfront check, before any cryptographic work). The pallet maps that to `StaleProof`,
/// dispatch routes to `settle_uncle_proof`, and SP1 runs once there. Net cost on the
/// measured path: one SP1 verification + uncle storage writes.
const LIVE_STATE_SCALE: [u8; 128] = hex_literal::hex!("d96bdf0118532a0100000000000000000000000000000000000000000000000000000000000000006a13000000000000580200002cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc7416b13000000000000580200002cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc741");

/// Wire-format proof: `[PROOF_TYPE_SP1] ++ abi_encode_params(SP1BeefyProof)`.
/// ABI bytes lifted verbatim from `evm/tests/foundry/fixtures/sp1_beefy_fixture.json`,
/// which is the same fixture consumed by `SP1BeefyForkTest`.
const WIRE_PROOF: [u8; 1281] = hex_literal::hex!("010000000000000000000000000000000000000000000000000000000001df6bd9000000000000000000000000000000000000000000000000000000000000136a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001df6bd8b06c82d25b39550a06ab64cf89004fce1f913b27190ab108320812295591fa89000000000000000000000000000000000000000000000000000000000000136b00000000000000000000000000000000000000000000000000000000000002582cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc741ed96e512661b155ef81e590ca5ad1bacf2ccce06e7e822ca521daa71efb4ff91000000000000000000000000000000000000000000000000000000000000018000000000000000000000000000000000000000000000000000000000000003608eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000d2700000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000139557ed2657ce1e450327c6006e17e64425bb2154a7e6a55514e3d37fc7fd5d9884697790283bf3e632f74afab019365ed730a6deb0bc3e70bb229635fcf769d28febdf61f520234bde985bfdd18c0b04baf50ddae8f48860b9546184888ce393886265396140661757261209441d70800000000045250535290b43da1ab3f398f7008b0bd1374925ba70102ff77f33c1acce60e98a4e40fb8cf56af7d070449534d500101af5c78d7d0420a25ee6b68dc946d9919da3799b923cff420aa27ab1b646f355794a54ad343c04bc95bb013d63caecc98e97b65edb739cacc2c6e97f7d5aba5c9044953544d20f612176a000000000561757261010162f8da99803bec263b758f801ed06717d9af6178ac74e3c5ecba6e9cf6ab5c33e4daf9b75a18945bc3bb2221e1aceef06b67c87a0c6756872789dffbcd747b810000000000000000000000000000000000000000000000000000000000000000000000000001644388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f2535200000000000000000000000000000000000000000000000000000000000000002607774c88245bcad79f2414d5829f9b61771e86fb92366a5d224ba9a42cea9b16e91bc69ca90c8f455e8973ca2d522e460b371e95a8cdd298e00558bb25e39c1046ac2fb71dfc17f57e39ae5309c9522d97cc181e836aa679be1c168e25180b1556c8c21b6537ff21a57ecb73d497301f5fc9fe8f8d312d03720e401684da5e16440efe811bc61f2bfa210171efc4745d1b7461ce5593c8bcd9a9b2f6489f6e0c8cfbf59f1489e3e9a93143084cd57df0bf06cbf1fce9a7098154abcfb984a90fd4708053142c7043ce767492db2f5c0055f8791ef0cfb31173e9cac6ab47600e3953c2efe616bbd960b6048026dd1e0bb8bba4a29f3ccdb5ac21ce3899751300000000000000000000000000000000000000000000000000000000");

const FIXTURE_VKEY: H256 =
	H256(hex_literal::hex!("007d1720c695842ed647a1a72e981751f9b5e26fc5ca038523b23430a1292f08"));

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

		// The committed nonce in the fixture is Bob's account, so the signer must be Bob for
		// the `nonce == signer` check to pass.
		let submitter: T::AccountId =
			hex_literal::hex!("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48")
				.into();

		let proof =
			frame_support::BoundedVec::<u8, T::MaxProofSize>::truncate_from(WIRE_PROOF.to_vec());

		#[extrinsic_call]
		_(RawOrigin::Signed(submitter), proof);

		// Uncle accepted at position 0; one submitter account recorded under height 0.
		assert_eq!(pallet::ProverCount::<T>::get(0u64), 1);
		assert_eq!(pallet::AcceptedProvers::<T>::get(0u64).len(), 1);
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
		// Mainnet curve: 100%, 50%, 30%, 20%, 10%, 5% — the first prover (position 0) earns the
		// full reward and each successive uncle a decreasing share. The curve is bounded by
		// `MaxStoredProvers` (`MaxUncleProvers + 1`), covering position 0 plus every uncle slot.
		let curve: frame_support::BoundedVec<(u32, u32), pallet::MaxStoredProvers<T>> =
			frame_support::BoundedVec::truncate_from(alloc::vec![
				(1u32, 1u32),
				(1, 2),
				(3, 10),
				(1, 5),
				(1, 10),
				(1, 20),
			]);

		#[extrinsic_call]
		_(RawOrigin::Root, curve.clone());

		assert_eq!(pallet::RewardCurve::<T>::get(), curve);
	}

	// NOTE: `initialize_state` still has no benchmark because it requires an ABI-encoded
	// solidity `BeefyConsensusState` fixture. Add alongside the SP1 fixture once we need
	// its weight to be accurate.
}
