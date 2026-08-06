// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use codec::{Decode, Encode};
use hex_literal::hex;
use polkadot_sdk::{sp_consensus_beefy::VersionedFinalityProof, *};
use sp_core::H256;
use sp_io::hashing::keccak_256;
use subxt::{PolkadotConfig, backend::legacy::LegacyRpcMethods, ext::subxt_rpcs::rpc_params};

use beefy_prover::{
	Prover,
	relay::{fetch_mmr_proof, paras_parachains},
	rs_merkle::MerkleTree,
	util::{MerkleHasher, hash_authority_addresses},
};
use beefy_verifier_primitives::{
	ConsensusMessage, ConsensusState, MmrProof, ParachainHeader, ParachainProof,
	SignatureWithAuthorityIndex, SignedCommitment,
};
use ismp::messaging::Keccak256;
use polkadot_sdk::sp_consensus_beefy::{
	Commitment, MmrRootHash, Payload, ValidatorSetId,
	ecdsa_crypto::Signature,
	mmr::{BeefyAuthoritySet, BeefyNextAuthoritySet, MmrLeaf, MmrLeafVersion},
};
use sp_mmr_primitives::LeafProof;

#[cfg(feature = "bls")]
use crate::verify_bls_consensus;
use crate::{EcdsaRecover, error::Error, verify_consensus, verify_mmr_update_proof};

struct TestHost;

impl Keccak256 for TestHost {
	fn keccak256(bytes: &[u8]) -> H256 {
		sp_io::hashing::keccak_256(bytes).into()
	}
}

impl EcdsaRecover for TestHost {
	fn secp256k1_recover(prehash: &[u8; 32], signature: &[u8; 65]) -> anyhow::Result<[u8; 64]> {
		sp_io::crypto::secp256k1_ecdsa_recover(signature, prehash)
			.map_err(|_| anyhow::anyhow!("Failed to recover secp256k1 public key"))
	}
}

#[cfg(feature = "bls-crypto")]
impl crate::BlsAggregateVerify for TestHost {
	fn verify_aggregate(
		message: &[u8],
		signature: &[u8; beefy_verifier_primitives::BLS_G1_SIGNATURE_LEN],
		public_keys: &[[u8; beefy_verifier_primitives::BLS_G2_PUBLIC_KEY_LEN]],
	) -> anyhow::Result<bool> {
		crate::bls::aggregate_verify(message, signature, public_keys)
	}
}

// Integration test: hits live Polkadot/parachain RPCs (see RELAY_WS_URL / PARA_WS_URL env vars).
// Run explicitly with `cargo test -- --ignored`.
#[tokio::test]
#[ignore]
async fn test_verify_consensus() {
	let max_rpc_payload_size = 15 * 1024 * 1024;

	let relay_ws_url =
		std::env::var("RELAY_WS_URL").unwrap_or("wss://rpc.ibp.network/polkadot".to_string());
	let para_ws_url =
		std::env::var("PARA_WS_URL").unwrap_or("wss://nexus.dotters.network".to_string());

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

	let prover = Prover {
		beefy_activation_block: 0,
		relay: relay_client.clone(),
		relay_rpc: relay_rpc.clone(),
		relay_rpc_client: relay_rpc_client.clone(),
		para: para_client.clone(),
		para_rpc,
		para_rpc_client,
		para_ids: vec![],
		query_batch_size: Some(100),
	};

	println!("Finding latest and previous beefy blocks...");
	let latest_beefy_hash: H256 =
		relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await.unwrap();

	let mut previous_beefy_hash = H256::default();
	let mut current_hash = latest_beefy_hash;
	for _ in 0..1000 {
		let header = relay_rpc.chain_get_header(Some(current_hash.into())).await.unwrap().unwrap();
		let parent_hash: H256 = header.parent_hash.into();

		if parent_hash.is_zero() {
			panic!("Reached genesis block without finding a previous beefy block.");
		}

		let block = relay_rpc.chain_get_block(Some(parent_hash.into())).await.unwrap().unwrap();

		if let Some(justifications) = block.justifications {
			if justifications.iter().any(|j| j.0 == sp_consensus_beefy::BEEFY_ENGINE_ID) {
				previous_beefy_hash = parent_hash;
				break;
			}
		}
		current_hash = parent_hash;
	}

	if previous_beefy_hash.is_zero() {
		panic!("Could not find a previous BEEFY block to initialize the state.");
	}

	println!("Getting initial consensus state from block: {:?}", previous_beefy_hash);
	let trusted_state =
		prover.get_initial_consensus_state(Some(previous_beefy_hash)).await.unwrap();

	let (signed_commitment_raw, block_hash) = {
		let block = relay_rpc
			.chain_get_block(Some(latest_beefy_hash.into()))
			.await
			.unwrap()
			.unwrap();
		let justifications =
			block.justifications.expect("Latest beefy block must have justifications");
		let beefy_justification = justifications
			.into_iter()
			.find_map(|j| (j.0 == sp_consensus_beefy::BEEFY_ENGINE_ID).then_some(j.1))
			.expect("Latest beefy block must have a beefy justification");

		let VersionedFinalityProof::V1(signed_commitment) =
			VersionedFinalityProof::<u32, Signature>::decode(&mut &*beefy_justification)
				.expect("Beefy justification should decode correctly");
		(signed_commitment, latest_beefy_hash)
	};

	let block_number = signed_commitment_raw.commitment.block_number;

	println!("Generating the relay chain proof for block #{}", block_number);
	let (mmr_leaf_proof, latest_leaf) =
		fetch_mmr_proof(&prover.relay_rpc, block_number, None).await.unwrap();

	let signatures = signed_commitment_raw
		.signatures
		.iter()
		.enumerate()
		.filter_map(|(index, sig)| {
			sig.as_ref().map(|s: &Signature| {
				let slice: &[u8] = s.as_ref();
				let signature_array: [u8; 65] =
					slice.try_into().expect("Signature should be 65 bytes long");
				SignatureWithAuthorityIndex { index: index as u32, signature: signature_array }
			})
		})
		.collect::<Vec<_>>();

	let current_authorities = prover.beefy_authorities(Some(block_hash)).await.unwrap();
	let authority_address_hashes =
		hash_authority_addresses(current_authorities.into_iter().map(|x| x.encode()).collect())
			.unwrap();

	let authority_indices = signatures.iter().map(|x| x.index as usize).collect::<Vec<_>>();
	let authority_tree = MerkleTree::<MerkleHasher>::from_leaves(&authority_address_hashes);
	let authority_proof_hashes = authority_tree.proof(&authority_indices).proof_hashes().to_vec();

	let signed_commitment = beefy_verifier_primitives::SignedCommitment {
		commitment: signed_commitment_raw.commitment.clone(),
		signatures,
	};

	let mmr = MmrProof {
		signed_commitment,
		latest_mmr_leaf: latest_leaf.clone(),
		mmr_proof: mmr_leaf_proof.clone(),
		authority_proof: authority_proof_hashes,
	};

	println!("Generating the parachain proof");
	let heads = paras_parachains(
		&prover.relay_rpc,
		Some(
			H256::decode(&mut &*latest_leaf.parent_number_and_hash.1.encode())
				.unwrap()
				.into(),
		),
	)
	.await
	.unwrap();

	let (parachains, indices): (Vec<_>, Vec<_>) = if !heads.is_empty() {
		let first_head = &heads[0];
		(
			vec![ParachainHeader { header: first_head.1.clone(), index: 0, para_id: first_head.0 }],
			vec![0],
		)
	} else {
		(vec![], vec![])
	};

	let leaves = heads.iter().map(|pair| keccak_256(&pair.encode())).collect::<Vec<_>>();
	let parachain_tree = MerkleTree::<MerkleHasher>::from_leaves(&leaves);
	let proof = parachain_tree.proof(&indices).proof_hashes().to_vec();
	let parachain_proof = ParachainProof { parachains, proof, total_leaves: leaves.len() as u32 };

	println!("Assembling final proof for verification");
	let consensus_proof = ConsensusMessage { mmr, parachain: parachain_proof };

	// secp256k1_ecdsa_recover is a host function; run inside test externalities.
	let result = sp_io::TestExternalities::default()
		.execute_with(|| verify_consensus::<TestHost>(trusted_state, consensus_proof));

	assert!(result.is_ok(), "Consensus verification failed: {:?}", result.err());

	println!("Successfully verified beefy justification for block #{}", block_number);
}

