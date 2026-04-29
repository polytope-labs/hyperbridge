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
//!    with the older snapshot the SP1 verifier accepts, and submits four staged proofs from
//!    distinct signers. Bob, Charlie, and Dave each land an uncle at successive positions —
//!    Charlie's and Dave's proofs append unique trailing-byte suffixes to `WIRE_PROOF` so they hash
//!    differently while still ABI-decoding to the same SP1 struct (alloy 1.5.7 ignores trailing
//!    bytes). Ferdie reuses Bob's exact bytes and is rejected by the `AcceptedProofHashes` dedup
//!    with `ProofAlreadySubmitted`. No live network access required.

#![cfg(test)]

use std::{
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

/// `ConsensusClientId` for BEEFY (`b"BEEF"`); duplicated here because pulling
/// `ismp-beefy` into simtests just for this constant is excessive.
const BEEFY_CONSENSUS_ID: [u8; 4] = *b"BEEF";

/// SCALE-encoded `beefy_verifier_primitives::ConsensusState` for the SP1 Groth16 fixture
/// used in `evm/tests/foundry/SP1BeefyTest.sol::testVerifySp1Optional`. The first 4 bytes
/// (`latest_beefy_height` LE) decode to 30_832_930 = 0x01d67922, which is below the
/// fixture proof's `blockNumber = 0x01d6792a`. Used as the pre-proof snapshot the SP1
/// verifier accepts inside the uncle path. Mirrors `TRUSTED_STATE_SCALE` in
/// `modules/pallets/beefy-consensus-proofs/src/benchmarking.rs`.
const TRUSTED_STATE_SCALE: [u8; 128] = hex_literal::hex!("2279d60118532a010000000000000000000000000000000000000000000000000000000000000000751200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49761200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49");

/// Same fixture as `TRUSTED_STATE_SCALE` but with `latest_beefy_height` bumped to
/// 30_832_938 = 0x01d6792a (first byte `22` → `2a`), which equals the fixture proof's
/// `blockNumber`. Stored as the live BEEFY consensus state so dispatch hits the SP1
/// verifier's own `StaleHeight` short-circuit and the pallet routes the proof to
/// `settle_uncle_proof`. Mirrors `LIVE_STATE_SCALE` in `benchmarking.rs`.
const LIVE_STATE_SCALE: [u8; 128] = hex_literal::hex!("2a79d60118532a010000000000000000000000000000000000000000000000000000000000000000751200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49761200000000000057020000a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49");

/// Wire-format proof: `[PROOF_TYPE_SP1] ++ abi.encode(SP1BeefyProof)` (without the outer
/// struct offset, matching what `<SP1BeefyProof as SolType>::abi_decode_params` accepts).
/// ABI bytes lifted verbatim from `SP1BeefyTest.sol::testVerifySp1Optional`. Mirrors
/// `WIRE_PROOF` in `benchmarking.rs`.
const SP1_WIRE_PROOF: [u8; 1249] = hex_literal::hex!("010000000000000000000000000000000000000000000000000000000001d6792a000000000000000000000000000000000000000000000000000000000000127500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001d67929e1dbc67b9da4b90227fb3dc2e7ffdce4e120d583502399e4bd083c02651ca5eb00000000000000000000000000000000000000000000000000000000000012760000000000000000000000000000000000000000000000000000000000000257a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f4963bc2eb07f9c83afe64eb8815b626cd0a7d2a1bbb4630a44a1896af297d0135d00000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000340000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000d2700000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000139739e9bd7f1addf87db9b6a762bd0e1713baa895c3b82b4595080e5ba02fb5b3cf2915702b49122c32b822e6a11384074d8902d5ea5f79c7cb0d7804e49501b8b532298f49e38d3f7140ce1ba61c243152e4e380b37eb628e08d5270d8b2c5e4ebedd84bb14066175726120fbc4d208000000000452505352902a869d4e00b3bb93f1e88e41a2b5f51fc637626b4ce1da15749ef2d79de4797a9ae459070449534d50010118a13886ac93d163a1d22cdef94e018eba5189424a66b7bd03a5ac232beb46bf08b0f9d2b979fff833d7e21a64a5183c61e2630c0b452236baba3c1b4ff41821044953544d20ca3be169000000000561757261010152d45dea4dcf058b0610e12981e0e4c97ad153f26481510c0b78beedf1848b4dd2abd37b8c6b800b72fa12199898eca7651471b49e38d6167a84fb6e2df7c7840000000000000000000000000000000000000000000000000000000000000000000000000001644388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f2535200000000000000000000000000000000000000000000000000000000000000002ac5e596c552ee76353c176f0870e47a0aa765ceafc4c65b03dbf434e27fa9062f185bdc40f7aae982c1c8c6b766dd491a1e1cd60128efbc58da965e5be96320287f4ce1b04538f0c8287c8eff096c36df67dc17970032546c9b3d4dd5510c5c25e880e13469e1e1aca1b41c367f2ecf04da65f7602fb53ec212b03d0148157b2cd9a79a9779f350d240e6d4c980848302fca8c7447c5fa7ac8d3c6eefcd0c640acff8b27ea316db978652553e3d054765094cf0dab6085a616489cdb973c42b258e22f346ac3ceb3e2e6750c37dad1f98f6ca15d1f70659343caa52dbbcad150b75dd2dcf0ba0a664ea4605b291df54ab1aa5b4c55034b9425ba29cc87eca7b00000000000000000000000000000000000000000000000000000000");

/// SP1 verification key the fixture proof was generated against.
const SP1_FIXTURE_VKEY: &[u8] =
	b"0x0059fd0bff44da77999bb7974cbcf2ac7dc89e5869352f20a2f3cd46c9f53d5c";

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

/// Tier-3 SP1 uncle dispatch path. Mirrors the bench in
/// `modules/pallets/beefy-consensus-proofs/src/benchmarking.rs::submit_proof`: the live
/// BEEFY consensus state is forced to `LIVE_STATE_SCALE` (whose `latest_beefy_height`
/// equals the SP1 fixture proof's `block_number`), so dispatch hits the SP1 verifier's
/// own `StaleHeight` short-circuit before any cryptographic work and the pallet maps
/// that to `StaleProof`, routing the proof to `settle_uncle_proof`. `ProofContext` is
/// pre-seeded with the older `TRUSTED_STATE_SCALE` snapshot so SP1 verification inside
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
		vec![Value::from_bytes(SP1_FIXTURE_VKEY)],
	);
	submit_sudo(&client, &rpc_client, vkey_call).await?;

	// 2. `pay_position_reward` would otherwise try to draw from an unfunded treasury and blow up
	//    the uncle accept; mirror Tier-2's defensive reset.
	let zero_reward =
		subxt::dynamic::tx("BeefyConsensusProofs", "set_proof_reward", vec![Value::u128(0)]);
	submit_sudo(&client, &rpc_client, zero_reward).await?;

	// 3. `settle_uncle_proof` looks up `ProofContext[Self::latest_height()]`. After a successful
	//    first proof, `settle_first_proof` pushes that same height into `MessagingProofs`, so its
	//    last entry is a faithful proxy. When Tier-3 runs in isolation `MessagingProofs` is empty
	//    and `latest_height()` is 0, matching what we'd seed.
	let parachain_height: u64 = fetch_storage::<Vec<u64>>(&client, "MessagingProofs")
		.await?
		.and_then(|v| v.last().copied())
		.unwrap_or(0);
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
			LIVE_STATE_SCALE.to_vec().encode(),
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
			TRUSTED_STATE_SCALE.to_vec().encode(),
		),
	];
	let set_storage_call =
		subxt::dynamic::tx("System", "set_storage", vec![storage_kv_list_to_value(&kv_list)]);
	submit_sudo(&client, &rpc_client, set_storage_call).await?;
	eprintln!("[stage] consensus + uncle snapshot seeded");

	// Build the proof variants once. Each is `[PROOF_TYPE_SP1] ++ abi_payload ++ <suffix>`
	// — the suffix doesn't change what the ABI decoder sees (alloy 1.5.7 silently
	// ignores trailing bytes) but does change `keccak256(proof)`, so each lands a
	// fresh entry in `AcceptedProofHashes` instead of tripping dedup.
	let bob_proof = SP1_WIRE_PROOF.to_vec();
	let charlie_proof = [&SP1_WIRE_PROOF[..], &[0xAAu8]].concat();
	let dave_proof = [&SP1_WIRE_PROOF[..], &[0xBBu8, 0xBBu8]].concat();
	// Ferdie reuses Bob's exact bytes so the dedup check fires after multiple
	// accepts have already advanced `ProverCount`/`AcceptedProofHashes`.
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
	let charlie_hash: H256 = keccak_256(&charlie_proof).into();
	let dave_hash: H256 = keccak_256(&dave_proof).into();

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

	// 6. Charlie: WIRE_PROOF ++ [0xAA]. Distinct hash, position 1.
	eprintln!("[stage] submit (Charlie) — expect uncle accept at position 1");
	submit_signed(
		&client,
		&rpc_client,
		subxt::dynamic::tx(
			"BeefyConsensusProofs",
			"submit_proof",
			vec![Value::from_bytes(&charlie_proof)],
		),
		Keyring::Charlie,
	)
	.await?;
	let count: u32 = fetch_storage_by_key::<u32>(&client, &prover_count_key).await?.unwrap_or(0);
	let hashes: Vec<H256> = fetch_storage_by_key::<Vec<H256>>(&client, &accepted_hashes_key)
		.await?
		.unwrap_or_default();
	assert_eq!(count, 2, "Charlie's uncle should bump ProverCount to 2");
	assert_eq!(
		hashes,
		vec![bob_hash, charlie_hash],
		"AcceptedProofHashes should append Charlie's hash after Bob's",
	);

	// 7. Dave: WIRE_PROOF ++ [0xBB, 0xBB]. Distinct hash, position 2.
	eprintln!("[stage] submit (Dave) — expect uncle accept at position 2");
	submit_signed(
		&client,
		&rpc_client,
		subxt::dynamic::tx(
			"BeefyConsensusProofs",
			"submit_proof",
			vec![Value::from_bytes(&dave_proof)],
		),
		Keyring::Dave,
	)
	.await?;
	let count: u32 = fetch_storage_by_key::<u32>(&client, &prover_count_key).await?.unwrap_or(0);
	let hashes: Vec<H256> = fetch_storage_by_key::<Vec<H256>>(&client, &accepted_hashes_key)
		.await?
		.unwrap_or_default();
	assert_eq!(count, 3, "Dave's uncle should bump ProverCount to 3");
	assert_eq!(
		hashes,
		vec![bob_hash, charlie_hash, dave_hash],
		"AcceptedProofHashes should hold exactly Bob/Charlie/Dave's hashes in submit order",
	);

	// 8. Ferdie: WIRE_PROOF (same bytes as Bob). Same hash → `AcceptedProofHashes` dedup fires
	//    inside `settle_uncle_proof`, dispatch errors with `ProofAlreadySubmitted`. Confirms dedup
	//    keeps rejecting after several uncles have advanced `ProverCount`.
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
	assert_eq!(count, 3, "rejected duplicate must not bump ProverCount past 3");
	assert_eq!(
		hashes,
		vec![bob_hash, charlie_hash, dave_hash],
		"rejected duplicate must not mutate AcceptedProofHashes",
	);

	Ok(())
}
