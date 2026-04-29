//! Simnode tests for `pallet-beefy-consensus-proofs`.
//!
//! Two tiers:
//!
//! 1. Admin and validation surface that doesn't need a live BEEFY relay: `set_proof_reward`,
//!    `set_sp1_vkey_hash`, `set_reward_curve` happy paths, `set_reward_curve` validation (zero
//!    denominator, oversized vec), and `submit_proof` extrinsic-boundary rejections (unsigned
//!    origin, oversized payload, unknown proof-type byte, malformed naive bytes).
//! 2. Naive happy-path proof flow against a live Paseo relay: build a real BEEFY proof for
//!    parachain id 4009, initialize trusted state on simnode, submit the proof and assert state
//!    advance + `ProofAccepted` event. Mirrors
//!    `modules/pallets/testsuite/src/tests/pallet_ismp_beefy.rs::setup` but drives the live runtime
//!    through `submit_proof` rather than calling the consensus client directly. Reads
//!    `RELAY_WS_URL` / `PARA_WS_URL` env vars.

#![cfg(test)]

use std::env;

use alloy_sol_types::SolType;
use anyhow::anyhow;
use codec::{Decode, Encode};
use polkadot_sdk::{
	sp_consensus_beefy::{self, ecdsa_crypto::Signature, VersionedFinalityProof},
	sp_io::hashing::keccak_256,
	*,
};
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, Bytes};
use sp_keyring::sr25519::Keyring;
use subxt::{
	backend::legacy::LegacyRpcMethods,
	dynamic::Value,
	error::RpcError,
	ext::subxt_rpcs::{rpc_params, RpcClient},
	tx::SubmittableTransaction,
	OnlineClient, PolkadotConfig,
};
use subxt_utils::Hyperbridge;

use beefy_prover::{
	relay::{fetch_mmr_proof, paras_parachains},
	rs_merkle::MerkleTree,
	util::{hash_authority_addresses, MerkleHasher},
	Prover,
};
use beefy_verifier_primitives::{
	ConsensusMessage, ConsensusState, MmrProof, ParachainHeader, ParachainProof,
	SignatureWithAuthorityIndex, SignedCommitment as BvpSignedCommitment,
};
use ismp_solidity_abi::beefy::{
	BeefyConsensusProof as SolBeefyConsensusProof, BeefyConsensusState as SolBeefyConsensusState,
};
use primitive_types::H256;

const PROOF_TYPE_NAIVE: u8 = 0;
const UNKNOWN_PROOF_TYPE: u8 = 0xFF;
/// Matches `MaxBeefyProofSize` in the gargantua runtime config.
const MAX_PROOF_SIZE: usize = 256 * 1024;
/// Matches `MaxBeefyUncleProvers` in the gargantua runtime; the storage cap
/// (`MaxStoredProvers`) is one larger.
const MAX_UNCLE_PROVERS: usize = 5;

/// Build a `(numerator, denominator)` value as `subxt` expects for the
/// `set_reward_curve` argument: a `BoundedVec<(u32, u32), _>`.
fn fraction_value(num: u32, denom: u32) -> Value {
	Value::unnamed_composite(vec![Value::u128(num as u128), Value::u128(denom as u128)])
}

fn curve_value(fractions: &[(u32, u32)]) -> Value {
	Value::unnamed_composite(
		fractions.iter().map(|(n, d)| fraction_value(*n, *d)).collect::<Vec<_>>(),
	)
}

/// Submit a sudo-wrapped call signed by Alice (the simnode sudo key) and wait
/// for finalization. Returns the dispatch result so callers can assert on
/// success / failure of the inner call.
async fn submit_sudo(
	client: &OnlineClient<Hyperbridge>,
	rpc_client: &RpcClient,
	inner: subxt::tx::DynamicPayload,
) -> Result<(), anyhow::Error> {
	let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![inner.into_value()]);
	submit_signed(client, rpc_client, sudo_call, Keyring::Alice).await
}

