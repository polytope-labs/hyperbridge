use std::convert::TryInto;

use codec::{Decode, Encode};
use polkadot_sdk::{
	sp_consensus_beefy::VersionedFinalityProof, sp_core::H256, sp_io::hashing::keccak_256, *,
};
use sp_consensus_beefy::ecdsa_crypto::Signature;
use subxt::{backend::legacy::LegacyRpcMethods, ext::subxt_rpcs::rpc_params, PolkadotConfig};

use beefy_prover::{
	relay::{fetch_mmr_proof, paras_parachains},
	util::{hash_authority_addresses, merkle_proof},
	Prover,
};
use beefy_verifier_primitives::{
	BeefyConsensusProof, BeefyMmrLeaf, ConsensusState, Node, ParachainHeader, ParachainProof,
	RelaychainProof, SignatureWithAuthorityIndex,
};
use ismp::{
	consensus::{ConsensusClient, StateMachineId},
	host::{IsmpHost, StateMachine},
	messaging::Keccak256,
};
use ismp_beefy::{consensus::BEEFY_CONSENSUS_ID, Config};
use ismp_parachain::Parachains;

use crate::runtime::*;

struct TestKeccak256;

impl Keccak256 for TestKeccak256 {
	fn keccak256(bytes: &[u8]) -> H256 {
		sp_core::hashing::keccak_256(bytes).into()
	}
}

async fn setup() -> (ConsensusState, BeefyConsensusProof) {
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
		para_ids: vec![3367],
		query_batch_size: Some(100),
	};

	let latest_beefy_hash: H256 =
		relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await.unwrap();

	let mut previous_beefy_hash = H256::default();
	let mut current_hash = latest_beefy_hash;
	for _ in 0..1000 {
		let header = relay_rpc.chain_get_header(Some(current_hash.into())).await.unwrap().unwrap();
		let parent_hash: H256 = header.parent_hash.into();
		let block = relay_rpc.chain_get_block(Some(parent_hash.into())).await.unwrap().unwrap();

		if let Some(justifications) = block.justifications {
			if justifications.iter().any(|j| j.0 == sp_consensus_beefy::BEEFY_ENGINE_ID) {
				previous_beefy_hash = parent_hash;
				break;
			}
		}
		current_hash = parent_hash;
	}

	let initial_state = prover
		.get_initial_consensus_state(Some(previous_beefy_hash.into()))
		.await
		.unwrap();

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

	let (mmr_leaf_proof, latest_leaf) =
		fetch_mmr_proof(&prover.relay_rpc, signed_commitment_raw.commitment.block_number, None)
			.await
			.unwrap();

	let signatures = signed_commitment_raw
		.signatures
		.iter()
		.enumerate()
		.filter_map(|(index, sig)| {
			sig.as_ref().map(|s| {
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

	let (parachains, indices): (Vec<_>, Vec<_>) = prover
		.para_ids
		.iter()
		.map(|id| {
			let index = heads.iter().position(|(i, _)| *i == *id).expect("ParaId should exist");
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
	let proof_2d = merkle_proof(&leaves, &indices);
	let proof = proof_2d.into_iter().flatten().map(|level| level.1).collect();
	dbg!(&leaves.len());
	let parachain_proof = ParachainProof { parachains, proof, total_leaves: leaves.len() as u32 };

	let beefy_consensus_proof =
		BeefyConsensusProof { relay: relay_proof, parachain: parachain_proof };

	(initial_state, beefy_consensus_proof)
}

#[tokio::test]
async fn test_verify_consensus() {
	let (initial_state, beefy_consensus_proof) = setup().await;
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		Parachains::<Test>::insert(3367, 12000);

		let host = Ismp::default();
		let consensus_client = host.consensus_client(BEEFY_CONSENSUS_ID).unwrap();
		let consensus_state_id = b"BEEF".to_vec();
		let trusted_consensus_state = initial_state.encode();
		let proof = beefy_consensus_proof.encode();

		let result = consensus_client.verify_consensus(
			&host,
			consensus_state_id.try_into().unwrap(),
			trusted_consensus_state,
			proof,
		);

		assert!(result.is_ok(), "Consensus verification failed: {:?}", result.err());

		let (new_state, commitments) = result.unwrap();
		let new_consensus_state = ConsensusState::decode(&mut &*new_state).unwrap();

		assert!(new_consensus_state.latest_beefy_height > initial_state.latest_beefy_height);
		assert!(!commitments.is_empty());

		let (state_machine, state_commitments) = commitments.into_iter().next().unwrap();
		assert_eq!(
			state_machine,
			StateMachineId {
				state_id: StateMachine::Kusama(3367),
				consensus_state_id: b"BEEF".to_vec().try_into().unwrap()
			}
		);
		assert!(!state_commitments.is_empty());
		dbg!(state_commitments);
		println!("Successfully verified beefy justification and extracted parachain commitments");
	});
}