/// Prints the SCALE-encoded `ConsensusState` and SP1 `Sp1BeefyProof` wire bytes
/// (prefixed with `PROOF_TYPE_SP1`) for the fixture used by
/// `test_sp1_verify_consensus_accepts_solidity_fixture`. Run with:
///
///   cargo test -p beefy-verifier --lib dump_sp1_fixture_scale_bytes -- --nocapture --ignored
///
/// Output is copied into `pallet-beefy-consensus-proofs`'s benchmark to avoid
/// pulling solidity-abi (std-only) into the wasm runtime build.
#[test]
#[ignore]
fn dump_sp1_fixture_scale_bytes() {
	use alloy_sol_types::{SolType, SolValue, sol};
	use beefy_verifier_primitives::{ConsensusState, PROOF_TYPE_SP1, Sp1BeefyProof};
	use ismp_abi::{
		ecdsa_beefy::Beefy::BeefyConsensusState,
		sp1_beefy::SP1Beefy::{MiniCommitment, ParachainHeader, PartialBeefyMmrLeaf},
	};

	let state_bytes = hex!(
		"0000000000000000000000000000000000000000000000000000000001d6792200000000000000000000000000000000000000000000000000000000012a531800000000000000000000000000000000000000000000000000000000000012750000000000000000000000000000000000000000000000000000000000000257a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f4900000000000000000000000000000000000000000000000000000000000012760000000000000000000000000000000000000000000000000000000000000257a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f49"
	);
	let proof_bytes = hex!(
		"0000000000000000000000000000000000000000000000000000000001d6792a000000000000000000000000000000000000000000000000000000000000127500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001d67929e1dbc67b9da4b90227fb3dc2e7ffdce4e120d583502399e4bd083c02651ca5eb00000000000000000000000000000000000000000000000000000000000012760000000000000000000000000000000000000000000000000000000000000257a7161e52f2f4249039441385a41c6c8e36207a9b6a65d9bfae4272156ec31f4963bc2eb07f9c83afe64eb8815b626cd0a7d2a1bbb4630a44a1896af297d0135d00000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000340000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000d2700000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000139739e9bd7f1addf87db9b6a762bd0e1713baa895c3b82b4595080e5ba02fb5b3cf2915702b49122c32b822e6a11384074d8902d5ea5f79c7cb0d7804e49501b8b532298f49e38d3f7140ce1ba61c243152e4e380b37eb628e08d5270d8b2c5e4ebedd84bb14066175726120fbc4d208000000000452505352902a869d4e00b3bb93f1e88e41a2b5f51fc637626b4ce1da15749ef2d79de4797a9ae459070449534d50010118a13886ac93d163a1d22cdef94e018eba5189424a66b7bd03a5ac232beb46bf08b0f9d2b979fff833d7e21a64a5183c61e2630c0b452236baba3c1b4ff41821044953544d20ca3be169000000000561757261010152d45dea4dcf058b0610e12981e0e4c97ad153f26481510c0b78beedf1848b4dd2abd37b8c6b800b72fa12199898eca7651471b49e38d6167a84fb6e2df7c7840000000000000000000000000000000000000000000000000000000000000000000000000001644388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f2535200000000000000000000000000000000000000000000000000000000000000002ac5e596c552ee76353c176f0870e47a0aa765ceafc4c65b03dbf434e27fa9062f185bdc40f7aae982c1c8c6b766dd491a1e1cd60128efbc58da965e5be96320287f4ce1b04538f0c8287c8eff096c36df67dc17970032546c9b3d4dd5510c5c25e880e13469e1e1aca1b41c367f2ecf04da65f7602fb53ec212b03d0148157b2cd9a79a9779f350d240e6d4c980848302fca8c7447c5fa7ac8d3c6eefcd0c640acff8b27ea316db978652553e3d054765094cf0dab6085a616489cdb973c42b258e22f346ac3ceb3e2e6750c37dad1f98f6ca15d1f70659343caa52dbbcad150b75dd2dcf0ba0a664ea4605b291df54ab1aa5b4c55034b9425ba29cc87eca7b00000000000000000000000000000000000000000000000000000000"
	);

	let sol_state =
		<BeefyConsensusState as SolValue>::abi_decode(&state_bytes).expect("decode state");
	let trusted: ConsensusState = sol_state.into();
	let trusted_scale = trusted.encode();

	// The solidity side encodes the SP1 proof as a tuple of top-level params
	// (matches `abi.decode(proof, (MiniCommitment, PartialBeefyMmrLeaf,
	// ParachainHeader[], bytes))` in SP1Beefy.sol). Decode as a sequence, not a
	// struct, and assemble `Sp1BeefyProof` by hand.
	type ProofTuple = sol! { (MiniCommitment, PartialBeefyMmrLeaf, ParachainHeader[], bytes) };
	let (commitment, leaf, headers, plonk_proof) =
		<ProofTuple as SolType>::abi_decode_sequence(&proof_bytes).expect("decode proof tuple");
	let sp1_proof = Sp1BeefyProof {
		block_number: commitment.blockNumber.try_into().expect("block number out of bounds"),
		validator_set_id: commitment
			.validatorSetId
			.try_into()
			.expect("validator set id out of bounds"),
		mmr_leaf: leaf.into(),
		headers: headers.into_iter().map(Into::into).collect(),
		proof: plonk_proof.to_vec(),
		// REGEN: fixtures predate the committed-nonce public input; regenerate with the
		// rebuilt ELF/vkey and set this to the fixture's committed nonce.
		nonce: Default::default(),
	};

	let mut wire = vec![PROOF_TYPE_SP1];
	sp1_proof.encode_to(&mut wire);

	println!("TRUSTED_STATE_SCALE_HEX_LEN = {}", trusted_scale.len());
	println!("TRUSTED_STATE_SCALE_HEX = \"{}\"", hex::encode(&trusted_scale));
	println!("WIRE_PROOF_HEX_LEN = {}", wire.len());
	println!("WIRE_PROOF_HEX = \"{}\"", hex::encode(&wire));
}

/// One-off SP1 verifier smoke test using the Groth16 fixture produced by
/// `zk-beefy::tests::test_sp1_beefy` (live tesseract-prover run),
/// also consumed by `SP1BeefyForkTest` in `evm/tests/foundry/`. Regenerate via:
///
/// ```text
/// cargo test --release -p zk-beefy --lib tests::test_sp1_beefy -- --ignored --nocapture
/// ```
///
/// The test decodes the solidity-ABI-encoded `BeefyConsensusState` and tuple-encoded
/// `SP1BeefyProof` fields using the bindings in `ismp-abi`, converts them via
/// the existing `From` impls in `evm/abi/src/conversions.rs`, and runs them through
/// our Rust [`crate::sp1::verify_sp1_consensus`].
#[test]
fn test_sp1_verify_consensus_accepts_solidity_fixture() {
	use alloy_sol_types::{SolType, SolValue, sol};
	use beefy_verifier_primitives::{ConsensusState, Sp1BeefyProof};
	use ismp_abi::{
		ecdsa_beefy::Beefy::BeefyConsensusState,
		sp1_beefy::SP1Beefy::{MiniCommitment, ParachainHeader, PartialBeefyMmrLeaf},
	};

	// Fixture: evm/tests/foundry/fixtures/sp1_beefy_fixture.json (committed nonce = Bob).
	// Generated against SP1 program vkey 0x007d1720 (v1.1.0, committed nonce = Bob).
	let state_bytes = hex!(
		"0000000000000000000000000000000000000000000000000000000001df6bd100000000000000000000000000000000000000000000000000000000012a5318000000000000000000000000000000000000000000000000000000000000136a00000000000000000000000000000000000000000000000000000000000002582cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc741000000000000000000000000000000000000000000000000000000000000136b00000000000000000000000000000000000000000000000000000000000002582cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc741"
	);
	let proof_bytes = hex!(
		"0000000000000000000000000000000000000000000000000000000001df6bd9000000000000000000000000000000000000000000000000000000000000136a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001df6bd8b06c82d25b39550a06ab64cf89004fce1f913b27190ab108320812295591fa89000000000000000000000000000000000000000000000000000000000000136b00000000000000000000000000000000000000000000000000000000000002582cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc741ed96e512661b155ef81e590ca5ad1bacf2ccce06e7e822ca521daa71efb4ff91000000000000000000000000000000000000000000000000000000000000018000000000000000000000000000000000000000000000000000000000000003608eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000d2700000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000139557ed2657ce1e450327c6006e17e64425bb2154a7e6a55514e3d37fc7fd5d9884697790283bf3e632f74afab019365ed730a6deb0bc3e70bb229635fcf769d28febdf61f520234bde985bfdd18c0b04baf50ddae8f48860b9546184888ce393886265396140661757261209441d70800000000045250535290b43da1ab3f398f7008b0bd1374925ba70102ff77f33c1acce60e98a4e40fb8cf56af7d070449534d500101af5c78d7d0420a25ee6b68dc946d9919da3799b923cff420aa27ab1b646f355794a54ad343c04bc95bb013d63caecc98e97b65edb739cacc2c6e97f7d5aba5c9044953544d20f612176a000000000561757261010162f8da99803bec263b758f801ed06717d9af6178ac74e3c5ecba6e9cf6ab5c33e4daf9b75a18945bc3bb2221e1aceef06b67c87a0c6756872789dffbcd747b810000000000000000000000000000000000000000000000000000000000000000000000000001644388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f2535200000000000000000000000000000000000000000000000000000000000000002607774c88245bcad79f2414d5829f9b61771e86fb92366a5d224ba9a42cea9b16e91bc69ca90c8f455e8973ca2d522e460b371e95a8cdd298e00558bb25e39c1046ac2fb71dfc17f57e39ae5309c9522d97cc181e836aa679be1c168e25180b1556c8c21b6537ff21a57ecb73d497301f5fc9fe8f8d312d03720e401684da5e16440efe811bc61f2bfa210171efc4745d1b7461ce5593c8bcd9a9b2f6489f6e0c8cfbf59f1489e3e9a93143084cd57df0bf06cbf1fce9a7098154abcfb984a90fd4708053142c7043ce767492db2f5c0055f8791ef0cfb31173e9cac6ab47600e3953c2efe616bbd960b6048026dd1e0bb8bba4a29f3ccdb5ac21ce3899751300000000000000000000000000000000000000000000000000000000"
	);

	let sol_state =
		<BeefyConsensusState as SolValue>::abi_decode(&state_bytes).expect("decode state");
	let trusted: ConsensusState = sol_state.into();

	// Proof payload matches SP1Beefy.sol:verifyConsensus's `abi.decode(...)` call:
	// a sequence of four top-level types, not a struct wrapper.
	type ProofTuple =
		sol! { (MiniCommitment, PartialBeefyMmrLeaf, ParachainHeader[], bytes, bytes32) };
	let (commitment, leaf, headers, plonk_proof, nonce) =
		<ProofTuple as SolType>::abi_decode_sequence(&proof_bytes).expect("decode proof tuple");
	let sp1_proof = Sp1BeefyProof {
		block_number: commitment.blockNumber.try_into().expect("block number out of bounds"),
		validator_set_id: commitment
			.validatorSetId
			.try_into()
			.expect("validator set id out of bounds"),
		mmr_leaf: leaf.into(),
		headers: headers.into_iter().map(Into::into).collect(),
		proof: plonk_proof.to_vec(),
		nonce: H256(nonce.0),
	};

	// Mainnet SP1Beefy verification key — matches `SP1_VERIFICATION_KEY` in
	// `0x82582f85cf370adCB61D97dab3068c0C4102Ccb6`.
	let vkey_hash = "0x007d1720c695842ed647a1a72e981751f9b5e26fc5ca038523b23430a1292f08";
	let result = sp_io::TestExternalities::default().execute_with(|| {
		crate::sp1::verify_sp1_consensus::<TestHost>(trusted.clone(), sp1_proof, vkey_hash)
	});

	let (new_state_bytes, verified_headers) =
		result.expect("SP1 consensus verification should succeed against the solidity fixture");

	let new_state = ConsensusState::decode(&mut &*new_state_bytes).unwrap();
	assert!(
		new_state.latest_beefy_height > trusted.latest_beefy_height,
		"latest_beefy_height should advance"
	);
	assert_eq!(verified_headers.len(), 1, "fixture contains one parachain header");
}

fn authority_set(id: ValidatorSetId, len: u32) -> BeefyAuthoritySet<H256> {
	BeefyAuthoritySet { id, len, keyset_commitment: H256::zero() }
}

fn dummy_mmr_proof(commitment: Commitment<u32>, signature_count: u32) -> MmrProof {
	let signatures = (0..signature_count)
		.map(|index| SignatureWithAuthorityIndex { index, signature: [0u8; 65] })
		.collect();
	MmrProof {
		signed_commitment: SignedCommitment { commitment, signatures },
		latest_mmr_leaf: MmrLeaf {
			version: MmrLeafVersion::new(0, 0),
			parent_number_and_hash: (0, H256::zero()),
			beefy_next_authority_set: BeefyNextAuthoritySet {
				id: 0,
				len: 0,
				keyset_commitment: H256::zero(),
			},
			leaf_extra: H256::zero(),
		},
		mmr_proof: LeafProof { leaf_indices: vec![0], leaf_count: 1, items: vec![] },
		authority_proof: vec![],
	}
}