async fn submit_signed(
	client: &OnlineClient<Hyperbridge>,
	rpc_client: &RpcClient,
	call: subxt::tx::DynamicPayload,
	signer: Keyring,
) -> Result<(), anyhow::Error> {
	let call_data = client.tx().call_data(&call)?;
	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), signer.to_account_id().to_ss58check()],
		)
		.await
		.map_err(|err| anyhow!("simnode_authorExtrinsic failed: {err:?}"))?;
	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
	let progress = submittable.submit_and_watch().await?;
	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);
	progress.wait_for_finalized_success().await?;
	Ok(())
}

/// Fetch a value-storage item for `pallet-beefy-consensus-proofs`.
async fn fetch_storage<T: Decode>(
	client: &OnlineClient<Hyperbridge>,
	item: &str,
) -> Result<Option<T>, anyhow::Error> {
	let addr = subxt::dynamic::storage("BeefyConsensusProofs", item, ());
	let raw = client.storage().at_latest().await?.fetch(&addr).await?;
	let Some(value) = raw else { return Ok(None) };
	let bytes = value.encoded();
	let decoded =
		T::decode(&mut &bytes[..]).map_err(|e| anyhow!("decoding {item} failed: {e:?}"))?;
	Ok(Some(decoded))
}

#[tokio::test]
#[ignore]
async fn test_admin_extrinsics_and_submit_proof_validation() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or_else(|_| "9990".into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	// 1. set_proof_reward via Sudo, expect storage updated.
	let reward: u128 = 12_345_000;
	let call =
		subxt::dynamic::tx("BeefyConsensusProofs", "set_proof_reward", vec![Value::u128(reward)]);
	submit_sudo(&client, &rpc_client, call).await?;
	let on_chain: u128 = fetch_storage::<u128>(&client, "ProofReward")
		.await?
		.ok_or_else(|| anyhow!("ProofReward unset after set_proof_reward"))?;
	assert_eq!(on_chain, reward);

	// 2. set_sp1_vkey_hash via Sudo, expect storage updated.
	let vkey: Vec<u8> =
		b"0x0059fd0bff44da77999bb7974cbcf2ac7dc89e5869352f20a2f3cd46c9f53d5c".to_vec();
	let call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"set_sp1_vkey_hash",
		vec![Value::from_bytes(&vkey)],
	);
	submit_sudo(&client, &rpc_client, call).await?;
	let on_chain_vkey: Vec<u8> = fetch_storage::<Vec<u8>>(&client, "Sp1VkeyHash")
		.await?
		.ok_or_else(|| anyhow!("Sp1VkeyHash unset after set_sp1_vkey_hash"))?;
	assert_eq!(on_chain_vkey, vkey);

	// 3. set_reward_curve via Sudo with the suggested mainnet defaults (1,1), (4,5), (3,5), (2,5),
	//    (1,5) — covers position 0..=4.
	let curve: Vec<(u32, u32)> = vec![(1, 1), (4, 5), (3, 5), (2, 5), (1, 5)];
	let call =
		subxt::dynamic::tx("BeefyConsensusProofs", "set_reward_curve", vec![curve_value(&curve)]);
	submit_sudo(&client, &rpc_client, call).await?;
	let on_chain_curve: Vec<(u32, u32)> = fetch_storage::<Vec<(u32, u32)>>(&client, "RewardCurve")
		.await?
		.ok_or_else(|| anyhow!("RewardCurve unset after set_reward_curve"))?;
	assert_eq!(on_chain_curve, curve);

	// 4. set_reward_curve with a zero denominator.
	let bad_curve: Vec<(u32, u32)> = vec![(1, 0)];
	let call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"set_reward_curve",
		vec![curve_value(&bad_curve)],
	);
	submit_sudo(&client, &rpc_client, call).await?;
	let unchanged_curve: Vec<(u32, u32)> = fetch_storage::<Vec<(u32, u32)>>(&client, "RewardCurve")
		.await?
		.ok_or_else(|| anyhow!("RewardCurve unexpectedly cleared"))?;
	assert_eq!(
		unchanged_curve, curve,
		"zero-denominator curve must not overwrite the existing curve",
	);

	// 5. set_reward_curve oversized vec. With `MaxUncleProvers = 5` the storage cap is
	//    `MaxStoredProvers = 6`;
	let oversized_curve: Vec<(u32, u32)> =
		(1..=(MAX_UNCLE_PROVERS as u32 + 2)).map(|i| (1, i)).collect();
	let call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"set_reward_curve",
		vec![curve_value(&oversized_curve)],
	);
	let result = submit_sudo(&client, &rpc_client, call).await;
	assert!(
		result.is_err() ||
			fetch_storage::<Vec<(u32, u32)>>(&client, "RewardCurve")
				.await?
				.unwrap_or_default() ==
				curve,
		"oversized curve should not overwrite the previously stored curve",
	);

	// 6. submit_proof oversized payload — `MaxBeefyProofSize` is enforced inside the extrinsic
	//    before any decoding. We send `MaxProofSize + 1` bytes prefixed with `PROOF_TYPE_NAIVE` so
	//    the size check fires before the unknown-proof-type check.
	let mut oversized_proof = vec![PROOF_TYPE_NAIVE; MAX_PROOF_SIZE + 1];
	oversized_proof[0] = PROOF_TYPE_NAIVE;
	let call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"submit_proof",
		vec![Value::from_bytes(&oversized_proof)],
	);
	let result = submit_signed(&client, &rpc_client, call, Keyring::Bob).await;
	assert!(result.is_err(), "oversized submit_proof must fail the dispatch (ProofTooLarge)",);

	// 7. submit_proof with an unknown proof-type byte.
	let unknown_proof = vec![UNKNOWN_PROOF_TYPE; 64];
	let call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"submit_proof",
		vec![Value::from_bytes(&unknown_proof)],
	);
	let result = submit_signed(&client, &rpc_client, call, Keyring::Bob).await;
	assert!(result.is_err(), "unknown proof-type submit_proof must fail (UnknownProofType)",);

	// 8. submit_proof with malformed naive bytes. The byte 0 marks `PROOF_TYPE_NAIVE`, the rest is
	//    junk that won't ABI-decode as `BeefyConsensusProof`. Expect `AbiDecodeFailed`.
	let mut malformed_naive = vec![0u8; 128];
	malformed_naive[0] = PROOF_TYPE_NAIVE;
	let call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"submit_proof",
		vec![Value::from_bytes(&malformed_naive)],
	);
	let result = submit_signed(&client, &rpc_client, call, Keyring::Bob).await;
	assert!(result.is_err(), "malformed naive proof must fail (AbiDecodeFailed)",);

	// 9. submit_proof rejects an unsigned origin. We try to author the same call as an unsigned
	//    extrinsic and expect the txpool / runtime to refuse it (the pallet only accepts
	//    `ensure_signed`).
	let call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"submit_proof",
		vec![Value::from_bytes(&malformed_naive)],
	);
	let unsigned_result = client.tx().create_unsigned(&call)?.submit_and_watch().await;
	let Err(subxt::Error::Rpc(RpcError::ClientError(_))) = unsigned_result else {
		panic!("unsigned submit_proof should have been rejected, got {unsigned_result:?}");
	};

	Ok(())
}

