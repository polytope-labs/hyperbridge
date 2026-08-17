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

use crate::{
	EcdsaRecover,
	ecdsa::{verify_consensus, verify_mmr_update_proof},
	error::Error,
};

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
/// verifier itself is unchanged: this is "Option A", where a BLS-BEEFY relay is verified through
/// the existing ECDSA path.
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

	// Initial trusted state via the prover, which exercises the folded BLS justification decode.
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

	// Each signer's key on its own, for the G2_ADD path.
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

/// Checks the group-bridging step an APK proof would need, against a live BLS relay.
///
/// `gnark-apk-proofs` aggregates public keys in G1 and pairs them against a G2 signature, while
/// BEEFY signs in G1 with G2 public keys. `DoublePublicKey` publishes the same secret in both
/// groups, so both aggregates describe one aggregate secret and the two can be tied together
/// without changing how the relay signs:
///
/// ```text
///   e(apk_g1, g2) == e(g1, apk_g2)              binds an untrusted apk_g2 to apk_g1
///   e(sig_g1, g2) == e(hash_to_g1(msg), apk_g2) BEEFY's existing G1 signature
/// ```
///
/// A verifier gets `apk_g1` from the SNARK and takes `apk_g2` as an untrusted input, so proving
/// both equations hold for real validator keys is what makes the design viable. Both checks are
/// constant cost, unlike the per-signer merkle paths they would replace.
///
///   RELAY_WS_URL=ws://127.0.0.1:9979 \
///     cargo test -p beefy-verifier --features bls,bls-crypto bls_apk_group_binding -- --ignored
/// --nocapture
#[cfg(all(feature = "bls", feature = "bls-crypto"))]
#[tokio::test]
#[ignore]
async fn bls_apk_group_binding() {
	use ark_bls12_381::{Bls12_381, G1Affine, G1Projective, G2Affine, G2Projective};
	use ark_ec::{AffineRepr, CurveGroup, Group, pairing::Pairing};
	use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
	use beefy_prover::bls::{
		aggregate_signatures, beefy_g1_authorities, beefy_g2_authorities,
		decode_paired_justification,
	};
	use w3f_bls::{Message, TinyBLS381};

	let relay_ws_url = std::env::var("RELAY_WS_URL").expect("RELAY_WS_URL must be set");
	let (_relay_client, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, 15 * 1024 * 1024)
			.await
			.unwrap();
	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());

	// A real finalized commitment, signed by the relay's paired ecdsa_bls381 validators.
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

	let at = Some(latest);
	let g1_keys = beefy_g1_authorities(&relay_rpc, at).await.unwrap();
	let g2_keys = beefy_g2_authorities(&relay_rpc, at).await.unwrap();
	assert_eq!(g1_keys.len(), g2_keys.len(), "both halves come from the same paired keys");

	// Aggregate only the validators that actually signed, which is what a bitmask selects.
	let mut apk_g1 = G1Projective::default();
	let mut apk_g2 = G2Projective::default();
	let mut signatures = Vec::new();
	let mut signer_count = 0usize;
	for (index, maybe_signature) in signed.signatures.iter().enumerate() {
		let Some(signature) = maybe_signature else { continue };
		apk_g1 += G1Affine::deserialize_compressed(&g1_keys[index][..])
			.expect("validator G1 public key decodes");
		apk_g2 += G2Affine::deserialize_compressed(&g2_keys[index][..])
			.expect("validator G2 public key decodes");
		signatures.push(signature.g1_signature());
		signer_count += 1;
	}
	assert!(signer_count > 0, "commitment carries no signatures");

	let apk_g1 = apk_g1.into_affine();
	let apk_g2 = apk_g2.into_affine();

	// 1. The binding check. Holds only when both aggregates share a discrete log, so a verifier can
	//    accept apk_g2 from an untrusted relayer once the SNARK has fixed apk_g1.
	let bound = Bls12_381::pairing(apk_g1, G2Affine::generator()) ==
		Bls12_381::pairing(G1Affine::generator(), apk_g2);
	assert!(bound, "e(apk_g1, g2) != e(g1, apk_g2): the two aggregates disagree");

	// 2. BEEFY's unmodified G1 signature, verified against the G2 aggregate just bound above.
	let message = signed.commitment.encode();
	let aggregate_signature = aggregate_signatures(&signatures).unwrap();
	let sig_g1 = G1Affine::deserialize_compressed(&aggregate_signature[..])
		.expect("aggregate signature decodes");
	let message_point = Message::new(b"", &message)
		.hash_to_signature_curve::<TinyBLS381>()
		.into_affine();
	let signed_ok = Bls12_381::pairing(sig_g1, G2Affine::generator()) ==
		Bls12_381::pairing(message_point, apk_g2);
	assert!(signed_ok, "e(sig_g1, g2) != e(H(m), apk_g2): signature does not verify");

	// 3. A negative control, so the equations are not vacuously true.
	let tampered = (apk_g2.into_group() + G2Projective::generator()).into_affine();
	assert!(
		Bls12_381::pairing(apk_g1, G2Affine::generator()) !=
			Bls12_381::pairing(G1Affine::generator(), tampered),
		"binding check accepted a tampered apk_g2",
	);

	println!(
		"[ok] group binding holds for {signer_count} live signers: apk_g1 <-> apk_g2 bound, \
		 and BEEFY's G1 signature verifies against the bound apk_g2",
	);

	// Dump everything an APK proof fixture needs, so a test elsewhere can be built against real
	// BEEFY data rather than synthetic keypairs.
	let hex_affine_g1 = |p: &G1Affine| {
		let mut b = Vec::new();
		p.serialize_compressed(&mut b).unwrap();
		hex::encode(b)
	};
	let hex_affine_g2 = |p: &G2Affine| {
		let mut b = Vec::new();
		p.serialize_compressed(&mut b).unwrap();
		hex::encode(b)
	};
	println!("=== apk fixture inputs ===");
	println!("message  {}", hex::encode(&message));
	println!("apk_g1   {}", hex_affine_g1(&apk_g1));
	println!("apk_g2   {}", hex_affine_g2(&apk_g2));
	println!("agg_sig  {}", hex::encode(aggregate_signature));
	for (i, key) in g1_keys.iter().enumerate() {
		println!("g1[{i}]    {}", hex::encode(key));
	}
}

