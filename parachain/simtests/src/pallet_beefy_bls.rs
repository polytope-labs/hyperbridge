//! Simnode test for the aggregate BLS12-381 BEEFY proof path.
//!
//! Mirrors the naive happy path in [`crate::pallet_beefy_consensus_proofs`], but against a relay
//! chain whose BEEFY authorities hold paired `ecdsa_bls_crypto` keys and whose keyset commitment is
//! over their BLS G2 public keys. The prover builds the proof, the runtime verifies it in a single
//! pairing check, and the consensus state advances.
//!
//! This needs a BLS BEEFY relay, which no public network is. Bring one up from Parity's
//! `skalman--enable-bls-beefy-on-westend` branch with the G2 keyset converter applied (see
//! `docs/bls-beefy-skalman-migration.md`), then point `RELAY_WS_URL` at it. Without that the test
//! cannot run, so it is `#[ignore]`d rather than silently passing.
//!
//!   RELAY_WS_URL=ws://127.0.0.1:9979 PORT=9990 \
//!     cargo test -p simtests bls_beefy -- --ignored --nocapture

#![cfg(test)]

use std::env;

use alloy_sol_types::SolType;
use anyhow::anyhow;
use codec::Decode;
use polkadot_sdk::{sp_consensus_beefy, *};
use sp_keyring::sr25519::Keyring;
use subxt::{
	backend::legacy::LegacyRpcMethods, dynamic::Value, ext::subxt_rpcs::rpc_params, OnlineClient,
	PolkadotConfig,
};
use subxt_utils::Hyperbridge;

use beefy_prover::{bls::decode_paired_justification, Prover};
use beefy_verifier_primitives::{BlsConsensusMessage, ConsensusState, PROOF_TYPE_BLS};
use ismp_abi::{
	bls_beefy::BlsBeefy::BlsBeefyConsensusProof as SolBlsProof,
	ecdsa_beefy::BeefyConsensusState as SolBeefyConsensusState,
};

use crate::pallet_beefy_consensus_proofs::{submit_signed, submit_sudo, BEEFY_CONSENSUS_ID};

/// Build a real BLS consensus proof, and the trusted state it advances from, off a live relay.
async fn build_live_bls_proof() -> Result<(ConsensusState, BlsConsensusMessage), anyhow::Error> {
	let max_rpc_payload_size = 15 * 1024 * 1024;
	let relay_ws_url = env::var("RELAY_WS_URL")
		.map_err(|_| anyhow!("RELAY_WS_URL must point at a BLS BEEFY relay"))?;

	let (relay_client, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, max_rpc_payload_size)
			.await?;
	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());

	// Track the parachain registered on the BLS relay. `pallet-beefy-consensus-proofs` reads the
	// child trie root out of a finalized parachain head, so the proof has to carry one; 4009 is
	// the id gargantua's `is_parachain_tracked` allows and the one its coprocessor resolves to.
	let para_ws_url = env::var("PARA_WS_URL").unwrap_or_else(|_| "ws://127.0.0.1:9991".into());
	let (para_client, para_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&para_ws_url, max_rpc_payload_size)
			.await?;
	let para_rpc = LegacyRpcMethods::<PolkadotConfig>::new(para_rpc_client.clone());

	let prover = Prover {
		beefy_activation_block: 0,
		relay: relay_client,
		relay_rpc: relay_rpc.clone(),
		relay_rpc_client: relay_rpc_client.clone(),
		para: para_client,
		para_rpc,
		para_rpc_client,
		para_ids: vec![4009],
		query_batch_size: Some(100),
	};

	let latest_beefy_hash: sp_core::H256 =
		relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await?;

	let justification = beefy_justification(&relay_rpc, latest_beefy_hash).await?;
	// Keeps the whole 177-byte paired signature, where the ECDSA path would slice out the first
	// 65 bytes.
	let signed_commitment = decode_paired_justification(&justification)?;
	let latest_set_id = signed_commitment.commitment.validator_set_id;

	// Anchor the trusted state one authority set back, so this proof rotates the set.
	//
	// `pallet-beefy-consensus-proofs` rejects a proof that neither rotates the authority set nor
	// finalizes a parachain head it has not already seen. Our BLS relay has no registered
	// parachains, so no proof can ever finalize a head and only a rotation proof is accepted.
	// The anchor has to be exactly one set back: the verifier requires the commitment to be
	// signed by the trusted state's current or next set, so a further-back anchor is rejected
	// outright with `UnknownAuthoritySet`.
	let previous_beefy_hash =
		previous_set_anchor(&relay_rpc, latest_beefy_hash, latest_set_id).await?;
	let initial_state =
		prover.get_initial_consensus_state(Some(previous_beefy_hash.into())).await?;

	let proof = prover.bls_consensus_proof(signed_commitment).await?;

	Ok((initial_state, proof))
}

/// The BEEFY justification attached to `hash`.
async fn beefy_justification(
	relay_rpc: &LegacyRpcMethods<PolkadotConfig>,
	hash: sp_core::H256,
) -> Result<Vec<u8>, anyhow::Error> {
	let block = relay_rpc
		.chain_get_block(Some(hash.into()))
		.await?
		.ok_or_else(|| anyhow!("missing block {hash:?}"))?;

	block
		.justifications
		.ok_or_else(|| anyhow!("block {hash:?} lacks justifications"))?
		.into_iter()
		.find_map(|j| (j.0 == sp_consensus_beefy::BEEFY_ENGINE_ID).then_some(j.1))
		.ok_or_else(|| anyhow!("block {hash:?} lacks a beefy justification"))
}

