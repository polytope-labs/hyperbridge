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

//! Checks the pallet accepts an apk proof whose only contribution is new messages.
//!
//! Rotating and teaching are both proven by watching the prover run, but neither of them is what
//! the bridge is for. Messaging is, and it is the one thing the prover's own loop will not reach on
//! a test relay: a session there lasts about as long as a proof takes, so there is always a
//! rotation waiting and the loop takes that path first. The test does what the loop cannot and
//! picks the block itself.
//!
//! The pallet turns away a proof that rotates nothing, finalizes nothing new and teaches nothing,
//! so acceptance has to come through one of the three. Picking a justification from the set the
//! client already trusts rules out rotation, and the state afterwards shows whether a commitment
//! was learned. If it was not, and the proof was accepted anyway, messages are the only thing left
//! it can have been accepted for, which pins the branch without reaching inside the pallet.
//!
//! Needs the relay, its parachain, and ismp traffic on the parachain so there is something to
//! prove. Takes minutes, since it compiles the circuit and generates a real SNARK.
//!
//!   RELAY_WS_URL=ws://127.0.0.1:9979 PARA_WS_URL=ws://127.0.0.1:9981 PARA_ID=4009 \
//!     PARA_SIGNER=0xe5be9a50... \
//!     cargo test -p tesseract-beefy --no-default-features --features sp1-cluster,local \
//!       --test apk_messaging -- --ignored --nocapture

use std::sync::Arc;

use anyhow::anyhow;
use codec::Decode;
use ismp::{consensus::StateMachineId, host::StateMachine, messaging::ConsensusMessage};
use polkadot_sdk::sp_consensus_beefy::{
	ecdsa_crypto::Signature, SignedCommitment, VersionedFinalityProof,
};
use primitive_types::H256;
use subxt::{backend::legacy::LegacyRpcMethods, config::Header as _};
use tesseract_beefy::{
	backend::{ConsensusProof, OnchainBackend, ProofBackend},
	prover::{
		query_parachain_header, BeefyProver, BeefyProverConfig, ProofVariant, Prover, ProverConfig,
	},
};
use tesseract_substrate::{
	config::{Blake2SubstrateChain, KeccakSubstrateChain},
	SubstrateClient, SubstrateConfig,
};

/// How far back to look for a justification the client can still verify.
const SEARCH_WINDOW: u64 = 2400;

fn env(name: &str) -> String {
	std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}

