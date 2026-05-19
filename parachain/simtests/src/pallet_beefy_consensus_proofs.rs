//! Simnode tests for `pallet-beefy-consensus-proofs`.
//!
//! Three tiers:
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
//! 3. SP1 uncle dispatch path: mirrors the bench setup in
//!    `modules/pallets/beefy-consensus-proofs/src/benchmarking.rs::submit_proof` to exercise
//!    `settle_uncle_proof` end-to-end. Forces the live BEEFY consensus state ahead of the SP1
//!    fixture proof's block number (so the verifier returns `StaleHeight`), seeds `ProofContext`
//!    with the older snapshot the SP1 verifier accepts, then submits the fixture proof from Bob
//!    (uncle accept at position 0) and re-submits the identical bytes from Ferdie (rejected by
//!    `AcceptedProofHashes` dedup with `ProofAlreadySubmitted`). The multi-position fan-out is
//!    covered by the bench rather than here — generating multiple distinct valid SP1 proofs
//!    requires running `polytope-labs/sp1-beefy` once per fixture. No live network access.

#![cfg(test)]

use std::{
	collections::BTreeMap,
	env,
	time::{SystemTime, UNIX_EPOCH},
};

use alloy_sol_types::SolType;
use anyhow::anyhow;
use codec::{Decode, Encode};
use polkadot_sdk::{
	sp_consensus_beefy::{self, ecdsa_crypto::Signature, VersionedFinalityProof},
	sp_io::hashing::{blake2_128, keccak_256, twox_128, twox_64},
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
use subxt_utils::{values::storage_kv_list_to_value, Hyperbridge};

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
use ismp_abi::ecdsa_beefy::{
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

/// `ConsensusClientId` for BEEFY (`b"BEEF"`); duplicated here because pulling
/// `ismp-beefy` into simtests just for this constant is excessive.
const BEEFY_CONSENSUS_ID: [u8; 4] = *b"BEEF";

/// Path-embedded SP1 fixture produced by the prover (`zk-beefy::tests::test_sp1_beefy`)
/// and consumed by the on-chain SP1Beefy fork test under `evm/tests/foundry/`. Sourcing
/// from one JSON blob keeps prover, EVM and pallet tests in lock-step — bumping the
/// SP1 program (e.g. adding a new public input) only needs the file regenerated.
const SP1_FIXTURE_JSON: &str =
	include_str!("../../../evm/tests/foundry/fixtures/sp1_beefy_fixture.json");

#[derive(serde::Deserialize)]
struct Sp1Fixture {
	block_number: u32,
	previous_state: String,
	proof: String,
}

fn sp1_fixture() -> Sp1Fixture {
	serde_json::from_str(SP1_FIXTURE_JSON).expect("sp1 fixture JSON malformed")
}

/// Decode the ABI-encoded previous_state from the fixture and re-emit it as SCALE
/// `beefy_verifier_primitives::ConsensusState` — the on-chain encoding pallet-ismp uses.
fn fixture_state_scale(latest_beefy_height: u32) -> Vec<u8> {
	use alloy_sol_types::SolValue;
	use beefy_verifier_primitives::ConsensusState;
	use ismp_abi::ecdsa_beefy::BeefyConsensusState as SolBeefyConsensusState;

	let fx = sp1_fixture();
	let raw = hex::decode(fx.previous_state.trim_start_matches("0x")).expect("hex state");
	let sol_state = <SolBeefyConsensusState as SolValue>::abi_decode(&raw).expect("abi state");
	let mut state: ConsensusState = sol_state.into();
	state.latest_beefy_height = latest_beefy_height;
	state.encode()
}

/// Pre-proof snapshot the SP1 verifier accepts inside the uncle path. `latest_beefy_height`
/// is held below the proof's `blockNumber` so verification succeeds.
fn trusted_state_scale() -> Vec<u8> {
	let fx = sp1_fixture();
	let prev_height = {
		use alloy_sol_types::SolValue;
		use ismp_abi::ecdsa_beefy::BeefyConsensusState as SolBeefyConsensusState;
		let raw = hex::decode(fx.previous_state.trim_start_matches("0x")).expect("hex state");
		let sol = <SolBeefyConsensusState as SolValue>::abi_decode(&raw).expect("abi state");
		u32::try_from(sol.latestHeight).expect("latest height fits u32")
	};
	fixture_state_scale(prev_height)
}

/// Same shape as `trusted_state_scale` but with `latest_beefy_height` bumped to equal the
/// proof's `blockNumber`. Stored as the live BEEFY state so dispatch hits SP1Beefy's
/// `StaleHeight` short-circuit and the pallet routes the proof to `settle_uncle_proof`.
fn live_state_scale() -> Vec<u8> {
	fixture_state_scale(sp1_fixture().block_number)
}

/// Wire-format proof: `[PROOF_TYPE_SP1] ++ abi_encode_params(SP1BeefyProof)`.
fn sp1_wire_proof() -> Vec<u8> {
	let fx = sp1_fixture();
	// Discriminant byte: 0x01 == PROOF_TYPE_SP1 (see
	// `modules/pallets/beefy-consensus-proofs/src/types.rs`).
	let mut out = vec![0x01u8];
	out.extend_from_slice(&hex::decode(fx.proof.trim_start_matches("0x")).expect("hex proof"));
	out
}

/// SP1 verification key the fixture proof was generated against — matches the mainnet
/// SP1Beefy deployment at `0x82582f85cf370adCB61D97dab3068c0C4102Ccb6`.
const SP1_FIXTURE_VKEY: [u8; 32] =
	hex_literal::hex!("009ce9c86546ac790c9e694519e16e59ff34b633c309fe4d6a4f850b886cddcf");

/// Storage-key builder for a `Twox64Concat` map (`twox_128(pallet) ++ twox_128(item) ++
/// twox_64(key) ++ key`).
fn twox_64_concat_key(pallet: &[u8], item: &[u8], key: &[u8]) -> Vec<u8> {
	[twox_128(pallet).as_slice(), twox_128(item).as_slice(), twox_64(key).as_slice(), key].concat()
}

/// Storage-key builder for a `Blake2_128Concat` map (`twox_128(pallet) ++ twox_128(item)
/// ++ blake2_128(key) ++ key`).
fn blake2_128_concat_key(pallet: &[u8], item: &[u8], key: &[u8]) -> Vec<u8> {
	[twox_128(pallet).as_slice(), twox_128(item).as_slice(), blake2_128(key).as_slice(), key]
		.concat()
}

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

/// Fetch raw storage bytes by precomputed key, decoding as `T`. Used when the key needs a
/// hashing scheme `subxt::dynamic::storage`'s metadata bridge can't easily express.
async fn fetch_storage_by_key<T: Decode>(
	client: &OnlineClient<Hyperbridge>,
	key: &[u8],
) -> Result<Option<T>, anyhow::Error> {
	let raw = client.storage().at_latest().await?.fetch_raw(key).await?;
	let Some(bytes) = raw else { return Ok(None) };
	let decoded =
		T::decode(&mut &bytes[..]).map_err(|e| anyhow!("decoding raw storage failed: {e:?}"))?;
	Ok(Some(decoded))
}

/// Highest parachain height tracked across both ring buffers. The pallet writes
/// the proven height into `MessagingProofs` for non-rotating proofs and into
/// `RotationProofs` for rotating ones, so neither map alone is a complete view
/// after a successful first proof.
async fn latest_recorded_height(client: &OnlineClient<Hyperbridge>) -> Result<u64, anyhow::Error> {
	let messaging = fetch_storage::<Vec<u64>>(client, "MessagingProofs")
		.await?
		.and_then(|v| v.last().copied())
		.unwrap_or(0);
	let rotation = fetch_storage::<BTreeMap<u64, u64>>(client, "RotationProofs")
		.await?
		.and_then(|m| m.values().copied().max())
		.unwrap_or(0);
	Ok(messaging.max(rotation))
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
	let vkey: [u8; 32] =
		hex_literal::hex!("0059fd0bff44da77999bb7974cbcf2ac7dc89e5869352f20a2f3cd46c9f53d5c");
	let call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"set_sp1_vkey_hash",
		vec![Value::unnamed_composite(vec![Value::from_bytes(&vkey)])],
	);
	submit_sudo(&client, &rpc_client, call).await?;
	let on_chain_vkey: [u8; 32] = fetch_storage::<[u8; 32]>(&client, "Sp1VkeyHash")
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

	// 4b. set_reward_curve with numerator > denominator — would multiply the base reward
	//     above 100% and could drain the treasury on a fat-finger.
	let over_unity: Vec<(u32, u32)> = vec![(1, 1), (3, 2)];
	let call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"set_reward_curve",
		vec![curve_value(&over_unity)],
	);
	submit_sudo(&client, &rpc_client, call).await?;
	let unchanged_curve: Vec<(u32, u32)> = fetch_storage::<Vec<(u32, u32)>>(&client, "RewardCurve")
		.await?
		.ok_or_else(|| anyhow!("RewardCurve unexpectedly cleared"))?;
	assert_eq!(
		unchanged_curve, curve,
		"numerator > denominator curve must not overwrite the existing curve",
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

	// 6. submit_proof oversized payload — `proof: BoundedVec<u8, MaxProofSize>` rejects at the
	//    txpool decode stage, before dispatch. We send `MaxProofSize + 1` bytes prefixed with
	//    `PROOF_TYPE_NAIVE`.
	let mut oversized_proof = vec![PROOF_TYPE_NAIVE; MAX_PROOF_SIZE + 1];
	oversized_proof[0] = PROOF_TYPE_NAIVE;
	let call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"submit_proof",
		vec![Value::from_bytes(&oversized_proof)],
	);
	let result = submit_signed(&client, &rpc_client, call, Keyring::Bob).await;
	assert!(result.is_err(), "oversized submit_proof must be rejected by the BoundedVec decode",);

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

	// Same predicate the verifier uses to decide whether to rotate: the leaf's
	// next-set id is strictly greater than the trusted state's next-set id. When
	// rotation fires, the pallet routes the proof into `RotationProofs` instead
	// of `MessagingProofs`, so we have to know in advance which bucket to check.
	let will_rotate = consensus_message.mmr.latest_mmr_leaf.beefy_next_authority_set.id >
		initial_state.next_authorities.id;

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

	// The first-proof path writes the proven height into `MessagingProofs` when
	// no rotation happened, and into `RotationProofs` when it did. A non-empty
	// entry in the right bucket means dispatch ran the full BEEFY check, stored
	// a parachain commitment, and ran ring-buffer eviction. Combined with
	// `wait_for_finalized_success` having returned ok, that's sufficient
	// evidence the naive happy path works end-to-end.
	if will_rotate {
		let rotation_proofs: BTreeMap<u64, u64> =
			fetch_storage::<BTreeMap<u64, u64>>(&client, "RotationProofs")
				.await?
				.unwrap_or_default();
		assert!(
			!rotation_proofs.is_empty(),
			"RotationProofs must contain the rotation height after a successful rotating proof",
		);
	} else {
		let messaging_proofs: Vec<u64> =
			fetch_storage::<Vec<u64>>(&client, "MessagingProofs").await?.unwrap_or_default();
		assert!(
			!messaging_proofs.is_empty(),
			"MessagingProofs must contain the proven height after a successful first proof",
		);
	}

	Ok(())
}