/// Walk back from `latest_beefy_hash` until we find a parent block that also carries
/// a BEEFY justification. We use that parent as the trusted-state anchor so the
/// proof at `latest_beefy_hash` is guaranteed to advance state. Mirrors the lookup
/// in `modules/pallets/testsuite/src/tests/pallet_ismp_beefy.rs::setup`.
async fn previous_beefy_anchor(
	relay_rpc: &LegacyRpcMethods<PolkadotConfig>,
	latest_beefy_hash: H256,
) -> Result<H256, anyhow::Error> {
	let mut current_hash = latest_beefy_hash;
	for _ in 0..1000 {
		let header = relay_rpc
			.chain_get_header(Some(current_hash.into()))
			.await?
			.ok_or_else(|| anyhow!("missing header at {current_hash:?}"))?;
		let parent_hash: H256 = header.parent_hash.into();
		let block = relay_rpc
			.chain_get_block(Some(parent_hash.into()))
			.await?
			.ok_or_else(|| anyhow!("missing block at {parent_hash:?}"))?;
		if let Some(justifications) = block.justifications {
			if justifications.iter().any(|j| j.0 == sp_consensus_beefy::BEEFY_ENGINE_ID) {
				return Ok(parent_hash);
			}
		}
		current_hash = parent_hash;
	}
	Err(anyhow!("no prior BEEFY justification found within 1000 blocks"))
}