// When current and next authority sets diverge in size, the threshold must be
// judged against the set named by `validator_set_id` rather than passing if
// either set's threshold is met. This mirrors the Solidity verifier and rules
// out a commitment that only clears the smaller set's bar.
#[test]
fn rejects_sub_supermajority_from_named_authority_set() {
	const CURRENT_SET_ID: ValidatorSetId = 42;
	const NEXT_SET_ID: ValidatorSetId = 43;

	let trusted_state = ConsensusState {
		latest_beefy_height: 0,
		beefy_activation_block: 0,
		mmr_root_hash: H256::zero(),
		current_authorities: authority_set(CURRENT_SET_ID, 100),
		next_authorities: authority_set(NEXT_SET_ID, 3),
	};

	let payload = Payload::from_single_entry(*b"mh", MmrRootHash::zero().0.to_vec());
	let commitment = Commitment { payload, block_number: 1, validator_set_id: CURRENT_SET_ID };

	let mmr = dummy_mmr_proof(commitment, 3);

	let result = sp_io::TestExternalities::default()
		.execute_with(|| verify_mmr_update_proof::<TestHost>(trusted_state, mmr));

	assert!(matches!(result, Err(Error::SuperMajorityRequired)));
}

// SP1 proves that the leaf is *in* the mmr, not that it is the latest leaf, and an mmr is
// append-only — so every historical leaf also proves against the commitment's root. Accepting
// one would advance `latest_beefy_height` while replaying an old `beefy_next_authority_set`,
// suppressing the rotation and stranding the client on a set the relay chain has retired.
#[test]
fn rejects_sp1_proof_carrying_a_stale_mmr_leaf() {
	use beefy_verifier_primitives::Sp1BeefyProof;

	const SET_ID: ValidatorSetId = 42;
	const BLOCK_NUMBER: u32 = 1_000;
	// Mainnet SP1Beefy verification key, as in the fixture test above.
	const VKEY: &str = "0x007d1720c695842ed647a1a72e981751f9b5e26fc5ca038523b23430a1292f08";

	let trusted_state = ConsensusState {
		latest_beefy_height: BLOCK_NUMBER - 1,
		beefy_activation_block: 0,
		mmr_root_hash: H256::zero(),
		current_authorities: authority_set(SET_ID, 100),
		next_authorities: authority_set(SET_ID + 1, 100),
	};

	let mut proof = Sp1BeefyProof {
		block_number: BLOCK_NUMBER,
		validator_set_id: SET_ID,
		mmr_leaf: MmrLeaf {
			version: MmrLeafVersion::new(0, 0),
			parent_number_and_hash: (BLOCK_NUMBER - 1, H256::zero()),
			beefy_next_authority_set: BeefyNextAuthoritySet {
				id: SET_ID + 1,
				len: 100,
				keyset_commitment: H256::zero(),
			},
			leaf_extra: H256::zero(),
		},
		headers: vec![],
		proof: vec![],
		nonce: H256::zero(),
	};

	// The leaf appended at `BLOCK_NUMBER` clears the freshness check and is only rejected
	// later, by the Groth16 verifier — so the check discriminates on leaf freshness alone.
	let fresh = sp_io::TestExternalities::default().execute_with(|| {
		crate::sp1::verify_sp1_consensus::<TestHost>(trusted_state.clone(), proof.clone(), VKEY)
	});
	assert!(matches!(fresh, Err(Error::Sp1VerificationFailed)), "got {fresh:?}");

	// Swap in a leaf from an earlier block, as an attacker replaying a historical leaf would.
	proof.mmr_leaf.parent_number_and_hash.0 = BLOCK_NUMBER - 500;
	let stale = sp_io::TestExternalities::default()
		.execute_with(|| crate::sp1::verify_sp1_consensus::<TestHost>(trusted_state, proof, VKEY));
	assert!(matches!(stale, Err(Error::StaleMmrLeaf { .. })), "got {stale:?}");
}

/// End-to-end verification against a relay chain whose BEEFY authorities use the paired
/// (ECDSA, BLS12-381) `ecdsa_bls_crypto` key type. Requires the `beefy-prover/bls` feature, which
/// makes the prover keep the ECDSA half of each 177-byte paired signature and authority key. The
/// verifier itself is unchanged: this is "Option A" — a BLS-BEEFY relay is verified through the
/// existing ECDSA path.
///
///   RELAY_WS_URL=ws://127.0.0.1:9977 \
///     cargo test -p beefy-verifier --features bls test_verify_consensus_bls -- --ignored
/// --nocapture
#[cfg(feature = "bls")]
#[tokio::test]
#[ignore]
async fn test_verify_consensus_bls() {
	let max_rpc_payload_size = 15 * 1024 * 1024;
	let relay_ws_url = std::env::var("RELAY_WS_URL").expect("RELAY_WS_URL must be set");

	let (relay_client, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, max_rpc_payload_size)
			.await
			.unwrap();
	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());
	// Relay-only: point the "para" client at the same relay; `para_ids` is empty so no parachain
	// headers are proven (our local relay has no registered parachains).
	let (para_client, para_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, max_rpc_payload_size)
			.await
			.unwrap();
	let para_rpc = LegacyRpcMethods::<PolkadotConfig>::new(para_rpc_client.clone());

	let prover = Prover {
		beefy_activation_block: 0,
		relay: relay_client,
		relay_rpc: relay_rpc.clone(),
		relay_rpc_client: relay_rpc_client.clone(),
		para: para_client,
		para_rpc,
		para_rpc_client,
		para_ids: vec![],
		query_batch_size: Some(100),
	};

	let engine_id = polkadot_sdk::sp_consensus_beefy::BEEFY_ENGINE_ID;
	let latest: H256 =
		relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await.unwrap();

	// Walk back to the previous BEEFY-justified block to seed the trusted state.
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
			.map(|js| js.iter().any(|j| j.0 == engine_id))
			.unwrap_or(false)
		{
			previous = parent;
			break;
		}
		cursor = parent;
	}
	assert!(!previous.is_zero(), "no previous beefy block found");

	// Initial trusted state via the prover — exercises the folded BLS justification decode.
	let trusted_state = prover.get_initial_consensus_state(Some(previous)).await.unwrap();

	// Latest justification -> signed commitment with ECDSA-half signatures (folded BLS decode).
	let latest_block = relay_rpc.chain_get_block(Some(latest.into())).await.unwrap().unwrap();
	let latest_just = latest_block
		.justifications
		.expect("latest beefy block must have justifications")
		.into_iter()
		.find_map(|j| (j.0 == engine_id).then_some(j.1))
		.expect("latest beefy block must have a beefy justification");
	let signed = beefy_prover::relay::decode_beefy_justification(&latest_just).unwrap();
	let block_number = signed.commitment.block_number;
	let signed_count = signed.signatures.iter().filter(|s| s.is_some()).count();

	let signatures = signed
		.signatures
		.iter()
		.enumerate()
		.filter_map(|(index, s)| {
			s.as_ref().map(|sig| {
				let slice: &[u8] = sig.as_ref();
				let signature: [u8; 65] = slice.try_into().expect("ecdsa half is 65 bytes");
				SignatureWithAuthorityIndex { index: index as u32, signature }
			})
		})
		.collect::<Vec<_>>();

	let (mmr_leaf_proof, latest_leaf) =
		fetch_mmr_proof(&prover.relay_rpc, block_number, None).await.unwrap();

	// Folded BLS-aware authorities: the ECDSA halves of the paired keys.
	let current_authorities = prover.beefy_authorities(Some(latest)).await.unwrap();
	let authority_address_hashes =
		hash_authority_addresses(current_authorities.into_iter().map(|x| x.encode()).collect())
			.unwrap();

	let authority_indices = signatures.iter().map(|x| x.index as usize).collect::<Vec<_>>();
	let authority_tree = MerkleTree::<MerkleHasher>::from_leaves(&authority_address_hashes);
	let authority_proof = authority_tree.proof(&authority_indices).proof_hashes().to_vec();

	let signed_commitment = SignedCommitment { commitment: signed.commitment.clone(), signatures };
	let mmr = MmrProof {
		signed_commitment,
		latest_mmr_leaf: latest_leaf.clone(),
		mmr_proof: mmr_leaf_proof,
		authority_proof,
	};
	// Relay-only: `verify_parachain_headers` short-circuits to `Ok(vec![])` on empty parachains.
	let parachain_proof = ParachainProof { parachains: vec![], proof: vec![], total_leaves: 0 };
	let consensus_proof = ConsensusMessage { mmr, parachain: parachain_proof };

	let result = sp_io::TestExternalities::default()
		.execute_with(|| verify_consensus::<TestHost>(trusted_state, consensus_proof));

	assert!(result.is_ok(), "BLS BEEFY verification failed: {:?}", result.err());
	println!(
		"BLS BEEFY verify OK: verified {signed_count} paired signatures for beefy block #{block_number}"
	);
}

