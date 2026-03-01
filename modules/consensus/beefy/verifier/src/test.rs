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
use polkadot_sdk::{sp_consensus_beefy::VersionedFinalityProof, *};
use sp_core::H256;
use sp_io::hashing::keccak_256;
use subxt::{PolkadotConfig, backend::legacy::LegacyRpcMethods, ext::subxt_rpcs::rpc_params};

use beefy_prover::{
	Prover,
	relay::{fetch_mmr_proof, paras_parachains},
	util::{hash_authority_addresses, merkle_proof},
};
use beefy_verifier_primitives::{
	BeefyConsensusProof, BeefyMmrLeaf, ParachainHeader, ParachainProof, RelaychainProof,
	SignatureWithAuthorityIndex,
};
use ismp::messaging::Keccak256;
use k256::ecdsa::{RecoveryId, Signature as K256Signature, VerifyingKey};
use polkadot_sdk::sp_consensus_beefy::ecdsa_crypto::Signature;

use crate::{EcdsaRecover, verify_consensus};

struct TestKeccak256;

impl Keccak256 for TestKeccak256 {
	fn keccak256(bytes: &[u8]) -> H256 {
		sp_core::hashing::keccak_256(bytes).into()
	}
}

impl EcdsaRecover for TestKeccak256 {
	fn secp256k1_recover(prehash: &[u8; 32], signature: &[u8; 65]) -> anyhow::Result<[u8; 64]> {
		let recovery_id = RecoveryId::from_byte(signature[64])
			.ok_or_else(|| anyhow::anyhow!("Invalid recovery id"))?;
		let k256_sig = K256Signature::from_slice(&signature[0..64])
			.map_err(|e| anyhow::anyhow!("Invalid signature format: {e}"))?;
		let recovered_verifying_key =
			VerifyingKey::recover_from_prehash(prehash, &k256_sig, recovery_id)
				.map_err(|e| anyhow::anyhow!("Failed to recover public key: {e}"))?;
		let uncompressed_point = recovered_verifying_key.to_encoded_point(false);
		let uncompressed_bytes = &uncompressed_point.as_bytes()[1..];
		let mut result = [0u8; 64];
		result.copy_from_slice(uncompressed_bytes);
		Ok(result)
	}
}

#[tokio::test]
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
	let authority_proof_2d = merkle_proof(&authority_address_hashes, &authority_indices);
	let authority_proof_nodes = authority_proof_2d
		.into_iter()
		.flatten()
		.map(|(_, hash)| H256::from(hash))
		.collect();

	let signed_commitment = beefy_verifier_primitives::SignedCommitment {
		commitment: signed_commitment_raw.commitment.clone(),
		signatures,
	};

	let beefy_mmr_leaf = BeefyMmrLeaf {
		version: latest_leaf.version.clone(),
		parent_block_and_hash: (
			latest_leaf.parent_number_and_hash.0,
			latest_leaf.parent_number_and_hash.1,
		),
		beefy_next_authority_set: latest_leaf.beefy_next_authority_set.clone(),
		k_index: 0,
		leaf_index: mmr_leaf_proof.leaf_indices[0] as u32,
		extra: latest_leaf.leaf_extra,
	};

	let relay_proof = RelaychainProof {
		signed_commitment,
		latest_mmr_leaf: beefy_mmr_leaf,
		mmr_proof: mmr_leaf_proof.items,
		proof: authority_proof_nodes,
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
	let proof_2d = merkle_proof(&leaves, &indices);
	let proof = proof_2d.into_iter().flatten().map(|(_, hash)| hash).collect();
	let parachain_proof = ParachainProof { parachains, proof, total_leaves: leaves.len() as u32 };

	println!("Assembling final proof for verification");
	let consensus_proof = BeefyConsensusProof { relay: relay_proof, parachain: parachain_proof };

	let result = verify_consensus::<TestKeccak256>(trusted_state, consensus_proof);

	assert!(result.is_ok(), "Consensus verification failed: {:?}", result.err());

	println!("Successfully verified beefy justification for block #{}", block_number);
}