/// Walk back to a BEEFY-justified block signed by the set immediately before `set_id`.
///
/// Seeding the trusted state there leaves it holding `current = set_id - 1` and `next = set_id`,
/// so a commitment from `set_id` whose leaf announces `set_id + 1` advances the set by one.
async fn previous_set_anchor(
	relay_rpc: &LegacyRpcMethods<PolkadotConfig>,
	from: sp_core::H256,
	set_id: u64,
) -> Result<sp_core::H256, anyhow::Error> {
	let mut cursor = from;

	for _ in 0..4000 {
		let header = relay_rpc
			.chain_get_header(Some(cursor.into()))
			.await?
			.ok_or_else(|| anyhow!("missing header for {cursor:?}"))?;
		let parent: sp_core::H256 = header.parent_hash.into();
		if parent.is_zero() {
			break;
		}

		if let Ok(justification) = beefy_justification(relay_rpc, parent).await {
			let signed = decode_paired_justification(&justification)?;
			if signed.commitment.validator_set_id + 1 == set_id {
				return Ok(parent);
			}
		}

		cursor = parent;
	}

	Err(anyhow!("no beefy block found for the authority set preceding {set_id}"))
}

#[tokio::test]
#[ignore]
async fn bls_beefy_proof_happy_path() -> Result<(), anyhow::Error> {
	eprintln!("[stage] building live bls proof");
	let (initial_state, consensus_message) = build_live_bls_proof().await?;

	let initial_height = initial_state.latest_beefy_height;
	let proof_block = consensus_message.mmr.commitment.block_number;
	let signer_count = consensus_message.mmr.signers.len();
	eprintln!(
		"[stage] proof built: trusted_height={initial_height} proof_block={proof_block} \
		 signers={signer_count}",
	);
	assert!(
		proof_block > initial_height,
		"proof block {proof_block} must be ahead of trusted height {initial_height}",
	);
	assert!(signer_count > 0, "proof carries no signers");

	// The keyset commitment here is over BLS public keys, so this trusted state is only usable by
	// the BLS proof type. Feeding it a naive proof would fail the authority merkle proof.
	let abi_state: SolBeefyConsensusState = initial_state.into();
	let abi_state_bytes = SolBeefyConsensusState::abi_encode(&abi_state);

	let abi_proof: SolBlsProof = consensus_message.into();
	let abi_proof_bytes = <SolBlsProof as SolType>::abi_encode_params(&abi_proof);
	let mut wire_proof = Vec::with_capacity(1 + abi_proof_bytes.len());
	wire_proof.push(PROOF_TYPE_BLS);
	wire_proof.extend_from_slice(&abi_proof_bytes);
	eprintln!(
		"[stage] abi-encoded: state={} bytes proof={} bytes",
		abi_state_bytes.len(),
		wire_proof.len(),
	);

	let port = env::var("PORT").unwrap_or_else(|_| "9990".into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	let init_call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"initialize_state",
		vec![Value::from_bytes(&abi_state_bytes)],
	);
	eprintln!("[stage] submitting initialize_state via sudo");
	submit_sudo(&client, &rpc_client, init_call).await?;

	// Same reason as the naive test: keep `ProofReward` at zero so the reward path does not try to
	// draw from an unfunded treasury account.
	let zero_reward =
		subxt::dynamic::tx("BeefyConsensusProofs", "set_proof_reward", vec![Value::u128(0)]);
	submit_sudo(&client, &rpc_client, zero_reward).await?;

	let submit_call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"submit_proof",
		vec![Value::from_bytes(&wire_proof)],
	);
	eprintln!("[stage] submitting submit_proof signed by Bob");
	submit_signed(&client, &rpc_client, submit_call, Keyring::Bob).await?;
	eprintln!("[stage] submit_proof finalized");

	// The consensus state is the thing that matters: it only advances if the aggregate pairing
	// check, the keyset merkle multi-proof and the MMR leaf proof all passed inside the runtime.
	let final_state = fetch_beefy_consensus_state(&client).await?;
	assert_eq!(
		final_state.latest_beefy_height, proof_block,
		"consensus state should have advanced to the proven block",
	);
	assert!(
		final_state.latest_beefy_height > initial_height,
		"consensus state did not move forward",
	);

	eprintln!(
		"[ok] BLS BEEFY verified in-runtime: {signer_count} signers aggregated, \
		 height {initial_height} -> {proof_block}"
	);

	Ok(())
}

/// Read back the BEEFY consensus state pallet-ismp stores.
async fn fetch_beefy_consensus_state(
	client: &OnlineClient<Hyperbridge>,
) -> Result<ConsensusState, anyhow::Error> {
	let addr = subxt::dynamic::storage(
		"Ismp",
		"ConsensusStates",
		vec![Value::from_bytes(BEEFY_CONSENSUS_ID)],
	);
	let raw = client
		.storage()
		.at_latest()
		.await?
		.fetch(&addr)
		.await?
		.ok_or_else(|| anyhow!("no beefy consensus state stored"))?
		.as_type::<Vec<u8>>()?;

	ConsensusState::decode(&mut &raw[..]).map_err(|e| anyhow!("decode consensus state: {e:?}"))
}