/// Option B prototype: actually verify the BLS signatures, aggregated into a single pairing check.
///
/// This reads the validators' paired keys straight from `Beefy.Authorities` and the paired
/// signatures from the latest justification, extracts the BLS halves (G2 public key, G1 signature),
/// and does two things:
///   1. Per-signature Chaum-Pedersen verification via w3f-bls (proves our byte extraction and the
///      message hash-to-curve are correct).
///   2. The aggregate pairing check `e(gen, sum(sig)) == e(sum(pubkey), H(commitment))` by summing
///      the signer G1 signatures and G2 public keys and verifying the sums as one signature.
///
/// If step 2 passes, plain aggregation over the on-chain-committed key set is sound and we do not
/// need delinearization, which makes the eventual Solidity/EIP-2537 path much simpler.
///
///   RELAY_WS_URL=ws://127.0.0.1:9977 \
///     cargo test -p beefy-verifier --features bls test_bls_aggregate_verify -- --ignored
/// --nocapture
#[cfg(feature = "bls")]
#[tokio::test]
#[ignore]
async fn test_bls_aggregate_verify() {
	use w3f_bls::{
		DoublePublicKey, DoubleSignature, Message, PublicKey, SerializableToBytes, Signature,
		TinyBLS381,
	};

	/// A 177-byte paired signature exactly as SCALE-encoded on the wire.
	#[derive(Clone)]
	struct Sig177([u8; 177]);
	impl codec::Decode for Sig177 {
		fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
			let mut bytes = [0u8; 177];
			input.read(&mut bytes)?;
			Ok(Sig177(bytes))
		}
	}

	let max_rpc_payload_size = 15 * 1024 * 1024;
	let relay_ws_url = std::env::var("RELAY_WS_URL").expect("RELAY_WS_URL must be set");
	let (_relay_client, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, max_rpc_payload_size)
			.await
			.unwrap();
	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());
	let engine_id = polkadot_sdk::sp_consensus_beefy::BEEFY_ENGINE_ID;

	let latest: H256 =
		relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await.unwrap();
	let latest_block = relay_rpc.chain_get_block(Some(latest.into())).await.unwrap().unwrap();
	let latest_just = latest_block
		.justifications
		.expect("justifications")
		.into_iter()
		.find_map(|j| (j.0 == engine_id).then_some(j.1))
		.expect("beefy justification");

	let VersionedFinalityProof::V1(sc) =
		VersionedFinalityProof::<u32, Sig177>::decode(&mut &*latest_just).unwrap();
	let commitment_encoded = sc.commitment.encode();
	// The BLS half signs the SCALE-encoded commitment with an empty context (see sp-core
	// `bls381::Pair::sign` -> `Message::new(b"", message)`), hashed to G1 by w3f-bls.
	let message = Message::new(b"", &commitment_encoded);

	// Validator keys are the 177-byte paired keys; the BLS DoublePublicKey is bytes [33..177].
	let raw_auth = relay_rpc
		.state_get_storage(beefy_prover::BEEFY_AUTHORITIES.as_slice(), Some(latest.into()))
		.await
		.unwrap()
		.expect("beefy authorities storage");
	let authorities = Vec::<[u8; 177]>::decode(&mut raw_auth.as_ref()).unwrap();

	let mut agg_sig: Option<<TinyBLS381 as w3f_bls::EngineBLS>::SignatureGroup> = None;
	let mut agg_pub: Option<<TinyBLS381 as w3f_bls::EngineBLS>::PublicKeyGroup> = None;
	let mut count = 0u32;

	for (i, maybe_sig) in sc.signatures.iter().enumerate() {
		let Some(sig) = maybe_sig else { continue };
		// DoublePublicKey = G1(48) || G2(96); it lives at bytes [33..177] of the paired key.
		let dpk = DoublePublicKey::<TinyBLS381>::from_bytes(&authorities[i][33..177])
			.expect("double public key");
		// DoubleSignature = G1 sig(48) || SchnorrProof(64); at bytes [65..177] of the paired sig.
		let dsig =
			DoubleSignature::<TinyBLS381>::from_bytes(&sig.0[65..177]).expect("double signature");

		// 1. Per-signature Chaum-Pedersen verification.
		assert!(dpk.verify(&message, &dsig), "per-signature BLS verify failed for validator {i}");

		// 2. Accumulate for the aggregate pairing check.
		agg_sig = Some(agg_sig.map_or(dsig.0, |acc| acc + dsig.0));
		agg_pub = Some(agg_pub.map_or(dpk.1, |acc| acc + dpk.1));
		count += 1;
	}

	let agg_sig = agg_sig.expect("at least one signer");
	let agg_pub = agg_pub.expect("at least one signer");

	let aggregate_ok =
		Signature::<TinyBLS381>(agg_sig).verify(&message, &PublicKey::<TinyBLS381>(agg_pub));

	assert!(aggregate_ok, "aggregate BLS pairing check failed");
	println!(
		"Aggregate BLS verify OK: {count} BLS signatures aggregated into ONE pairing check; \
		 per-signature Chaum-Pedersen also verified. Plain aggregation over the committed key set \
		 is sound (no delinearization needed)."
	);
}

/// Phase 1 (trustless): the full aggregate-BLS verification flow a light client would run.
///
/// Unlike `test_bls_aggregate_verify` (which trusts the keys read from storage), this proves the
/// signing keys against the on-chain keyset commitment. The runtime's `BeefyBls381G2ToKeysetLeaf`
/// converter commits `keccak(g2_pubkey)` leaves, so the flow is:
///   1. build the merkle tree of all validators' G2 keys and check its root equals the on-chain
///      `keyset_commitment` (confirms the runtime commits G2 keys as expected),
///   2. check the signer count meets the >2/3 threshold,
///   3. prove the signers' keys are committed (merkle multi-proof at the signer indices),
///   4. aggregate the signers' G2 keys and G1 signatures and verify one pairing check.
///
///   RELAY_WS_URL=ws://127.0.0.1:9977 \
///     cargo test -p beefy-verifier --features bls test_bls_trustless_verify -- --ignored
/// --nocapture
#[cfg(feature = "bls")]
#[tokio::test]
#[ignore]
async fn test_bls_trustless_verify() {
	use beefy_prover::rs_merkle::MerkleProof;
	use w3f_bls::{Message, PublicKey, SerializableToBytes, Signature, TinyBLS381};

	#[derive(Clone)]
	struct Sig177([u8; 177]);
	impl codec::Decode for Sig177 {
		fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
			let mut bytes = [0u8; 177];
			input.read(&mut bytes)?;
			Ok(Sig177(bytes))
		}
	}

	let max_rpc_payload_size = 15 * 1024 * 1024;
	let relay_ws_url = std::env::var("RELAY_WS_URL").expect("RELAY_WS_URL must be set");
	let (_relay_client, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, max_rpc_payload_size)
			.await
			.unwrap();
	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());
	let engine_id = polkadot_sdk::sp_consensus_beefy::BEEFY_ENGINE_ID;

	// Latest justification -> commitment + per-validator signature slots.
	let latest: H256 =
		relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await.unwrap();
	let latest_block = relay_rpc.chain_get_block(Some(latest.into())).await.unwrap().unwrap();
	let latest_just = latest_block
		.justifications
		.expect("justifications")
		.into_iter()
		.find_map(|j| (j.0 == engine_id).then_some(j.1))
		.expect("beefy justification");
	let VersionedFinalityProof::V1(sc) =
		VersionedFinalityProof::<u32, Sig177>::decode(&mut &*latest_just).unwrap();
	let commitment_encoded = sc.commitment.encode();
	let message = Message::new(b"", &commitment_encoded);

	// All validators' G2 public keys, in authority-set order, from `Beefy.Authorities`.
	let raw_auth = relay_rpc
		.state_get_storage(beefy_prover::BEEFY_AUTHORITIES.as_slice(), Some(latest.into()))
		.await
		.unwrap()
		.expect("beefy authorities");
	let paired = Vec::<[u8; 177]>::decode(&mut raw_auth.as_ref()).unwrap();
	let g2_keys: Vec<[u8; 96]> = paired
		.iter()
		.map(|k| {
			let mut g2 = [0u8; 96];
			g2.copy_from_slice(&k[81..177]);
			g2
		})
		.collect();
	let total = g2_keys.len();

	// The on-chain keyset commitment (root of keccak(g2_pubkey) leaves) from the MmrLeaf pallet.
	let raw_set = relay_rpc
		.state_get_storage(
			beefy_prover::BEEFY_MMR_LEAF_BEEFY_AUTHORITIES.as_slice(),
			Some(latest.into()),
		)
		.await
		.unwrap()
		.expect("mmr leaf beefy authorities");
	let authority_set = BeefyAuthoritySet::<H256>::decode(&mut raw_set.as_ref()).unwrap();
	let keyset_commitment: [u8; 32] = authority_set.keyset_commitment.into();

	// 1. Rebuild the tree and confirm its root matches the on-chain commitment.
	let leaves: Vec<[u8; 32]> = g2_keys.iter().map(|k| keccak_256(k)).collect();
	let tree = MerkleTree::<MerkleHasher>::from_leaves(&leaves);
	assert_eq!(
		tree.root().expect("root"),
		keyset_commitment,
		"rebuilt keyset root does not match on-chain keyset_commitment (runtime converter mismatch)"
	);

	// 2. Signer set from the bitfield + the >2/3 threshold check.
	let signer_indices: Vec<usize> = sc
		.signatures
		.iter()
		.enumerate()
		.filter_map(|(i, s)| s.as_ref().map(|_| i))
		.collect();
	assert!(
		signer_indices.len() * 3 > total * 2,
		"below supermajority: {} of {}",
		signer_indices.len(),
		total
	);

	// 3. Prove the signers' keys are committed (merkle multi-proof).
	let signer_leaves: Vec<[u8; 32]> = signer_indices.iter().map(|&i| leaves[i]).collect();
	let proof = tree.proof(&signer_indices);
	assert!(
		MerkleProof::<MerkleHasher>::new(proof.proof_hashes().to_vec()).verify(
			keyset_commitment,
			&signer_indices,
			&signer_leaves,
			total,
		),
		"merkle multi-proof of signer keys against keyset_commitment failed"
	);

	// 4. Aggregate the signers' G2 keys and G1 signatures, one pairing check.
	let mut agg_sig: Option<<TinyBLS381 as w3f_bls::EngineBLS>::SignatureGroup> = None;
	let mut agg_pub: Option<<TinyBLS381 as w3f_bls::EngineBLS>::PublicKeyGroup> = None;
	for &i in &signer_indices {
		let sig = sc.signatures[i].as_ref().unwrap();
		let g1 = Signature::<TinyBLS381>::from_bytes(&sig.0[65..113]).expect("g1 signature");
		let pk = PublicKey::<TinyBLS381>::from_bytes(&g2_keys[i]).expect("g2 public key");
		agg_sig = Some(agg_sig.map_or(g1.0, |acc| acc + g1.0));
		agg_pub = Some(agg_pub.map_or(pk.0, |acc| acc + pk.0));
	}
	let aggregate_ok = Signature::<TinyBLS381>(agg_sig.unwrap())
		.verify(&message, &PublicKey::<TinyBLS381>(agg_pub.unwrap()));
	assert!(aggregate_ok, "aggregate BLS pairing check failed");

	println!(
		"Trustless aggregate BLS verify OK: {}/{} signers proven against the on-chain \
		 keyset_commitment (merkle multi-proof), >2/3 threshold met, aggregated into ONE pairing \
		 check.",
		signer_indices.len(),
		total
	);
}

