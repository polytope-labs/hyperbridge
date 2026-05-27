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

use crate::{EcdsaRecover, error::Error, verify_consensus, verify_mmr_update_proof};

struct TestHost;

impl Keccak256 for TestHost {
	fn keccak256(bytes: &[u8]) -> H256 {
		sp_core::hashing::keccak_256(bytes).into()
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

	// Fixture: Polkadot relay block 31213559, Nexus parachain height 10184385.
	// Generated against SP1 program vkey 0x009ce9c8...cddcf (the mainnet deployment).
	let state_bytes = hex!(
		"0000000000000000000000000000000000000000000000000000000001dc47ef00000000000000000000000000000000000000000000000000000000012a53180000000000000000000000000000000000000000000000000000000000001314000000000000000000000000000000000000000000000000000000000000025880af94e4aabe6b11819d8e50059b73693140c4e781a3380311ffd1334d3685800000000000000000000000000000000000000000000000000000000000001315000000000000000000000000000000000000000000000000000000000000025880af94e4aabe6b11819d8e50059b73693140c4e781a3380311ffd1334d368580"
	);
	let proof_bytes = hex!(
		"0000000000000000000000000000000000000000000000000000000001dc47f7000000000000000000000000000000000000000000000000000000000000131400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001dc47f626de324920a139adbcfec37592c3d5d4f1c5d47be3c962da23f54de266b6b7af0000000000000000000000000000000000000000000000000000000000001315000000000000000000000000000000000000000000000000000000000000025880af94e4aabe6b11819d8e50059b73693140c4e781a3380311ffd1334d368580dd392bcf43ae0ec709e1b092c8104422611665975c6cd579c30dd08e9b087b8700000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000340000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000d2700000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000139371a10acc61519dfb7f41332f5b20fe72f320506c47091aaaf4210d4f078dec7069b6d028d87933c5237ba98c8dca60d753a1053494e6c11df7c23cba900e0cf54797f2a008138fa61940fae7e4f1ec58907ccc57eced453b3d00b29c7f7eaa999e03e09140661757261208caed508000000000452505352906934e5099eb4a44dfb23d258d0510adb4e9a427fc7499b8870130457d826ad10ce1f71070449534d500101db5a2025cacc6a30cb8359a594d7d5cd3b001b060efc658775c43dd9febee19f9b04637257e1a28fb87a12795e7c1f4bb4f43c5c03f904dbda1b6dcbd63883ae044953544d20902e046a0000000005617572610101367b70f2da76391f7f810ab08a393987460cff2d2e290fc689ff6fccc30807568f8e57226898192ad284ee82e6187e07bd3864988c29a905d4f108323d38f18b0000000000000000000000000000000000000000000000000000000000000000000000000001644388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f2535200000000000000000000000000000000000000000000000000000000000000001e42a064240ad4f396a1db358a97274a6b43b143323c70ccb064463c51f620ae18cdd61384fb99d6440bde7472eac01b5441502a7ae704ad89bb1708640b138e228e12adc1afa827f2fa3de02243430dec1f4c9eb55cf7c7d1a0e764ac7b76540f8544daaa4e46e81227c73b6dbb99e60bc412100b588b6d00178b37943a43b2262ecd4e750cf44b41b95b306a24e408cc1b1549b3087717f57660b23eca45de121714e2e0045462d3ce7aa03700daf7842ca59126f6b30819523e3eabc8ad9c0db273840c5d6bf2468b8925dcd6e8aee857e3197ef280dcf0a8d5369be964822b4b0f8508f24277738b1f4c9ca06d2f2166f42f9f9b470bc5968526d55bbbc300000000000000000000000000000000000000000000000000000000"
	);

	let sol_state =
		<BeefyConsensusState as SolValue>::abi_decode(&state_bytes).expect("decode state");
	let trusted: ConsensusState = sol_state.into();

	// Proof payload matches SP1Beefy.sol:verifyConsensus's `abi.decode(...)` call:
	// a sequence of four top-level types, not a struct wrapper.
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

	// Mainnet SP1Beefy verification key — matches `SP1_VERIFICATION_KEY` in
	// `0x82582f85cf370adCB61D97dab3068c0C4102Ccb6`.
	let vkey_hash = "0x009ce9c86546ac790c9e694519e16e59ff34b633c309fe4d6a4f850b886cddcf";
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