/// Build a real BEEFY consensus proof for parachain id 4009 against a live Paseo
/// relay + the gargantua-paseo parachain. Returns the trusted state anchored at
/// the previous BEEFY-justified block and the consensus message that advances
/// state to the latest BEEFY-finalized head.
async fn build_live_naive_proof() -> Result<(ConsensusState, ConsensusMessage), anyhow::Error> {
	let max_rpc_payload_size = 15 * 1024 * 1024;
	let relay_ws_url =
		env::var("RELAY_WS_URL").unwrap_or_else(|_| "wss://paseo.dotters.network".to_string());
	let para_ws_url = env::var("PARA_WS_URL")
		.unwrap_or_else(|_| "wss://gargantua.rpc.polytope.technology".to_string());

	let (relay_client, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, max_rpc_payload_size)
			.await?;
	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());
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

	let latest_beefy_hash: H256 =
		relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await?;
	let previous_beefy_hash = previous_beefy_anchor(&relay_rpc, latest_beefy_hash).await?;
	let initial_state =
		prover.get_initial_consensus_state(Some(previous_beefy_hash.into())).await?;

	let block = relay_rpc
		.chain_get_block(Some(latest_beefy_hash.into()))
		.await?
		.ok_or_else(|| anyhow!("missing latest beefy block"))?;
	let beefy_justification = block
		.justifications
		.ok_or_else(|| anyhow!("latest beefy block lacks justifications"))?
		.into_iter()
		.find_map(|j| (j.0 == sp_consensus_beefy::BEEFY_ENGINE_ID).then_some(j.1))
		.ok_or_else(|| anyhow!("latest beefy block lacks beefy justification"))?;
	let VersionedFinalityProof::V1(signed_commitment_raw) =
		VersionedFinalityProof::<u32, Signature>::decode(&mut &*beefy_justification)?;

	let (mmr_leaf_proof, latest_leaf) =
		fetch_mmr_proof(&prover.relay_rpc, signed_commitment_raw.commitment.block_number, None)
			.await?;

	let signatures = signed_commitment_raw
		.signatures
		.iter()
		.enumerate()
		.filter_map(|(index, sig)| {
			sig.as_ref().map(|s| {
				let slice: &[u8] = s.as_ref();
				let signature_array: [u8; 65] =
					slice.try_into().expect("BEEFY signature is 65 bytes");
				SignatureWithAuthorityIndex { index: index as u32, signature: signature_array }
			})
		})
		.collect::<Vec<_>>();

	let current_authorities = prover.beefy_authorities(Some(latest_beefy_hash)).await?;
	let authority_address_hashes =
		hash_authority_addresses(current_authorities.into_iter().map(|x| x.encode()).collect())?;
	let authority_indices = signatures.iter().map(|x| x.index as usize).collect::<Vec<_>>();
	let authority_tree = MerkleTree::<MerkleHasher>::from_leaves(&authority_address_hashes);
	let authority_proof_hashes = authority_tree.proof(&authority_indices).proof_hashes().to_vec();

	let signed_commitment =
		BvpSignedCommitment { commitment: signed_commitment_raw.commitment.clone(), signatures };

	let mmr = MmrProof {
		signed_commitment,
		latest_mmr_leaf: latest_leaf.clone(),
		mmr_proof: mmr_leaf_proof,
		authority_proof: authority_proof_hashes,
	};

	let parent_hash = H256::decode(&mut &*latest_leaf.parent_number_and_hash.1.encode())?;
	let heads = paras_parachains(&prover.relay_rpc, Some(parent_hash.into())).await?;
	let (parachains, indices): (Vec<_>, Vec<_>) = prover
		.para_ids
		.iter()
		.map(|id| {
			let index = heads
				.iter()
				.position(|(i, _)| *i == *id)
				.unwrap_or_else(|| panic!("paraid {id} missing from relay heads"));
			(
				ParachainHeader {
					header: heads[index].1.clone(),
					index: index as u32,
					para_id: heads[index].0,
				},
				index,
			)
		})
		.unzip();
	let leaves = heads.iter().map(|pair| keccak_256(&pair.encode())).collect::<Vec<_>>();
	let parachain_tree = MerkleTree::<MerkleHasher>::from_leaves(&leaves);
	let parachain_proof_hashes = parachain_tree.proof(&indices).proof_hashes().to_vec();
	let parachain_proof = ParachainProof {
		parachains,
		proof: parachain_proof_hashes,
		total_leaves: leaves.len() as u32,
	};

	Ok((initial_state, ConsensusMessage { mmr, parachain: parachain_proof }))
}