/// Pins down exactly how BEEFY's BLS half hashes a commitment onto the signature curve, and emits
/// a test vector for the Solidity/EIP-2537 implementation to be checked against.
///
/// This is the make-or-break detail of the EVM path. `w3f-bls` does *not* use the IETF ciphersuite
/// string as the domain separation tag the way a textbook implementation would. It uses a one-byte
/// DST of `0x01`, and prepends the ciphersuite string to the message instead:
///
/// ```text
/// suite    = "BLS_SIG_" || "BLS12381" || "G1" || "_XMD:SHA-256_SSWU_RO_" || "NUL_"
/// preimage = suite || context || message          // context is empty for BEEFY
/// point    = hash_to_curve(preimage, DST = 0x01)  // expand_message_xmd<SHA-256>, WB/SSWU map
/// ```
///
/// Everything after that composition is standard RFC 9380, which is what the EIP-2537
/// `MAP_FP_TO_G1` precompile implements, so the contract has to reproduce the composition and the
/// `expand_message_xmd` step and can lean on precompiles for the rest.
///
///   cargo test -p beefy-verifier --features bls bls_hash_to_curve_vector -- --nocapture
#[cfg(feature = "bls")]
#[test]
fn bls_hash_to_curve_vector() {
	use ark_bls12_381::{Fq, G1Affine, g1::Config as G1Config};
	use ark_ec::{
		AffineRepr, CurveGroup,
		hashing::{HashToCurve, curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher},
	};
	use ark_ff::{
		BigInteger, PrimeField,
		field_hashers::{DefaultFieldHasher, HashToField},
	};
	use w3f_bls::{Message, TinyBLS381};

	// Any byte string stands in for a SCALE-encoded commitment here; the composition is what is
	// being pinned down, and it does not depend on the contents.
	let message = b"beefy-bls-hash-to-curve-vector";

	let suite = [
		b"BLS_SIG_".as_ref(),
		b"BLS12381".as_ref(),
		b"G1".as_ref(),
		b"_XMD:SHA-256_SSWU_RO_".as_ref(),
		b"NUL_".as_ref(),
	]
	.concat();
	assert_eq!(
		suite.as_slice(),
		b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_".as_ref(),
		"ciphersuite string drifted from what w3f-bls composes"
	);

	// context is empty for BEEFY, matching sp-core's `bls381::Pair::sign`.
	let preimage = [suite.as_slice(), b"".as_ref(), message.as_ref()].concat();

	// Rebuild the hasher from the documented parameters rather than going through w3f-bls, then
	// check it lands on the same point. If this assert holds, the recipe above is the whole story
	// and a Solidity implementation has everything it needs.
	let hasher = MapToCurveBasedHasher::<
		ark_ec::short_weierstrass::Projective<G1Config>,
		DefaultFieldHasher<sha2::Sha256, 128>,
		WBMap<G1Config>,
	>::new(&[1u8])
	.expect("hasher construction");
	let reconstructed: G1Affine = hasher.hash(&preimage).expect("hash to curve");

	let expected = Message::new(b"", message).hash_to_signature_curve::<TinyBLS381>().into_affine();

	assert_eq!(
		reconstructed, expected,
		"reconstructing the hash-to-curve from DST=0x01 and suite-prefixed message did not match \
		 w3f-bls; the Solidity recipe would be wrong"
	);

	let (x, y) = expected.xy().expect("point is not the identity");
	let fq_hex = |value: &Fq| hex::encode(value.into_bigint().to_bytes_be());

	// The two field elements the contract has to derive before it can call MAP_FP_TO_G1. This is
	// the only part of the pipeline Solidity implements by hand, so it is the part worth pinning.
	let field_hasher = <DefaultFieldHasher<sha2::Sha256, 128> as HashToField<Fq>>::new(&[1u8]);
	let u: Vec<Fq> = field_hasher.hash_to_field(&preimage, 2);

	println!("=== BEEFY BLS hash-to-curve vector (BLS12-381 G1) ===");
	println!("dst              0x01");
	println!("suite            {}", core::str::from_utf8(&suite).unwrap());
	println!("message          {}", hex::encode(message));
	println!("preimage         {}", hex::encode(&preimage));
	println!("u[0]             {}", fq_hex(&u[0]));
	println!("u[1]             {}", fq_hex(&u[1]));
	println!("point.x          {}", fq_hex(x));
	println!("point.y          {}", fq_hex(y));
	println!();
	println!(
		"Solidity must: expand_message_xmd<SHA-256>(preimage, 0x01, 128) -> 2 field elements,"
	);
	println!("MAP_FP_TO_G1 each, G1_ADD them. Cofactor clearing is linear, so it may be applied");
	println!("per point by the precompile or once at the end without changing the result.");
}

/// The production path end to end: the prover assembles a BLS consensus proof and the verifier
/// accepts it, with no proof-building logic in the test itself.
///
/// `test_bls_trustless_verify` above proves the same thing from first principles, rebuilding the
/// tree by hand so it fails loudly if the runtime's converter ever stops committing G2 keys. This
/// one exercises the API a relayer would actually call.
///
///   RELAY_WS_URL=ws://127.0.0.1:9979 \
///     cargo test -p beefy-verifier --features bls test_bls_consensus_via_prover -- --ignored
/// --nocapture
#[cfg(feature = "bls")]
#[tokio::test]
#[ignore]
async fn test_bls_consensus_via_prover() {
	use beefy_prover::bls::decode_paired_justification;

	let max_rpc_payload_size = 15 * 1024 * 1024;
	let relay_ws_url = std::env::var("RELAY_WS_URL").expect("RELAY_WS_URL must be set");

	let (relay_client, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, max_rpc_payload_size)
			.await
			.unwrap();
	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());
	// Relay-only: the "para" client points at the same relay and `para_ids` is empty, so no
	// parachain headers are proven.
	let (para_client, para_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, max_rpc_payload_size)
			.await
			.unwrap();
	let para_rpc = LegacyRpcMethods::<PolkadotConfig>::new(para_rpc_client.clone());

	let prover = Prover {
		beefy_activation_block: 0,
		relay: relay_client,
		relay_rpc: relay_rpc.clone(),
		relay_rpc_client: relay_rpc_client.clone(),
		para: para_client,
		para_rpc,
		para_rpc_client,
		para_ids: vec![],
		query_batch_size: Some(100),
	};

	let engine_id = polkadot_sdk::sp_consensus_beefy::BEEFY_ENGINE_ID;
	let latest: H256 =
		relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await.unwrap();

	// Seed the trusted state from an earlier BEEFY-justified block, so the proof advances it.
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
			.map(|js| js.iter().any(|j| j.0 == engine_id))
			.unwrap_or(false)
		{
			previous = parent;
			break;
		}
		cursor = parent;
	}
	assert!(!previous.is_zero(), "no previous beefy block found");

	let trusted_state = prover.get_initial_consensus_state(Some(previous)).await.unwrap();
	let trusted_height = trusted_state.latest_beefy_height;

	let latest_block = relay_rpc.chain_get_block(Some(latest.into())).await.unwrap().unwrap();
	let justification = latest_block
		.justifications
		.expect("latest beefy block must have justifications")
		.into_iter()
		.find_map(|j| (j.0 == engine_id).then_some(j.1))
		.expect("latest beefy block must have a beefy justification");

	let signed_commitment = decode_paired_justification(&justification).unwrap();
	let block_number = signed_commitment.commitment.block_number;
	let signer_count = signed_commitment.signatures.iter().filter(|s| s.is_some()).count();

	let proof = prover.bls_consensus_proof(signed_commitment).await.unwrap();
	assert_eq!(proof.mmr.signers.len(), signer_count, "prover dropped signers");

	let result = sp_io::TestExternalities::default()
		.execute_with(|| verify_bls_consensus::<TestHost>(trusted_state, proof));

	let (new_state, _headers) = result.expect("BLS consensus verification failed");
	let new_state = ConsensusState::decode(&mut &new_state[..]).unwrap();

	assert_eq!(new_state.latest_beefy_height, block_number, "height was not advanced");
	assert!(new_state.latest_beefy_height > trusted_height, "state did not move forward");

	println!(
		"BLS consensus verified via the prover API: {signer_count} signers aggregated, \
		 height {trusted_height} -> {block_number}"
	);
}

/// Offline coverage for the aggregate BLS path.
///
/// The BLS integration tests above all need a live relay chain, so none of them run in CI. These
/// build a validator set from deterministic seeds and exercise the verifier directly, including
/// the rejection paths, which is where the interesting behaviour lives.
#[cfg(feature = "bls-crypto")]
mod bls_offline {
	use super::*;
	use beefy_verifier_primitives::{
		BLS_G1_SIGNATURE_LEN, BLS_G2_PUBLIC_KEY_LEN, BlsMmrProof, BlsSigner,
	};
	use w3f_bls::{
		EngineBLS, Message, SecretKeyVT, SerializableToBytes, Signature as BlsSignature, TinyBLS381,
	};

	const SET_ID: ValidatorSetId = 7;
	const BLOCK: u32 = 100;

	type Validator = (SecretKeyVT<TinyBLS381>, [u8; BLS_G2_PUBLIC_KEY_LEN]);

	/// Deterministic validators, so failures reproduce.
	fn validators(count: usize) -> Vec<Validator> {
		(0..count)
			.map(|i| {
				let secret = SecretKeyVT::<TinyBLS381>::from_seed(&[b'v', i as u8]);
				let public = secret.into_public().to_bytes();
				(secret, public.try_into().expect("G2 public key is 96 bytes"))
			})
			.collect()
	}

	fn aggregate(signatures: &[BlsSignature<TinyBLS381>]) -> [u8; BLS_G1_SIGNATURE_LEN] {
		let mut sum: Option<<TinyBLS381 as EngineBLS>::SignatureGroup> = None;
		for signature in signatures {
			sum = Some(sum.map_or(signature.0, |acc| acc + signature.0));
		}
		BlsSignature::<TinyBLS381>(sum.expect("no signatures to aggregate"))
			.to_bytes()
			.try_into()
			.expect("G1 signature is 48 bytes")
	}

