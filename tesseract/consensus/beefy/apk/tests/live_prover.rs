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

//! Assembles a proof from a live chain and checks the verifier accepts it.
//!
//! The fixture tests prove the verifier agrees with Solidity on a proof somebody else built. This
//! closes the other half, that what this crate assembles is something the verifier accepts, which
//! is the path a relayer actually takes.
//!
//! Needs a relay whose BEEFY authorities hold paired `ecdsa_bls_crypto` keys, the parachain it
//! finalizes, and the prover binary. Takes minutes, since it generates a real SNARK.
//!
//!   RELAY_WS_URL=ws://127.0.0.1:9979 PARA_WS_URL=ws://127.0.0.1:9981 PARA_ID=4009 \
//!     APK_PROVER_BINARY=~/Documents/polytope/gnark-apk-proofs/target/release/examples/
//! prove_from_json \     cargo test -p apk-beefy --test live_prover -- --ignored --nocapture

use std::sync::Arc;

use apk_beefy::{CommandProver, Prover};
use beefy_prover::relay::fetch_latest_beefy_justification;
use beefy_verifier_primitives::{ApkAuthoritySet, ApkConsensusMessage, ApkConsensusState};
use polkadot_sdk::*;
use primitive_types::H256;
use subxt::{backend::legacy::LegacyRpcMethods, config::Header as _, PolkadotConfig};

const VERIFYING_KEY: &[u8] =
	include_bytes!("../../../../../evm/tests/foundry/fixtures/apk-verifying-key.bin");

struct TestHost;

impl ismp::messaging::Keccak256 for TestHost {
	fn keccak256(bytes: &[u8]) -> H256 {
		sp_io::hashing::keccak_256(bytes).into()
	}
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs a live BLS relay, its parachain, and the prover binary"]
async fn assembles_a_proof_the_verifier_accepts() {
	let max_rpc_payload_size = 15 * 1024 * 1024;
	let relay_ws_url = std::env::var("RELAY_WS_URL").expect("RELAY_WS_URL must be set");
	let para_ws_url = std::env::var("PARA_WS_URL").expect("PARA_WS_URL must be set");
	let para_id: u32 = std::env::var("PARA_ID")
		.expect("PARA_ID must be set")
		.parse()
		.expect("para id is a number");
	let prover_binary = std::env::var("APK_PROVER_BINARY").expect("APK_PROVER_BINARY must be set");

	let (relay_client, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, max_rpc_payload_size)
			.await
			.unwrap();
	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());
	let (para_client, para_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&para_ws_url, max_rpc_payload_size)
			.await
			.unwrap();
	let para_rpc = LegacyRpcMethods::<PolkadotConfig>::new(para_rpc_client.clone());

	let inner = beefy_prover::Prover {
		beefy_activation_block: 0,
		relay: relay_client,
		relay_rpc: relay_rpc.clone(),
		relay_rpc_client: relay_rpc_client.clone(),
		para: para_client,
		para_rpc,
		para_rpc_client,
		para_ids: vec![para_id],
		query_batch_size: Some(100),
	};

	let work_dir = std::env::temp_dir().join("apk-live-prover");
	let prover = Prover::new(
		inner,
		Arc::new(CommandProver::new(prover_binary.into(), work_dir).expect("prover")),
	);

	let latest: H256 = relay_rpc_client
		.request("beefy_getFinalizedHead", subxt::ext::subxt_rpcs::rpc_params!())
		.await
		.unwrap();

	// Anchor the trusted state at the previous beefy justified block, so the update is not stale.
	let mut previous = H256::default();
	let mut cursor = latest;
	for _ in 0..2000 {
		let header = relay_rpc.chain_get_header(Some(cursor.into())).await.unwrap().unwrap();
		let parent: H256 = header.parent_hash.into();
		if parent.is_zero() {
			panic!("reached genesis without a previous beefy block");
		}
		let block = relay_rpc.chain_get_block(Some(parent.into())).await.unwrap().unwrap();
		if block
			.justifications
			.map(|justifications| {
				justifications.iter().any(|j| j.0 == sp_consensus_beefy::BEEFY_ENGINE_ID)
			})
			.unwrap_or(false)
		{
			previous = parent;
			break;
		}
		cursor = parent;
	}
	assert!(!previous.is_zero(), "no previous beefy block found");

	let trusted = prover.inner.get_initial_consensus_state(Some(previous)).await.unwrap();
	let (signed_commitment, _) =
		fetch_latest_beefy_justification(&prover.inner.relay_rpc, latest).await.unwrap();

	let signed_set_id = signed_commitment.commitment.validator_set_id;
	println!(
		"proving beefy block {} for set {signed_set_id} against trusted height {}, this takes minutes",
		signed_commitment.commitment.block_number, trusted.latest_beefy_height
	);
	let proof = prover
		.consensus_proof(signed_commitment, trusted.clone())
		.await
		.expect("the prover assembles a proof");