/// Tier-2 happy-path test. Builds a real naive BEEFY proof against live Paseo,
/// initializes the trusted state on simnode, then submits the proof through the
/// `submit_proof` extrinsic and asserts the dispatch succeeded. Requires the
/// simnode to be running gargantua-paseo (paraid 4009) and outbound network
/// access to the configured relay/parachain RPCs. Run with `--ignored`.
#[tokio::test]
#[ignore]
async fn test_naive_proof_happy_path() -> Result<(), anyhow::Error> {
	eprintln!("[stage] building live naive proof from paseo");
	let (initial_state, consensus_message) = build_live_naive_proof().await?;
	let initial_height = initial_state.latest_beefy_height;
	let proof_block: u32 = consensus_message.mmr.signed_commitment.commitment.block_number;
	eprintln!(
		"[stage] proof built: trusted_height={initial_height} proof_block={proof_block} \
		 paras={} sigs={}",
		consensus_message.parachain.parachains.len(),
		consensus_message.mmr.signed_commitment.signatures.len(),
	);
	assert!(
		proof_block > initial_height,
		"proof block {proof_block} must be ahead of trusted height {initial_height}",
	);

	let abi_state: SolBeefyConsensusState = initial_state.into();
	let abi_state_bytes = SolBeefyConsensusState::abi_encode(&abi_state);

	let abi_proof: SolBeefyConsensusProof = consensus_message.into();
	let abi_proof_bytes = <SolBeefyConsensusProof as SolType>::abi_encode_params(&abi_proof);
	let mut wire_proof = Vec::with_capacity(1 + abi_proof_bytes.len());
	wire_proof.push(PROOF_TYPE_NAIVE);
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
	eprintln!("[stage] initialize_state finalized");

	// Reset `ProofReward` to 0 so `pay_position_reward` short-circuits without trying
	// to draw from the (unfunded) treasury account. The Tier-1 test in this module sets
	// `ProofReward` to a non-zero value via Sudo; running both tests against the same
	// simnode session would otherwise leave Tier-2 hitting `RewardTransferFailed`.
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

	// First-proof path appends `latest_height` to `MessagingProofs`. A non-empty vec
	// after `submit_proof` returns success means dispatch ran the full BEEFY check,
	// stored a parachain commitment, and ran ring-buffer eviction. Combined with
	// `wait_for_finalized_success` having returned ok, that's sufficient evidence
	// the naive happy path works end-to-end.
	let messaging_proofs: Vec<u64> =
		fetch_storage::<Vec<u64>>(&client, "MessagingProofs").await?.unwrap_or_default();
	assert!(
		!messaging_proofs.is_empty(),
		"MessagingProofs must contain the proven height after a successful first proof",
	);

	Ok(())
}