	/// A trusted state and a proof over `signer_indices`, both internally consistent.
	///
	/// The MMR is a single leaf, so its root is just the leaf hash and an empty proof verifies.
	/// That lets the happy path run offline rather than only against a chain.
	fn valid_proof(
		validators: &[Validator],
		signer_indices: &[usize],
	) -> (ConsensusState, BlsMmrProof) {
		let leaf = MmrLeaf {
			version: MmrLeafVersion::new(0, 0),
			parent_number_and_hash: (BLOCK - 1, H256::zero()),
			beefy_next_authority_set: BeefyNextAuthoritySet {
				id: SET_ID + 1,
				len: validators.len() as u32,
				keyset_commitment: H256::zero(),
			},
			leaf_extra: H256::zero(),
		};
		let mmr_root = H256(keccak_256(&leaf.encode()));

		let payload = Payload::from_single_entry(*b"mh", mmr_root.0.to_vec());
		let commitment = Commitment { payload, block_number: BLOCK, validator_set_id: SET_ID };

		// Two trees, mirroring the runtime. The BLS keys have their own tree, and its root is one
		// extra leaf of the authority set tree, after the per-authority leaves the ECDSA path
		// proves against. Those are stand-ins here; only their count matters.
		let bls_leaves = validators.iter().map(|(_, key)| keccak_256(key)).collect::<Vec<_>>();
		let bls_tree = MerkleTree::<MerkleHasher>::from_leaves(&bls_leaves);
		let bls_commitment = H256(bls_tree.root().expect("bls tree has a root"));
		let authority_proof = bls_tree.proof(signer_indices).proof_hashes().to_vec();

		let mut keyset_leaves =
			(0..validators.len()).map(|i| keccak_256(&[b'a', i as u8])).collect::<Vec<_>>();
		keyset_leaves.push(keccak_256(bls_commitment.as_bytes()));
		let keyset_tree = MerkleTree::<MerkleHasher>::from_leaves(&keyset_leaves);
		let keyset_commitment = H256(keyset_tree.root().expect("keyset tree has a root"));
		let keyset_proof = keyset_tree.proof(&[validators.len()]).proof_hashes().to_vec();

		let message = Message::new(b"", &commitment.encode());
		let signatures = signer_indices
			.iter()
			.map(|&i| validators[i].0.sign(&message))
			.collect::<Vec<_>>();

		let signers = signer_indices
			.iter()
			.map(|&i| BlsSigner { public_key: validators[i].1, index: i as u32 })
			.collect::<Vec<_>>();

		let trusted_state = ConsensusState {
			latest_beefy_height: BLOCK - 1,
			beefy_activation_block: 0,
			mmr_root_hash: H256::zero(),
			current_authorities: BeefyAuthoritySet {
				id: SET_ID,
				len: validators.len() as u32,
				keyset_commitment,
			},
			next_authorities: BeefyAuthoritySet {
				id: SET_ID + 1,
				len: validators.len() as u32,
				keyset_commitment: H256::zero(),
			},
		};

		let proof = BlsMmrProof {
			commitment,
			signers,
			aggregate_signature: aggregate(&signatures),
			latest_mmr_leaf: leaf,
			mmr_proof: LeafProof { leaf_indices: vec![0], leaf_count: 1, items: vec![] },
			bls_commitment,
			keyset_proof,
			authority_proof,
		};

		(trusted_state, proof)
	}

	fn verify(
		trusted_state: ConsensusState,
		proof: BlsMmrProof,
	) -> Result<(ConsensusState, H256), Error> {
		sp_io::TestExternalities::default()
			.execute_with(|| crate::verify_bls_mmr_update_proof::<TestHost>(trusted_state, proof))
	}

	#[test]
	fn accepts_a_valid_aggregate() {
		let validators = validators(4);
		let (trusted_state, proof) = valid_proof(&validators, &[0, 1, 2, 3]);

		let (new_state, _leaf_extra) = verify(trusted_state, proof).expect("should verify");

		assert_eq!(new_state.latest_beefy_height, BLOCK, "height should advance to the commitment");
	}

	// Three of four is the supermajority, so a partial set must still verify. The aggregate is
	// over the signers alone, which is what makes the merkle multi-proof necessary.
	#[test]
	fn accepts_a_supermajority_subset() {
		let validators = validators(4);
		let (trusted_state, proof) = valid_proof(&validators, &[0, 1, 3]);

		assert!(verify(trusted_state, proof).is_ok());
	}

	// The check that stops a prover claiming one validator many times. Without it, a single
	// signer's key and signature could be repeated to clear the supermajority threshold, and BLS
	// aggregation would happily verify the repeated key against the repeated signature.
	#[test]
	fn rejects_duplicate_signer_indices() {
		let validators = validators(4);
		let (trusted_state, mut proof) = valid_proof(&validators, &[0, 1, 2]);

		// One real signer, counted three times. The signature is genuinely the sum of three
		// copies of validator 0's signature, so the pairing check itself would pass.
		let message = Message::new(b"", &proof.commitment.encode());
		let signature = validators[0].0.sign(&message);
		proof.aggregate_signature =
			aggregate(&[validators[0].0.sign(&message), validators[0].0.sign(&message), signature]);
		for signer in proof.signers.iter_mut() {
			signer.public_key = validators[0].1;
			signer.index = 0;
		}

		assert!(matches!(verify(trusted_state, proof), Err(Error::InvalidBlsSignerOrdering)));
	}

	#[test]
	fn rejects_unordered_signer_indices() {
		let validators = validators(4);
		let (trusted_state, mut proof) = valid_proof(&validators, &[0, 1, 2]);
		proof.signers.swap(0, 2);

		assert!(matches!(verify(trusted_state, proof), Err(Error::InvalidBlsSignerOrdering)));
	}

	#[test]
	fn rejects_signer_outside_the_authority_set() {
		let validators = validators(4);
		let (trusted_state, mut proof) = valid_proof(&validators, &[0, 1, 2]);
		proof.signers.last_mut().unwrap().index = 9;

		assert!(matches!(verify(trusted_state, proof), Err(Error::InvalidBlsSignerOrdering)));
	}

	#[test]
	fn rejects_sub_supermajority() {
		let validators = validators(4);
		// Two of four is short of the >2/3 threshold.
		let (trusted_state, proof) = valid_proof(&validators, &[0, 1]);

		assert!(matches!(verify(trusted_state, proof), Err(Error::SuperMajorityRequired)));
	}

	#[test]
	fn rejects_a_proof_with_no_signers() {
		let validators = validators(4);
		let (trusted_state, mut proof) = valid_proof(&validators, &[0, 1, 2]);
		proof.signers.clear();

		assert!(matches!(verify(trusted_state, proof), Err(Error::NoBlsSigners)));
	}

	#[test]
	fn rejects_a_stale_commitment() {
		let validators = validators(4);
		let (mut trusted_state, proof) = valid_proof(&validators, &[0, 1, 2]);
		trusted_state.latest_beefy_height = BLOCK;

		assert!(matches!(verify(trusted_state, proof), Err(Error::StaleHeight { .. })));
	}

	#[test]
	fn rejects_an_unknown_authority_set() {
		let validators = validators(4);
		let (mut trusted_state, proof) = valid_proof(&validators, &[0, 1, 2]);
		trusted_state.current_authorities.id = SET_ID + 5;
		trusted_state.next_authorities.id = SET_ID + 6;

		assert!(matches!(verify(trusted_state, proof), Err(Error::UnknownAuthoritySet { .. })));
	}

	// A signature over a different commitment: well formed points, wrong message.
	#[test]
	fn rejects_an_aggregate_over_the_wrong_message() {
		let validators = validators(4);
		let (trusted_state, mut proof) = valid_proof(&validators, &[0, 1, 2]);

		let other = Message::new(b"", b"a different commitment");
		proof.aggregate_signature = aggregate(&[
			validators[0].0.sign(&other),
			validators[1].0.sign(&other),
			validators[2].0.sign(&other),
		]);

		assert!(matches!(verify(trusted_state, proof), Err(Error::BlsVerificationFailed)));
	}

	// Dropping a signer from the aggregate while leaving their key in the proof must not verify,
	// or a validator could be credited with a signature they never produced.
	#[test]
	fn rejects_an_aggregate_missing_a_claimed_signer() {
		let validators = validators(4);
		let (trusted_state, mut proof) = valid_proof(&validators, &[0, 1, 2]);

		let message = Message::new(b"", &proof.commitment.encode());
		proof.aggregate_signature =
			aggregate(&[validators[0].0.sign(&message), validators[1].0.sign(&message)]);

		assert!(matches!(verify(trusted_state, proof), Err(Error::BlsVerificationFailed)));
	}

	// All-ones bytes are not undecodable. The compressed encoding is big-endian with the point
	// flags in the high bits of the *first* byte, and 0xff sets the infinity flag, so these decode
	// to the identity element instead of failing. The identity is harmless (it contributes nothing
	// to either sum, and a signer still has to appear in the committed keyset), but the rejection
	// therefore arrives as a failed pairing rather than a decode error.
	#[test]
	fn rejects_an_identity_public_key() {
		let validators = validators(4);
		let (trusted_state, mut proof) = valid_proof(&validators, &[0, 1, 2]);
		proof.signers[1].public_key = [0xff; BLS_G2_PUBLIC_KEY_LEN];

		let result = verify(trusted_state, proof);
		assert!(matches!(result, Err(Error::BlsVerificationFailed)), "got {result:?}");
	}

	#[test]
	fn rejects_an_identity_signature() {
		let validators = validators(4);
		let (trusted_state, mut proof) = valid_proof(&validators, &[0, 1, 2]);
		proof.aggregate_signature = [0xff; BLS_G1_SIGNATURE_LEN];

		let result = verify(trusted_state, proof);
		assert!(matches!(result, Err(Error::BlsVerificationFailed)), "got {result:?}");
	}

	// Corrupting the coordinate while leaving the flag bits alone does fail to decode, which is
	// the path that reports `InvalidBlsPoint`.
	#[test]
	fn rejects_an_undecodable_public_key() {
		let validators = validators(4);
		let (trusted_state, mut proof) = valid_proof(&validators, &[0, 1, 2]);
		proof.signers[1].public_key[0] ^= 0xff;

		let result = verify(trusted_state, proof);
		assert!(matches!(result, Err(Error::InvalidBlsPoint)), "got {result:?}");
	}

	// Correctly signed by keys that simply are not the committed authority set. The pairing check
	// passes; only the merkle multi-proof catches it.
	#[test]
	fn rejects_signers_outside_the_committed_keyset() {
		let committed = validators(4);
		let impostors = (0..4)
			.map(|i| {
				let secret = SecretKeyVT::<TinyBLS381>::from_seed(&[b'x', i as u8]);
				let public = secret.into_public().to_bytes();
				(secret, public.try_into().expect("G2 public key is 96 bytes"))
			})
			.collect::<Vec<Validator>>();

		let (trusted_state, mut proof) = valid_proof(&impostors, &[0, 1, 2]);
		// Keep the impostors' signatures and keys, but point the state at the real keyset.
		let leaves = committed.iter().map(|(_, key)| keccak_256(key)).collect::<Vec<_>>();
		let tree = MerkleTree::<MerkleHasher>::from_leaves(&leaves);
		let mut trusted_state = trusted_state;
		trusted_state.current_authorities.keyset_commitment =
			H256(tree.root().expect("keyset tree has a root"));
		proof.authority_proof = tree.proof(&[0, 1, 2]).proof_hashes().to_vec();

		assert!(matches!(verify(trusted_state, proof), Err(Error::InvalidAuthoritiesProof)));
	}
}