	// The signing set's commitment has to be seeded, exactly as `initialize_apk_state` does when
	// bootstrapping a client. Every later one arrives in a header digest, and a set that has none
	// is refused rather than checked against zero, so it has to go on whichever set signed.
	let commitment = H256(prover.current_apk_commitment(latest).await.unwrap());
	let signing_set = signed_set_id;
	let authority_set = |set: sp_consensus_beefy::mmr::BeefyAuthoritySet<H256>| {
		let apk_commitment = if set.id == signing_set { commitment } else { H256::zero() };
		ApkAuthoritySet { id: set.id, len: set.len, apk_commitment }
	};
	let state = ApkConsensusState {
		latest_beefy_height: trusted.latest_beefy_height,
		beefy_activation_block: trusted.beefy_activation_block,
		mmr_root_hash: trusted.mmr_root_hash,
		current_authorities: authority_set(trusted.current_authorities),
		next_authorities: authority_set(trusted.next_authorities),
	};
	assert!(
		state.current_authorities.apk_commitment != H256::zero() ||
			state.next_authorities.apk_commitment != H256::zero(),
		"neither trusted set is the one that signed, so the client would have no commitment",
	);

	let message: ApkConsensusMessage = proof.try_into().expect("proof converts to scale");
	let (new_state, headers) = beefy_verifier::apk::verify_apk_consensus::<TestHost>(
		state.clone(),
		message,
		VERIFYING_KEY,
	)
	.expect("the verifier accepts what the prover built");

	assert!(new_state.latest_beefy_height > state.latest_beefy_height);
	assert_eq!(headers.len(), 1, "should finalize the parachain");
	assert_eq!(headers[0].para_id, para_id);
	println!(
		"prover to verifier round trip: beefy height {} -> {}, para {} finalized",
		state.latest_beefy_height, new_state.latest_beefy_height, para_id
	);
}

/// Writes the abi encoded state `initialize_apk_state` takes, for the set signing right now.
///
/// This is what the relayer's `query_initial_consensus_state` produces for the apk variant, run on
/// its own so a chain can be bootstrapped by hand.
///
///   RELAY_WS_URL=ws://127.0.0.1:9979 PARA_WS_URL=ws://127.0.0.1:9981 PARA_ID=4009 \
///     APK_STATE_OUT=/tmp/apk-state.hex \
///     cargo test -p apk-beefy --test live_prover -- --ignored --nocapture writes_initial_state
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs a live BLS relay"]
async fn writes_initial_state_for_bootstrapping() {
	use alloy_sol_types::SolValue;

	let max_rpc_payload_size = 15 * 1024 * 1024;
	let relay_ws_url = std::env::var("RELAY_WS_URL").expect("RELAY_WS_URL must be set");
	let para_ws_url = std::env::var("PARA_WS_URL").expect("PARA_WS_URL must be set");
	let para_id: u32 = std::env::var("PARA_ID")
		.expect("PARA_ID must be set")
		.parse()
		.expect("para id is a number");
	let out = std::env::var("APK_STATE_OUT").expect("APK_STATE_OUT must be set");

	let (relay_client, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, max_rpc_payload_size)
			.await
			.unwrap();
	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());
	let (para_client, para_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&para_ws_url, max_rpc_payload_size)
			.await
			.unwrap();
	let para_rpc = LegacyRpcMethods::<PolkadotConfig>::new(para_rpc_client.clone());

	let inner = beefy_prover::Prover {
		beefy_activation_block: 0,
		relay: relay_client,
		relay_rpc: relay_rpc.clone(),
		relay_rpc_client: relay_rpc_client.clone(),
		para: para_client,
		para_rpc,
		para_rpc_client,
		para_ids: vec![para_id],
		query_batch_size: Some(100),
	};
	let prover = Prover::new(
		inner,
		Arc::new(CommandProver::new("/nonexistent".into(), std::env::temp_dir()).unwrap()),
	);

	let latest: H256 = relay_rpc_client
		.request("beefy_getFinalizedHead", subxt::ext::subxt_rpcs::rpc_params!())
		.await
		.unwrap();
	let trusted = prover.inner.get_initial_consensus_state(Some(latest)).await.unwrap();

	// Both sets are seeded, not just the current one. A cold start can meet the rotation proof
	// into the next set before it has ever seen a header digest, and a set with no commitment is
	// refused rather than checked against zero. After this the digests take over.
	let current = H256(prover.current_apk_commitment(latest).await.unwrap());
	// Leaving the next set empty is how forward chaining gets demonstrated: the client then has
	// to learn that commitment from a header digest rather than being handed it here.
	let next = match std::env::var("APK_SEED_NEXT").as_deref() {
		Ok("0") => H256::zero(),
		_ => H256(prover.next_apk_commitment(latest).await.unwrap()),
	};
	let state = ApkConsensusState {
		latest_beefy_height: trusted.latest_beefy_height,
		beefy_activation_block: trusted.beefy_activation_block,
		mmr_root_hash: trusted.mmr_root_hash,
		current_authorities: ApkAuthoritySet {
			id: trusted.current_authorities.id,
			len: trusted.current_authorities.len,
			apk_commitment: current,
		},
		next_authorities: ApkAuthoritySet {
			id: trusted.next_authorities.id,
			len: trusted.next_authorities.len,
			apk_commitment: next,
		},
	};

	let abi = ismp_abi::bls_apk_beefy::BlsApkBeefy::BlsApkConsensusState::from(state.clone());
	std::fs::write(&out, format!("0x{}", hex::encode(abi.abi_encode()))).expect("write");
	println!(
		"wrote state for sets {} and {} at beefy height {} to {out}",
		state.current_authorities.id, state.next_authorities.id, state.latest_beefy_height
	);
}