/// Collects everything an APK consensus proof needs from a live BLS relay, except the SNARK.
///
/// The SNARK is generated by `gnark-apk-proofs`, which pulls in a Go toolchain through cgo and an
/// 800MB structured reference string, so it stays out of this workspace. The two halves meet over
/// the json this writes:
///
/// ```text
///   this test            ->  apk-inputs.json   (live BEEFY data, validator G1 keys)
///   gnark-apk-proofs     ->  apk-snark.json    (PLONK proof, public inputs)
///   bls_apk_live_fixture ->  the two .hex files BlsApkBeefy.t.sol reads
/// ```
///
/// The parachain must be registered as 4009, which is what gargantua tracks.
///
///   RELAY_WS_URL=ws://127.0.0.1:9979 PARA_WS_URL=ws://127.0.0.1:9981 \
///     APK_FIXTURE_DIR=/tmp/apk \
///     cargo test -p beefy-verifier --features bls,bls-crypto bls_apk_live_inputs -- --ignored
/// --nocapture
#[cfg(all(feature = "bls", feature = "bls-crypto"))]
#[tokio::test]
#[ignore]
async fn bls_apk_live_inputs() {
	use ark_bls12_381::{G1Affine, G1Projective, G2Affine, G2Projective};
	use ark_ec::{AffineRepr, CurveGroup};
	use ark_ff::{BigInteger, PrimeField};
	use ark_serialize::CanonicalDeserialize;
	use beefy_prover::bls::{
		aggregate_signatures, beefy_g1_authorities, beefy_g2_authorities,
		decode_paired_justification,
	};

	// EIP-2537 wants padded coordinates, but `ApkProof` takes bytes32[3] and bytes32[6], which are
	// the coordinates packed with no padding at all. Getting this wrong produces a well formed
	// point and a silent pairing failure, so both encodings exist side by side deliberately.
	fn fq_be(fq: &ark_bls12_381::Fq, out: &mut Vec<u8>) {
		out.extend_from_slice(&fq.into_bigint().to_bytes_be());
	}
	fn g1_packed(point: &G1Affine) -> String {
		let (x, y) = point.xy().expect("not the identity");
		let mut bytes = Vec::with_capacity(96);
		fq_be(x, &mut bytes);
		fq_be(y, &mut bytes);
		hex::encode(bytes)
	}
	fn g2_packed(point: &G2Affine) -> String {
		let (x, y) = point.xy().expect("not the identity");
		let mut bytes = Vec::with_capacity(192);
		fq_be(&x.c0, &mut bytes);
		fq_be(&x.c1, &mut bytes);
		fq_be(&y.c0, &mut bytes);
		fq_be(&y.c1, &mut bytes);
		hex::encode(bytes)
	}

	let max_rpc_payload_size = 15 * 1024 * 1024;
	let relay_ws_url = std::env::var("RELAY_WS_URL").expect("RELAY_WS_URL must be set");
	let para_ws_url = std::env::var("PARA_WS_URL").expect("PARA_WS_URL must be set");
	let out_dir = std::env::var("APK_FIXTURE_DIR").expect("APK_FIXTURE_DIR must be set");

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

	// Anchor one authority set back, so the proof also exercises a rotation rather than only the
	// current set. Exactly one back: the verifier accepts the trusted state's current or next set.
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
	let message = prover.bls_consensus_proof(signed.clone()).await.unwrap();

	// The validator set that signed, both halves of the same paired keys.
	let g1_keys = beefy_g1_authorities(&relay_rpc, Some(latest)).await.unwrap();
	let g2_keys = beefy_g2_authorities(&relay_rpc, Some(latest)).await.unwrap();
	assert_eq!(g1_keys.len(), g2_keys.len());

	let mut apk_g1 = G1Projective::default();
	let mut apk_g2 = G2Projective::default();
	let mut signatures = Vec::new();
	let mut participation = Vec::new();
	for (index, maybe_signature) in signed.signatures.iter().enumerate() {
		let Some(signature) = maybe_signature else { continue };
		apk_g1 += G1Affine::deserialize_compressed(&g1_keys[index][..]).expect("G1 key decodes");
		apk_g2 += G2Affine::deserialize_compressed(&g2_keys[index][..]).expect("G2 key decodes");
		signatures.push(signature.g1_signature());
		participation.push(index as u64);
	}
	assert!(!participation.is_empty(), "commitment carries no signatures");

	let apk_g1 = apk_g1.into_affine();
	let apk_g2 = apk_g2.into_affine();
	let aggregate = aggregate_signatures(&signatures).unwrap();
	let sig_g1 = G1Affine::deserialize_compressed(&aggregate[..]).expect("aggregate decodes");

	// The same commitment the runtime pallet builds, computed here over the signing set so the
	// fixture's consensus state can be seeded with it.
	let decompressed = g1_keys
		.iter()
		.map(|k| G1Affine::deserialize_compressed(&k[..]).expect("G1 key decodes"))
		.collect::<Vec<_>>();
	let padded = apk_commitment::padded_to_circuit_width(&decompressed);
	let apk_commitment_bytes = apk_commitment::public_keys_commitment_bytes(&padded);

	let mmr = &message.mmr;
	let leaf_index = mmr.mmr_proof.leaf_indices.first().copied().unwrap_or_default();
	let bundle = json::json!({
		"validatorSetId": set_id,
		"blockNumber": mmr.commitment.block_number,
		"payloadMh": hex::encode(mmr.commitment.payload.get_raw(b"mh").expect("mmr payload")),
		"keys": g1_keys.iter().map(|k| {
			g1_packed(&G1Affine::deserialize_compressed(&k[..]).expect("G1 key decodes"))
		}).collect::<Vec<_>>(),
		"participation": participation,
		"apk": g1_packed(&apk_g1),
		"apk2": g2_packed(&apk_g2),
		"signature": g1_packed(&sig_g1),
		"apkCommitment": hex::encode(apk_commitment_bytes),
		"mmrLeaf": {
			"parentNumber": mmr.latest_mmr_leaf.parent_number_and_hash.0,
			"parentHash": hex::encode(mmr.latest_mmr_leaf.parent_number_and_hash.1.0),
			"nextAuthoritySetId": mmr.latest_mmr_leaf.beefy_next_authority_set.id,
			"nextAuthoritySetLen": mmr.latest_mmr_leaf.beefy_next_authority_set.len,
			"nextAuthoritySetRoot":
				hex::encode(mmr.latest_mmr_leaf.beefy_next_authority_set.keyset_commitment.0),
			"extra": hex::encode(mmr.latest_mmr_leaf.leaf_extra.0),
			"leafIndex": leaf_index,
		},
		"mmrProof": mmr.mmr_proof.items.iter().map(|h| hex::encode(h.0)).collect::<Vec<_>>(),
		"parachains": message.parachain.parachains.iter().map(|p| json::json!({
			"index": p.index,
			"id": p.para_id,
			"header": hex::encode(&p.header),
		})).collect::<Vec<_>>(),
		"parachainProof":
			message.parachain.proof.iter().map(|h| hex::encode(h)).collect::<Vec<_>>(),
		"parachainLeafCount": message.parachain.total_leaves,
		"trusted": {
			"latestHeight": state.latest_beefy_height,
			"beefyActivationBlock": state.beefy_activation_block,
			"currentId": state.current_authorities.id,
			"currentLen": state.current_authorities.len,
			"nextId": state.next_authorities.id,
			"nextLen": state.next_authorities.len,
		},
	});

	std::fs::create_dir_all(&out_dir).expect("create fixture dir");
	let path = format!("{out_dir}/apk-inputs.json");
	std::fs::write(&path, json::to_string_pretty(&bundle).unwrap()).expect("write bundle");

	println!("=== apk live inputs ===");
	println!(
		"set {set_id} | {} validators | {} signed | block {} | mmr nodes {} | parachains {}",
		g1_keys.len(),
		participation.len(),
		mmr.commitment.block_number,
		mmr.mmr_proof.items.len(),
		message.parachain.parachains.len(),
	);
	println!("apk commitment 0x{}", hex::encode(apk_commitment_bytes));
	println!("wrote {path}");
	assert!(
		!message.parachain.parachains.is_empty(),
		"expected a parachain header from the registered para",
	);
}