/// The first BEEFY justification at or above `from`, searched over `window` blocks.
///
/// The prover has its own version of this, but it belongs to a type whose construction compiles
/// the circuit, and the point here is to find out whether there is anything worth proving before
/// paying for that.
async fn justification_at_or_above(
	rpc: &LegacyRpcMethods<Blake2SubstrateChain>,
	from: u64,
	window: u64,
) -> Result<Option<SignedCommitment<u32, Signature>>, anyhow::Error> {
	for number in from..from + window {
		let Some(hash) = rpc.chain_get_block_hash(Some(number.into())).await? else {
			continue;
		};
		let Some(justifications) = rpc
			.chain_get_block(Some(hash))
			.await?
			.ok_or_else(|| anyhow!("failed to find block for {hash:?}"))?
			.justifications
		else {
			continue;
		};
		if let Some(found) = justifications
			.into_iter()
			.find(|(id, _)| id == b"BEEF")
			.map(|(_, encoded)| VersionedFinalityProof::<u32, Signature>::decode(&mut &*encoded))
			.transpose()?
			.map(|VersionedFinalityProof::V1(commitment)| commitment)
		{
			return Ok(Some(found));
		}
	}
	Ok(None)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs a live BLS relay, its parachain, the prover binary and ismp traffic"]
async fn a_messaging_proof_is_accepted_as_new_work() -> Result<(), anyhow::Error> {
	let relay_ws = env("RELAY_WS_URL");
	let para_ws = env("PARA_WS_URL");
	let para_id: u32 = env("PARA_ID").parse().expect("para id is a number");
	let signer = env("PARA_SIGNER");

	let substrate = SubstrateClient::<KeccakSubstrateChain>::new(
		SubstrateConfig {
			state_machine: Some(StateMachine::Kusama(para_id)),
			hashing: None,
			consensus_state_id: None,
			rpc_ws: para_ws.clone(),
			max_rpc_payload_size: None,
			signer: Some(signer),
			initial_height: None,
			max_concurent_queries: None,
			poll_interval: None,
			fee_token_decimals: None,
		}
		.resolve()
		.await?,
	)
	.await?;

	let state_machine_id =
		StateMachineId { state_id: StateMachine::Kusama(para_id), consensus_state_id: *b"PAS0" };
	let backend: Arc<dyn ProofBackend> = Arc::new(OnchainBackend::<KeccakSubstrateChain>::new(
		substrate.client.clone(),
		substrate.rpc_client.clone(),
		substrate.signer.clone(),
		state_machine_id,
	));

	let before = backend.load_state().await?;
	let trusted_set = before.inner.current_authorities.id;

	// The client can only check a signature from the set it currently trusts, and it rejects a
	// justification it has already seen, so the block has to sit above its height and below the
	// end of its session. It also has to finalize a parachain head the client has not reached,
	// which is checked here rather than after spending minutes on a proof the pallet will refuse.
	let (_, relay_rpc_client) =
		subxt_utils::client::ws_client::<Blake2SubstrateChain>(&relay_ws, 15 * 1024 * 1024).await?;
	let relay_rpc = LegacyRpcMethods::<Blake2SubstrateChain>::new(relay_rpc_client);

	// Search upward rather than down. The earliest usable block is the one whose parachain header
	// still names the set the client just rotated into, so it teaches nothing and the acceptance
	// can only be about its messages. Blocks later in the session start naming the set after,
	// which the client does not know yet, and would be accepted for teaching that instead.
	let mut cursor = u64::from(before.inner.latest_beefy_height) + 1;
	let commitment = loop {
		let Some(candidate) = justification_at_or_above(&relay_rpc, cursor, SEARCH_WINDOW).await?
		else {
			return Err(anyhow!("no justification above {cursor} on the relay"));
		};
		let number = candidate.commitment.block_number;
		cursor = u64::from(number) + 1;

		if candidate.commitment.validator_set_id != trusted_set {
			return Err(anyhow!(
				"the client at {} has consumed everything set {trusted_set} signed, so there is \
				 nothing left it can verify. Let the prover run it forward and try again.",
				before.inner.latest_beefy_height,
			));
		}
		let hash = relay_rpc
			.chain_get_block_hash(Some(number.into()))
			.await?
			.ok_or_else(|| anyhow!("relay block {number} vanished"))?;
		let para_head: u64 = query_parachain_header(&relay_rpc, hash, para_id).await?.number.into();
		if para_head > before.finalized_parachain_height {
			break candidate;
		}
		// Seeding a client commits it to the parachain head of the block the seeding ran in, and
		// the relay's view of the parachain trails that, so right after a seed there is nothing to
		// prove until the relay catches up.
		println!(
			"beefy block {number} only finalizes parachain {para_head}, at or below the client's {}",
			before.finalized_parachain_height,
		);
	};

	println!(
		"proving beefy block {} for set {trusted_set} against trusted height {}, this takes minutes",
		commitment.commitment.block_number, before.inner.latest_beefy_height,
	);

	// The prover is built last. Starting it compiles the circuit, minutes of work, and there is no
	// sense paying that before knowing there is a block worth proving.
	let prover_config = ProverConfig {
		relay_rpc_ws: relay_ws.clone(),
		para_rpc_ws: para_ws.clone(),
		para_ids: vec![para_id],
		proof_variant: ProofVariant::Apk,
		max_rpc_payload_size: None,
		query_batch_size: None,
		apk_srs_dir: None,
		sp1_cluster: None,
	};
	let prover: Prover<Blake2SubstrateChain, KeccakSubstrateChain, zk_beefy::DefaultProver> =
		Prover::new(prover_config, Default::default()).await?;
	let beefy = BeefyProver::<
		Blake2SubstrateChain,
		KeccakSubstrateChain,
		zk_beefy::DefaultProver,
		dyn ProofBackend,
	>::new(
		BeefyProverConfig {
			consensus_state_id: *b"PAS0",
			minimum_finalization_height: 0,
			state_machines: vec![StateMachine::Evm(97)],
			backend: Default::default(),
		},
		substrate,
		prover,
		backend.clone(),
	)
	.await?;
	let consensus_proof = beefy.consensus_proof(commitment.clone(), before.inner.clone()).await?;

	// The same call the prover's loop makes, so the test exercises submission rather than
	// imitating it.
	backend
		.send_messages_proof(
			&[StateMachine::Evm(97)],
			ConsensusProof {
				finalized_height: commitment.commitment.block_number,
				set_id: trusted_set,
				message: ConsensusMessage {
					consensus_proof,
					consensus_state_id: *b"PAS0",
					signer: H256::random().as_bytes().to_vec(),
				},
			},
		)
		.await?;

	let after = backend.load_state().await?;
	assert_eq!(
		after.inner.current_authorities.id, trusted_set,
		"the proof rotated the authority set, so it was not accepted for its messages",
	);
	assert_eq!(
		after.inner.next_authorities.bls_poseidon_hash,
		before.inner.next_authorities.bls_poseidon_hash,
		"the proof taught a commitment, so that is what it could have been accepted for",
	);
	assert!(
		after.inner.latest_beefy_height > before.inner.latest_beefy_height,
		"the client did not move, so the proof was not applied",
	);
	assert!(
		after.finalized_parachain_height > before.finalized_parachain_height,
		"no new parachain height was finalized, which is the whole point of a messaging proof",
	);

	println!(
		"messaging proof accepted on set {trusted_set}: beefy {} -> {}, parachain {} -> {}",
		before.inner.latest_beefy_height,
		after.inner.latest_beefy_height,
		before.finalized_parachain_height,
		after.finalized_parachain_height,
	);
	Ok(())
}