/// Tier-3 SP1 uncle dispatch path. Mirrors the bench in
/// `modules/pallets/beefy-consensus-proofs/src/benchmarking.rs::submit_proof`: the live
/// BEEFY consensus state is forced to `live_state_scale()` (whose `latest_beefy_height`
/// equals the SP1 fixture proof's `block_number`), so dispatch hits the SP1 verifier's
/// own `StaleHeight` short-circuit before any cryptographic work and the pallet maps
/// that to `StaleProof`, routing the proof to `settle_uncle_proof`. `ProofContext` is
/// pre-seeded with the older `trusted_state_scale()` snapshot so SP1 verification inside
/// the uncle path actually succeeds.
///
/// Multi-uncle accumulation is exercised by appending unique suffix bytes to the SP1
/// fixture for each successive submitter. `alloy-sol-types` 1.5.7's
/// `SP1BeefyProof::abi_decode_params` reads only the bytes the encoded struct needs and
/// silently ignores any trailing junk, so each `WIRE_PROOF ++ <suffix>` decodes to the
/// same `SP1BeefyProof` (verifies against the same Groth16 commitment + public inputs)
/// while producing a distinct `keccak256(proof)` — distinct enough to land in fresh
/// `AcceptedProofHashes` slots without tripping dedup. This is a test-only trick; in
/// production every relayer's SP1 prover already produces independently-randomised
/// proof bytes.
///
/// Four sequential submissions cover the uncle outcomes the pallet exposes:
///
/// 1. Bob (`WIRE_PROOF`): SP1 verifies, uncle accepted at position 0. `ProverCount` becomes 1 and
///    the proof hash is recorded.
/// 2. Charlie (`WIRE_PROOF ++ [0xAA]`): distinct hash, SP1 verifies again, uncle accepted at
///    position 1. `ProverCount` becomes 2.
/// 3. Dave (`WIRE_PROOF ++ [0xBB, 0xBB]`): distinct hash, uncle accepted at position 2.
///    `ProverCount` becomes 3.
/// 4. Ferdie (`WIRE_PROOF`, same bytes Bob already submitted): same hash as (1) → trips the
///    `AcceptedProofHashes` dedup inside `settle_uncle_proof` and the dispatch fails with
///    `ProofAlreadySubmitted`. State invariants must hold: `ProverCount` stays at 3 and
///    `AcceptedProofHashes` retains exactly the three accepted hashes.
///
/// Together these prove the uncle dispatch surface is wired up end-to-end on the live
/// runtime, that `ProverCount` advances correctly across multiple accepts, and that
/// dedup keeps rejecting after several successful uncles. No live network access is
/// required — the SP1 fixture is static.
#[tokio::test]
#[ignore]
async fn test_sp1_uncle_proof_dispatch_path() -> Result<(), anyhow::Error> {
	eprintln!("[stage] sp1 uncle dispatch path");
	let port = env::var("PORT").unwrap_or_else(|_| "9990".into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	// 1. Switch the SP1 vkey to the fixture vkey so verification against the SP1 proof matches.
	//    Idempotent — no-op when Tier-1 left the same vkey in place.
	let vkey_call = subxt::dynamic::tx(
		"BeefyConsensusProofs",
		"set_sp1_vkey_hash",
		vec![Value::unnamed_composite(vec![Value::from_bytes(&SP1_FIXTURE_VKEY)])],
	);
	submit_sudo(&client, &rpc_client, vkey_call).await?;

	// 2. `pay_position_reward` would otherwise try to draw from an unfunded treasury and blow up
	//    the uncle accept; mirror Tier-2's defensive reset.
	let zero_reward =
		subxt::dynamic::tx("BeefyConsensusProofs", "set_proof_reward", vec![Value::u128(0)]);
	submit_sudo(&client, &rpc_client, zero_reward).await?;

	// 3. `settle_uncle_proof` looks up `ProofContext[Self::latest_height()]`. After a successful
	//    first proof, `settle_first_proof` pushes that height into either `MessagingProofs` or
	//    `RotationProofs` depending on whether the proof rotated. Taking the max across both ring
	//    buffers keeps Tier-3 in lockstep with Tier-2 regardless of which path it took, and falls
	//    back to 0 when Tier-3 runs in isolation.
	let parachain_height = latest_recorded_height(&client).await?;
	eprintln!("[stage] seeding ProofContext at parachain_height={parachain_height}");

	// 4. Force the BEEFY consensus state and seed the uncle snapshot via `System::set_storage`. We
	//    override regardless of whether Tier-2 already ran `initialize_state` — the four ISMP keys
	//    cover the fresh-simnode case where `ConsensusStateClient` / `UnbondingPeriod` /
	//    `ConsensusClientUpdateTime` aren't populated yet, while `ConsensusStates` overrides
	//    whatever Tier-2 advanced state to. The values mirror what
	//    `pallet_ismp::create_consensus_client` writes during the bench setup.
	let now_secs = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map_err(|e| anyhow!("system time: {e:?}"))?
		.as_secs();
	let kv_list: Vec<(Vec<u8>, Vec<u8>)> = vec![
		// `Ismp::ConsensusStates` is `Twox64Concat, ConsensusClientId -> Vec<u8>`.
		(
			twox_64_concat_key(b"Ismp", b"ConsensusStates", &BEEFY_CONSENSUS_ID),
			live_state_scale().encode(),
		),
		// `Ismp::ConsensusStateClient` is `Blake2_128Concat, ConsensusStateId ->
		// ConsensusClientId`. ConsensusClientId is `[u8; 4]`.
		(
			blake2_128_concat_key(b"Ismp", b"ConsensusStateClient", &BEEFY_CONSENSUS_ID),
			BEEFY_CONSENSUS_ID.encode(),
		),
		// `Ismp::UnbondingPeriod` is `Blake2_128Concat, ConsensusStateId -> u64`. One
		// year is comfortably above the fixture timestamp window.
		(
			blake2_128_concat_key(b"Ismp", b"UnbondingPeriod", &BEEFY_CONSENSUS_ID),
			(60u64 * 60 * 24 * 365).encode(),
		),
		// `Ismp::ConsensusClientUpdateTime` is `Twox64Concat, ConsensusClientId -> u64`.
		(
			twox_64_concat_key(b"Ismp", b"ConsensusClientUpdateTime", &BEEFY_CONSENSUS_ID),
			now_secs.encode(),
		),
		// `BeefyConsensusProofs::ProofContext` is `Blake2_128Concat, u64 -> Vec<u8>`.
		(
			blake2_128_concat_key(
				b"BeefyConsensusProofs",
				b"ProofContext",
				&parachain_height.encode(),
			),
			trusted_state_scale().encode(),
		),
	];
	let set_storage_call =
		subxt::dynamic::tx("System", "set_storage", vec![storage_kv_list_to_value(&kv_list)]);
	submit_sudo(&client, &rpc_client, set_storage_call).await?;
	eprintln!("[stage] consensus + uncle snapshot seeded");

	// Bob lands the only valid SP1 fixture; Ferdie resubmits identical bytes to exercise
	// dedup. We can't cook multiple distinct uncles cheaply (each needs its own SP1
	// Groth16 proof from `polytope-labs/sp1-beefy`), so the multi-position fan-out is
	// covered by the bench instead. Trailing-byte malleability is now rejected at the
	// extrinsic boundary by `do_submit_proof`'s round-trip check.
	let bob_proof = sp1_wire_proof();
	let ferdie_proof = bob_proof.clone();
	let proof_context_key =
		blake2_128_concat_key(b"BeefyConsensusProofs", b"ProofContext", &parachain_height.encode());
	let prover_count_key =
		blake2_128_concat_key(b"BeefyConsensusProofs", b"ProverCount", &parachain_height.encode());
	let accepted_hashes_key = blake2_128_concat_key(
		b"BeefyConsensusProofs",
		b"AcceptedProofHashes",
		&parachain_height.encode(),
	);

	let bob_hash: H256 = keccak_256(&bob_proof).into();

	// 5. Bob: WIRE_PROOF as-is. Position 0.
	eprintln!("[stage] submit (Bob) — expect uncle accept at position 0");
	submit_signed(
		&client,
		&rpc_client,
		subxt::dynamic::tx(
			"BeefyConsensusProofs",
			"submit_proof",
			vec![Value::from_bytes(&bob_proof)],
		),
		Keyring::Bob,
	)
	.await?;
	let count: u32 = fetch_storage_by_key::<u32>(&client, &prover_count_key).await?.unwrap_or(0);
	let hashes: Vec<H256> = fetch_storage_by_key::<Vec<H256>>(&client, &accepted_hashes_key)
		.await?
		.unwrap_or_default();
	let ctx: Option<Vec<u8>> = fetch_storage_by_key::<Vec<u8>>(&client, &proof_context_key).await?;
	assert_eq!(count, 1, "Bob's uncle should set ProverCount to 1");
	assert_eq!(hashes, vec![bob_hash], "AcceptedProofHashes should record Bob's hash");
	assert!(ctx.is_some(), "ProofContext snapshot must persist across uncle accepts");

	// 6. Ferdie: WIRE_PROOF (same bytes as Bob). Same hash → `AcceptedProofHashes` dedup fires
	//    inside `settle_uncle_proof`, dispatch errors with `ProofAlreadySubmitted`.
	eprintln!(
		"[stage] submit (Ferdie) — expect ProofAlreadySubmitted (Bob's hash already recorded)"
	);
	let ferdie_result = submit_signed(
		&client,
		&rpc_client,
		subxt::dynamic::tx(
			"BeefyConsensusProofs",
			"submit_proof",
			vec![Value::from_bytes(&ferdie_proof)],
		),
		Keyring::Ferdie,
	)
	.await;
	assert!(
		ferdie_result.is_err(),
		"duplicate uncle submission must be rejected by AcceptedProofHashes dedup",
	);

	// State invariants across the failed dispatch — dedup short-circuits before
	// `ProverCount` is bumped or another hash is appended.
	let count: u32 = fetch_storage_by_key::<u32>(&client, &prover_count_key).await?.unwrap_or(0);
	let hashes: Vec<H256> = fetch_storage_by_key::<Vec<H256>>(&client, &accepted_hashes_key)
		.await?
		.unwrap_or_default();
	assert_eq!(count, 1, "rejected duplicate must not bump ProverCount past 1");
	assert_eq!(hashes, vec![bob_hash], "rejected duplicate must not mutate AcceptedProofHashes",);

	Ok(())
}