/// Assembles the two hex fixtures `BlsApkBeefy.t.sol` reads, from the live data collected by
/// `bls_apk_live_inputs` and the proof generated by `gnark-apk-proofs`. Needs no chain access, so
/// the fixture can be rebuilt without the relay still running.
///
///   APK_FIXTURE_DIR=/tmp/apk \
///     cargo test -p beefy-verifier --features bls bls_apk_live_fixture -- --ignored --nocapture
#[cfg(feature = "bls")]
#[test]
#[ignore]
fn bls_apk_live_fixture() {
	use alloy_primitives::{Bytes, FixedBytes, U256};
	use alloy_sol_types::{SolType, SolValue};
	use ismp_abi::bls_beefy::BlsBeefy;

	let dir = std::env::var("APK_FIXTURE_DIR").expect("APK_FIXTURE_DIR must be set");
	let read = |name: &str| -> json::Value {
		let path = format!("{dir}/{name}");
		json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}")))
			.expect("valid json")
	};
	let inputs = read("apk-inputs.json");
	let snark = read("apk-snark.json");

	let hex_bytes = |value: &json::Value| hex::decode(value.as_str().expect("hex string")).unwrap();
	let fixed32 = |value: &json::Value| FixedBytes::<32>::from_slice(&hex_bytes(value));
	let u64_of = |value: &json::Value| value.as_u64().expect("number");
	// Splits a packed curve point into the bytes32 words the SNARK verifier takes. Not the padded
	// EIP-2537 layout the rest of the BLS code uses.
	let words = |value: &json::Value, count: usize| -> Vec<FixedBytes<32>> {
		let raw = hex_bytes(value);
		assert_eq!(raw.len(), count * 32, "point is the wrong width for bytes32[{count}]");
		raw.chunks(32).map(FixedBytes::<32>::from_slice).collect()
	};

	// The commitment the proof was generated against has to be the one the client checks it with,
	// so take it from the SNARK's own public inputs and require the runtime's version to agree.
	let apk_commitment = fixed32(&snark["apkCommitment"]);
	assert_eq!(
		apk_commitment,
		fixed32(&inputs["apkCommitment"]),
		"the apk-commitment crate and the circuit disagree about the same validator set",
	);

	let trusted = &inputs["trusted"];
	let signing_set_id = u64_of(&inputs["validatorSetId"]);
	// Only the set that signed needs its commitment seeded; the other is learned from a digest.
	let authority_set = |id: u64, len: u64| BlsBeefy::AuthoritySetCommitment {
		id,
		len: len as u32,
		root: if id == signing_set_id { apk_commitment } else { FixedBytes::ZERO },
	};

	let state = BlsBeefy::BeefyConsensusState {
		latestHeight: U256::from(u64_of(&trusted["latestHeight"])),
		beefyActivationBlock: U256::from(u64_of(&trusted["beefyActivationBlock"])),
		currentAuthoritySet: authority_set(
			u64_of(&trusted["currentId"]),
			u64_of(&trusted["currentLen"]),
		),
		nextAuthoritySet: authority_set(u64_of(&trusted["nextId"]), u64_of(&trusted["nextLen"])),
	};
	assert!(
		state.currentAuthoritySet.root != FixedBytes::ZERO ||
			state.nextAuthoritySet.root != FixedBytes::ZERO,
		"neither trusted set matches the signing set, so the client would have no commitment",
	);

	let bitlist: [U256; 5] = snark["bitlist"]
		.as_array()
		.expect("bitlist")
		.iter()
		.map(|w| U256::from_be_slice(&hex_bytes(w)))
		.collect::<Vec<_>>()
		.try_into()
		.expect("five words");

	let leaf = &inputs["mmrLeaf"];
	let relay = BlsBeefy::BlsApkRelayChainProof {
		commitment: BlsBeefy::Commitment {
			payload: vec![BlsBeefy::Payload {
				id: FixedBytes(*b"mh"),
				data: Bytes::from(hex_bytes(&inputs["payloadMh"])),
			}],
			blockNumber: u64_of(&inputs["blockNumber"]) as u32,
			validatorSetId: signing_set_id,
		},
		bitlist,
		apk: words(&inputs["apk"], 3).try_into().expect("bytes32[3]"),
		apk2: words(&inputs["apk2"], 6).try_into().expect("bytes32[6]"),
		apkProof: Bytes::from(hex_bytes(&snark["apkProof"])),
		signature: words(&inputs["signature"], 3).try_into().expect("bytes32[3]"),
		latestMmrLeaf: BlsBeefy::BeefyMmrLeaf {
			version: 0,
			parentNumber: u64_of(&leaf["parentNumber"]) as u32,
			parentHash: fixed32(&leaf["parentHash"]),
			nextAuthoritySet: BlsBeefy::AuthoritySetCommitment {
				id: u64_of(&leaf["nextAuthoritySetId"]),
				len: u64_of(&leaf["nextAuthoritySetLen"]) as u32,
				root: fixed32(&leaf["nextAuthoritySetRoot"]),
			},
			extra: fixed32(&leaf["extra"]),
			leafIndex: U256::from(u64_of(&leaf["leafIndex"])),
		},
		mmrProof: inputs["mmrProof"].as_array().expect("mmrProof").iter().map(fixed32).collect(),
	};

	let parachain = BlsBeefy::ParachainProof {
		parachains: inputs["parachains"]
			.as_array()
			.expect("parachains")
			.iter()
			.map(|para| BlsBeefy::Parachain {
				index: U256::from(u64_of(&para["index"])),
				id: U256::from(u64_of(&para["id"])),
				header: Bytes::from(hex_bytes(&para["header"])),
			})
			.collect(),
		proof: inputs["parachainProof"]
			.as_array()
			.expect("proof")
			.iter()
			.map(fixed32)
			.collect(),
		leafCount: U256::from(u64_of(&inputs["parachainLeafCount"])),
	};

	// SolValue, not SolType: this has to match `abi.encode(struct)` on the Solidity side.
	let encoded_state = SolValue::abi_encode(&state);
	let encoded_proof =
		<(BlsBeefy::BlsApkRelayChainProof, BlsBeefy::ParachainProof) as SolType>::abi_encode_params(
			&(relay.clone(), parachain.clone()),
		);

	let fixtures = std::env::var("APK_FIXTURE_OUT")
		.unwrap_or_else(|_| "../../../../evm/tests/foundry/fixtures".to_string());
	std::fs::write(
		format!("{fixtures}/bls-apk-beefy-state.hex"),
		format!("0x{}", hex::encode(&encoded_state)),
	)
	.expect("write state");
	std::fs::write(
		format!("{fixtures}/bls-apk-beefy-proof.hex"),
		format!("0x{}", hex::encode(&encoded_proof)),
	)
	.expect("write proof");

	println!("=== apk live fixture ===");
	println!(
		"signers {} | parachains {} | mmr nodes {} | block {} | apk proof {} bytes",
		bitlist.iter().map(|w| w.count_ones()).sum::<usize>(),
		parachain.parachains.len(),
		relay.mmrProof.len(),
		relay.commitment.blockNumber,
		relay.apkProof.len(),
	);
	println!("state {} bytes, proof {} bytes", encoded_state.len(), encoded_proof.len());
	println!("wrote {fixtures}/bls-apk-beefy-{{state,proof}}.hex");
}