/// Emits an EIP-2537 shaped fixture for the Solidity aggregate verifier.
///
/// The Solidity side cannot consume the compressed points the proof carries, because EIP-2537 has
/// no decompression precompile: it wants uncompressed, big-endian, zero-padded coordinates. This
/// prints exactly that, for a deterministic validator set, and asserts `w3f-bls` accepts the same
/// aggregate first, so the fixture cannot drift from what the chain would produce.
///
///   cargo test -p beefy-verifier --features bls-crypto bls_eip2537_fixture -- --nocapture
#[cfg(feature = "bls-crypto")]
#[test]
fn bls_eip2537_fixture() {
	use ark_bls12_381::{Fq, G1Affine, G2Affine};
	use ark_ec::{AffineRepr, CurveGroup};
	use ark_ff::{BigInteger, PrimeField};
	use w3f_bls::{
		EngineBLS, Message, PublicKey, SecretKeyVT, SerializableToBytes, Signature as BlsSignature,
		TinyBLS381,
	};

	let message = b"beefy-bls-aggregate-fixture";
	let msg = Message::new(b"", message);

	// Same deterministic construction the offline tests use.
	let validators: Vec<_> =
		(0..3).map(|i| SecretKeyVT::<TinyBLS381>::from_seed(&[b'v', i as u8])).collect();

	let mut agg_sig: Option<<TinyBLS381 as EngineBLS>::SignatureGroup> = None;
	let mut agg_pub: Option<<TinyBLS381 as EngineBLS>::PublicKeyGroup> = None;
	for secret in &validators {
		let sig = secret.sign(&msg);
		let public = secret.into_public();
		agg_sig = Some(agg_sig.map_or(sig.0, |acc| acc + sig.0));
		agg_pub = Some(agg_pub.map_or(public.0, |acc| acc + public.0));
	}
	let agg_sig = agg_sig.expect("signers");
	let agg_pub = agg_pub.expect("signers");

	// The aggregate must verify before the fixture is worth anything.
	assert!(
		BlsSignature::<TinyBLS381>(agg_sig).verify(&msg, &PublicKey::<TinyBLS381>(agg_pub)),
		"aggregate does not verify, fixture would be meaningless"
	);

	let fq = |v: &Fq| hex::encode(v.into_bigint().to_bytes_be());

	let sig_affine: G1Affine = agg_sig.into_affine();
	let (sx, sy) = sig_affine.xy().expect("signature is not the identity");

	let pub_affine: G2Affine = agg_pub.into_affine();
	let (px, py) = pub_affine.xy().expect("public key is not the identity");

	println!("=== EIP-2537 fixture: {} signers ===", validators.len());
	println!("message              {}", hex::encode(message));
	println!(
		"compressed signature {}",
		hex::encode(BlsSignature::<TinyBLS381>(agg_sig).to_bytes())
	);
	println!("compressed pubkey    {}", hex::encode(PublicKey::<TinyBLS381>(agg_pub).to_bytes()));
	println!("-- aggregate signature, G1 uncompressed --");
	println!("sig.x                {}", fq(sx));
	println!("sig.y                {}", fq(sy));
	println!("-- aggregate public key, G2 uncompressed --");
	println!("pk.x.c0              {}", fq(&px.c0));
	println!("pk.x.c1              {}", fq(&px.c1));
	println!("pk.y.c0              {}", fq(&py.c0));
	println!("pk.y.c1              {}", fq(&py.c1));

	// The keyset commitment the Solidity client verifies against, built over the *uncompressed*
	// encoding since that is what the contract can hash without a decompression precompile. Four
	// authorities, of which the three above signed, so the multi-proof is non-trivial.
	let uncompressed = |secret: &SecretKeyVT<TinyBLS381>| -> Vec<u8> {
		let affine: G2Affine = secret.into_public().0.into_affine();
		let (x, y) = affine.xy().expect("public key is not the identity");
		let mut out = Vec::with_capacity(4 * 64);
		for coord in [&x.c0, &x.c1, &y.c0, &y.c1] {
			out.extend_from_slice(&[0u8; 16]);
			out.extend_from_slice(&coord.into_bigint().to_bytes_be());
		}
		out
	};

	let authorities: Vec<_> =
		(0..4).map(|i| SecretKeyVT::<TinyBLS381>::from_seed(&[b'v', i as u8])).collect();
	// The runtime commits the compressed encoding, and the contract compresses to match, so the
	// leaves are over compressed keys.
	let leaves: Vec<[u8; 32]> = authorities
		.iter()
		.map(|secret| keccak_256(&secret.into_public().to_bytes()))
		.collect();
	let bls_tree = MerkleTree::<MerkleHasher>::from_leaves(&leaves);
	let bls_commitment = bls_tree.root().expect("bls root");
	let _ = &uncompressed;

	// The authority set tree: per-authority leaves (stand-ins for the ECDSA addresses), then the
	// BLS commitment as one extra leaf.
	let mut keyset_leaves: Vec<[u8; 32]> =
		(0..authorities.len()).map(|i| keccak_256(&[b'a', i as u8])).collect();
	keyset_leaves.push(keccak_256(&bls_commitment));
	let keyset_tree = MerkleTree::<MerkleHasher>::from_leaves(&keyset_leaves);

	for (i, secret) in authorities.iter().enumerate() {
		println!("authority {i} compressed  {}", hex::encode(secret.into_public().to_bytes()));
	}
	println!("-- two-level keyset, {} authorities --", authorities.len());
	println!("bls commitment       {}", hex::encode(bls_commitment));
	println!("keyset root          {}", hex::encode(keyset_tree.root().expect("root")));
	for hash in keyset_tree.proof(&[authorities.len()]).proof_hashes() {
		println!("keyset proof node    {}", hex::encode(hash));
	}
	for hash in bls_tree.proof(&[0, 1, 2]).proof_hashes() {
		println!("authority proof node {}", hex::encode(hash));
	}

	// Each signer's key on its own, for the merkle leaves and the G2_ADD path.
	for (i, secret) in validators.iter().enumerate() {
		let affine: G2Affine = secret.into_public().0.into_affine();
		let (x, y) = affine.xy().expect("public key is not the identity");
		println!("-- signer {i} --");
		println!("  x.c0               {}", fq(&x.c0));
		println!("  x.c1               {}", fq(&x.c1));
		println!("  y.c0               {}", fq(&y.c0));
		println!("  y.c1               {}", fq(&y.c1));
	}
}

/// Emits a complete ABI-encoded state and proof so the Solidity client's `verify()` entry point
/// can be exercised, not just the pieces it calls.
///
/// The MMR is a single leaf, so its root is the leaf hash and an empty proof verifies. The
/// commitment carries that root, and the validators sign the SCALE encoding of the commitment,
/// which is exactly what the contract hashes. Keys are emitted uncompressed, because EIP-2537 has
/// no decompression precompile, and the keyset commitment is built over the same encoding.
///
///   cargo test -p beefy-verifier --features bls-crypto bls_solidity_proof_fixture -- --nocapture
#[cfg(feature = "bls-crypto")]
#[test]
fn bls_solidity_proof_fixture() {
	use alloy_sol_types::SolType;
	use ark_bls12_381::G2Affine;
	use ark_ec::{AffineRepr, CurveGroup};
	use ark_ff::{BigInteger, PrimeField};
	use ismp_abi::bls_beefy::BlsBeefy as Sol;
	use w3f_bls::{
		EngineBLS, Message, SecretKeyVT, SerializableToBytes, Signature as BlsSignature, TinyBLS381,
	};

	const SET_ID: u64 = 7;
	const BLOCK: u32 = 100;

	let uncompressed = |secret: &SecretKeyVT<TinyBLS381>| -> Vec<u8> {
		let affine: G2Affine = secret.into_public().0.into_affine();
		let (x, y) = affine.xy().expect("not the identity");
		let mut out = Vec::with_capacity(256);
		for coord in [&x.c0, &x.c1, &y.c0, &y.c1] {
			out.extend_from_slice(&[0u8; 16]);
			out.extend_from_slice(&coord.into_bigint().to_bytes_be());
		}
		out
	};

	let authorities: Vec<_> =
		(0..4).map(|i| SecretKeyVT::<TinyBLS381>::from_seed(&[b'v', i as u8])).collect();
	let keys: Vec<Vec<u8>> = authorities.iter().map(uncompressed).collect();

	// Two trees, as the runtime builds them: the BLS keys in their own tree, whose root is one
	// extra leaf of the authority set tree. The leaves before it stand in for the ECDSA addresses.
	let bls_leaves: Vec<[u8; 32]> =
		authorities.iter().map(|s| keccak_256(&s.into_public().to_bytes())).collect();
	let bls_tree = MerkleTree::<MerkleHasher>::from_leaves(&bls_leaves);
	let bls_commitment = bls_tree.root().expect("bls root");

	let mut keyset_leaves: Vec<[u8; 32]> =
		(0..authorities.len()).map(|i| keccak_256(&[b'a', i as u8])).collect();
	keyset_leaves.push(keccak_256(&bls_commitment));
	let keyset = MerkleTree::<MerkleHasher>::from_leaves(&keyset_leaves);
	let keyset_root = keyset.root().expect("keyset root");
	let keyset_proof = keyset.proof(&[authorities.len()]);

	// The MMR leaf, and the root it implies as the only leaf in the tree.
	let leaf = MmrLeaf {
		version: MmrLeafVersion::new(0, 0),
		parent_number_and_hash: (0u32, H256::zero()),
		beefy_next_authority_set: BeefyNextAuthoritySet {
			id: SET_ID + 1,
			len: authorities.len() as u32,
			keyset_commitment: H256(keyset_root),
		},
		leaf_extra: H256::zero(),
	};
	let mmr_root = H256(keccak_256(&leaf.encode()));

	// The validators sign the SCALE encoding of this commitment; the contract hashes the same.
	let payload = Payload::from_single_entry(*b"mh", mmr_root.0.to_vec());
	let commitment = Commitment { payload, block_number: BLOCK, validator_set_id: SET_ID };
	let message = Message::new(b"", &commitment.encode());

	let signer_indices = [0usize, 1, 2];
	let mut agg: Option<<TinyBLS381 as EngineBLS>::SignatureGroup> = None;
	for &i in &signer_indices {
		let sig = authorities[i].sign(&message);
		agg = Some(agg.map_or(sig.0, |acc| acc + sig.0));
	}
	let agg_affine = BlsSignature::<TinyBLS381>(agg.expect("signers")).0.into_affine();
	let (sx, sy) = agg_affine.xy().expect("not the identity");
	let mut aggregate_signature = Vec::with_capacity(128);
	for coord in [sx, sy] {
		aggregate_signature.extend_from_slice(&[0u8; 16]);
		aggregate_signature.extend_from_slice(&coord.into_bigint().to_bytes_be());
	}

	let authority_proof = bls_tree.proof(&signer_indices);

	// Assemble the sol types the contract decodes.
	let state = Sol::BeefyConsensusState {
		latestHeight: alloy_primitives::U256::from(BLOCK - 1),
		beefyActivationBlock: alloy_primitives::U256::ZERO,
		currentAuthoritySet: Sol::AuthoritySetCommitment {
			id: SET_ID,
			len: authorities.len() as u32,
			root: alloy_primitives::FixedBytes(keyset_root),
		},
		nextAuthoritySet: Sol::AuthoritySetCommitment {
			id: SET_ID + 1,
			len: authorities.len() as u32,
			root: alloy_primitives::FixedBytes(keyset_root),
		},
	};

	let relay = Sol::BlsRelayChainProof {
		commitment: Sol::Commitment {
			payload: vec![Sol::Payload {
				id: alloy_primitives::FixedBytes(*b"mh"),
				data: alloy_primitives::Bytes::from(mmr_root.0.to_vec()),
			}],
			blockNumber: BLOCK,
			validatorSetId: SET_ID,
		},
		signers: signer_indices
			.iter()
			.map(|&i| Sol::BlsSigner {
				publicKey: alloy_primitives::Bytes::from(keys[i].clone()),
				authorityIndex: alloy_primitives::U256::from(i),
			})
			.collect(),
		aggregateSignature: alloy_primitives::Bytes::from(aggregate_signature),
		latestMmrLeaf: Sol::BeefyMmrLeaf {
			version: 0,
			parentNumber: 0,
			parentHash: alloy_primitives::FixedBytes([0u8; 32]),
			nextAuthoritySet: Sol::AuthoritySetCommitment {
				id: SET_ID + 1,
				len: authorities.len() as u32,
				root: alloy_primitives::FixedBytes(keyset_root),
			},
			extra: alloy_primitives::FixedBytes([0u8; 32]),
			leafIndex: alloy_primitives::U256::ZERO,
		},
		mmrProof: vec![],
		blsCommitment: alloy_primitives::FixedBytes(bls_commitment),
		keysetProof: keyset_proof
			.proof_hashes()
			.iter()
			.map(|h| alloy_primitives::FixedBytes(*h))
			.collect(),
		proof: authority_proof
			.proof_hashes()
			.iter()
			.map(|h| alloy_primitives::FixedBytes(*h))
			.collect(),
	};

	let parachain = Sol::ParachainProof {
		parachains: vec![],
		proof: vec![],
		leafCount: alloy_primitives::U256::ZERO,
	};

	let encoded_state = Sol::BeefyConsensusState::abi_encode(&state);
	let encoded_proof =
		<(Sol::BlsRelayChainProof, Sol::ParachainProof) as SolType>::abi_encode_params(&(
			relay, parachain,
		));

	println!("=== solidity verify() fixture ===");
	println!("state 0x{}", hex::encode(encoded_state));
	println!("proof 0x{}", hex::encode(encoded_proof));
	println!("expected new latestHeight {BLOCK}");
}

/// Works out the compressed-encoding sign rule, so the Solidity client can compress an
/// uncompressed key on the fly and match the leaf the runtime commits.
///
/// Compressing is cheap; decompressing a G2 point on chain would need Fp2 square roots. So the
/// prover sends uncompressed points, which the pairing needs anyway, and the contract derives the
/// compressed form for the merkle leaf. That only works if the flag convention is pinned down.
///
///   cargo test -p beefy-verifier --features bls-crypto bls_compression_rule -- --nocapture
#[cfg(feature = "bls-crypto")]
#[test]
fn bls_compression_rule() {
	use ark_bls12_381::{Fq, G2Affine};
	use ark_ec::{AffineRepr, CurveGroup};
	use ark_ff::{BigInteger, PrimeField};
	use w3f_bls::{SecretKeyVT, SerializableToBytes, TinyBLS381};

	// (p - 1) / 2, the threshold the IETF convention uses to call a root "larger".
	let half = {
		let modulus = Fq::MODULUS;
		let mut bytes = modulus.to_bytes_be();
		// divide by two, big-endian, then subtract nothing: (p-1)/2 == p >> 1 for odd p
		let mut carry = 0u8;
		for b in bytes.iter_mut() {
			let cur = *b;
			*b = (cur >> 1) | (carry << 7);
			carry = cur & 1;
		}
		bytes
	};

	let gt_half = |v: &Fq| -> bool { v.into_bigint().to_bytes_be() > half };

	println!("=== compressed flag vs y sign, {} samples ===", 8);
	for i in 0..8u8 {
		let secret = SecretKeyVT::<TinyBLS381>::from_seed(&[b'c', i]);
		let compressed = secret.into_public().to_bytes();
		let affine: G2Affine = secret.into_public().0.into_affine();
		let (_, y) = affine.xy().expect("not the identity");

		println!(
			"seed {i}: flags {:#04x}  y.c1>half {}  y.c0>half {}",
			compressed[0] & 0xe0,
			gt_half(&y.c1),
			gt_half(&y.c0),
		);
	}
}

/// `compress_g2` / `compress_g1` must reproduce what `w3f-bls` serialises, since the keyset
/// commitment and the Rust verifier both work on the compressed encoding while an EVM-bound proof
/// carries uncompressed points.
#[cfg(feature = "bls-crypto")]
#[test]
fn compression_matches_w3f_bls() {
	use ark_bls12_381::{G1Affine, G2Affine};
	use ark_ec::{AffineRepr, CurveGroup};
	use ark_ff::{BigInteger, PrimeField};
	use beefy_verifier_primitives::{compress_g1, compress_g2};
	use w3f_bls::{Message, SecretKeyVT, SerializableToBytes, TinyBLS381};

	let msg = Message::new(b"", b"compression check");

	for i in 0..8u8 {
		let secret = SecretKeyVT::<TinyBLS381>::from_seed(&[b'c', i]);

		// G2 public key.
		let affine: G2Affine = secret.into_public().0.into_affine();
		let (x, y) = affine.xy().expect("not the identity");
		let mut uncompressed = [0u8; 256];
		for (slot, coord) in [&x.c0, &x.c1, &y.c0, &y.c1].iter().enumerate() {
			let bytes = coord.into_bigint().to_bytes_be();
			uncompressed[slot * 64 + 16..slot * 64 + 64].copy_from_slice(&bytes);
		}
		assert_eq!(
			compress_g2(&uncompressed).to_vec(),
			secret.into_public().to_bytes(),
			"G2 compression differs for seed {i}"
		);

		// G1 signature.
		let sig = secret.sign(&msg);
		let sig_affine: G1Affine = sig.0.into_affine();
		let (sx, sy) = sig_affine.xy().expect("not the identity");
		let mut sig_uncompressed = [0u8; 128];
		for (slot, coord) in [sx, sy].iter().enumerate() {
			let bytes = coord.into_bigint().to_bytes_be();
			sig_uncompressed[slot * 64 + 16..slot * 64 + 64].copy_from_slice(&bytes);
		}
		assert_eq!(
			compress_g1(&sig_uncompressed).to_vec(),
			sig.to_bytes(),
			"G1 compression differs for seed {i}"
		);
	}
}

/// Emits an ABI fixture built from a **live** BLS relay, so the Solidity client can be tested
/// against a real MMR proof and a real parachain header rather than a synthetic single-leaf tree.
///
/// Needs a BLS BEEFY relay running with a parachain registered on it, since the proof has to carry
/// a real parachain header. The para id must be 4009, which is what gargantua tracks.
///
///   RELAY_WS_URL=ws://127.0.0.1:9979 PARA_WS_URL=ws://127.0.0.1:9991 \
///     cargo test -p beefy-verifier --features bls bls_live_abi_fixture -- --ignored --nocapture
#[cfg(feature = "bls")]
#[tokio::test]
#[ignore]
async fn bls_live_abi_fixture() {
	use alloy_sol_types::SolType;
	use beefy_prover::bls::{abi::to_abi_proof, decode_paired_justification};
	use ismp_abi::{
		bls_beefy::BlsBeefy, ecdsa_beefy::BeefyConsensusState as SolBeefyConsensusState,
	};

	let max_rpc_payload_size = 15 * 1024 * 1024;
	let relay_ws_url = std::env::var("RELAY_WS_URL").expect("RELAY_WS_URL must be set");
	let para_ws_url = std::env::var("PARA_WS_URL").expect("PARA_WS_URL must be set");

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

	let latest: H256 =
		relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await.unwrap();
	let block = relay_rpc.chain_get_block(Some(latest.into())).await.unwrap().unwrap();
	let justification = block
		.justifications
		.expect("justifications")
		.into_iter()
		.find_map(|j| (j.0 == polkadot_sdk::sp_consensus_beefy::BEEFY_ENGINE_ID).then_some(j.1))
		.expect("beefy justification");

	let signed = decode_paired_justification(&justification).unwrap();
	let set_id = signed.commitment.validator_set_id;

	// Anchor one authority set back so the proof also exercises a rotation.
	let mut anchor = H256::default();
	let mut cursor = latest;
	for _ in 0..4000 {
		let header = relay_rpc.chain_get_header(Some(cursor.into())).await.unwrap().unwrap();
		let parent: H256 = header.parent_hash.into();
		if parent.is_zero() {
			break;
		}
		if let Ok(Some(b)) = relay_rpc.chain_get_block(Some(parent.into())).await {
			if let Some(js) = b.justifications {
				if let Some(raw) = js.into_iter().find_map(|j| {
					(j.0 == polkadot_sdk::sp_consensus_beefy::BEEFY_ENGINE_ID).then_some(j.1)
				}) {
					let prev = decode_paired_justification(&raw).unwrap();
					if prev.commitment.validator_set_id + 1 == set_id {
						anchor = parent;
						break;
					}
				}
			}
		}
		cursor = parent;
	}
	assert!(!anchor.is_zero(), "no anchor one set back");

	let state = prover.get_initial_consensus_state(Some(anchor)).await.unwrap();
	let message = prover.bls_consensus_proof(signed).await.unwrap();

	let signer_count = message.mmr.signers.len();
	let para_count = message.parachain.parachains.len();
	let mmr_nodes = message.mmr.mmr_proof.items.len();
	let block_number = message.mmr.commitment.block_number;

	let abi_state: SolBeefyConsensusState = state.into();
	let abi_proof = to_abi_proof(message).expect("to_abi_proof");

	let encoded_state = SolBeefyConsensusState::abi_encode(&abi_state);
	let encoded_proof =
		<(BlsBeefy::BlsRelayChainProof, BlsBeefy::ParachainProof) as SolType>::abi_encode_params(
			&(abi_proof.relay, abi_proof.parachain),
		);

	println!("=== live abi fixture ===");
	println!(
		"signers {signer_count} | parachains {para_count} | mmr nodes {mmr_nodes} | block {block_number}"
	);
	println!("state 0x{}", hex::encode(encoded_state));
	println!("proof 0x{}", hex::encode(encoded_proof));
	assert!(para_count > 0, "expected a parachain header from the registered para");
}
