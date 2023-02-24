use super::*;
use base2::Base2;
use sync_committee_primitives::{
	types::{LightClientState, LightClientUpdate, SyncCommitteeUpdate},
	util::compute_sync_committee_period_at_slot,
};

use ethereum_consensus::{
	altair::Checkpoint, bellatrix::compute_domain, primitives::Root, signing::compute_signing_root,
	state_transition::Context,
};
use ssz_rs::{calculate_multi_merkle_root, is_valid_merkle_branch, GeneralizedIndex, Merkleized};
use std::time::Duration;
use sync_committee_primitives::{
	types::{DOMAIN_SYNC_COMMITTEE, GENESIS_VALIDATORS_ROOT},
	util::compute_fork_version,
};
use sync_committee_verifier::verify_sync_committee_attestation;
use tokio::time;
use tokio_stream::{wrappers::IntervalStream, StreamExt};

// **NOTE**
// 1. To run these tests make sure the latest fork version on your devnet is the
// BELLATRIX_FORK_VERSION as defined in the mainnet config
// 2. Modify `sync_committee_primitives::types::GENESIS_ROOT_VALIDATORS` defined under the testing
// feature flag to match the one that is present in the devnet you are running the tests with
// 3. Make sure the SLOTS_PER_EPOCH is set to 32 in your beacon node.

const NODE_URL: &'static str = "http://localhost:5052";

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
#[ignore]
async fn fetch_block_header_works() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());
	let block_header = sync_committee_prover.fetch_header("head").await;
	assert!(block_header.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
#[ignore]
async fn fetch_block_works() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());
	let block = sync_committee_prover.fetch_block("head").await;
	assert!(block.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
#[ignore]
async fn fetch_sync_committee_works() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());
	let block = sync_committee_prover.fetch_sync_committee("head").await;
	assert!(block.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
#[ignore]
async fn fetch_validator_works() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());
	let validator = sync_committee_prover.fetch_validator("head", "48").await;
	assert!(validator.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
#[ignore]
async fn fetch_processed_sync_committee_works() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());
	let validator = sync_committee_prover.fetch_processed_sync_committee("head").await;
	assert!(validator.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
#[ignore]
async fn fetch_beacon_state_works() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());
	let beacon_state = sync_committee_prover.fetch_beacon_state("genesis").await;
	assert!(beacon_state.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
#[ignore]
async fn state_root_and_block_header_root_matches() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());
	let mut beacon_state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();

	let block_header = sync_committee_prover.fetch_header(&beacon_state.slot.to_string()).await;
	assert!(block_header.is_ok());

	let block_header = block_header.unwrap();
	let hash_tree_root = beacon_state.hash_tree_root();

	assert!(block_header.state_root == hash_tree_root.unwrap());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn fetch_finality_checkpoints_work() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());
	let finality_checkpoint = sync_committee_prover.fetch_finalized_checkpoint().await;
	assert!(finality_checkpoint.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn test_finalized_header() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());
	let mut state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();

	let proof = ssz_rs::generate_proof(state.clone(), &vec![FINALIZED_ROOT_INDEX as usize]);

	let leaves = vec![Node::from_bytes(
		state
			.finalized_checkpoint
			.hash_tree_root()
			.unwrap()
			.as_ref()
			.try_into()
			.unwrap(),
	)];
	let root = calculate_multi_merkle_root(
		&leaves,
		&proof.unwrap(),
		&[GeneralizedIndex(FINALIZED_ROOT_INDEX as usize)],
	);
	assert_eq!(root, state.hash_tree_root().unwrap());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn test_execution_payload_proof() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());

	let finalized_state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();
	let block_id = finalized_state.slot.to_string();
	let execution_payload_proof = prove_execution_payload(finalized_state.clone()).unwrap();

	let finalized_header = sync_committee_prover.fetch_header(&block_id).await.unwrap();

	// verify the associated execution header of the finalized beacon header.
	let mut execution_payload = execution_payload_proof.clone();
	let multi_proof_vec = execution_payload.multi_proof;
	let multi_proof_nodes = multi_proof_vec
		.iter()
		.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
		.collect::<Vec<_>>();
	let execution_payload_root = calculate_multi_merkle_root(
		&[
			Node::from_bytes(execution_payload.state_root.as_ref().try_into().unwrap()),
			execution_payload.block_number.hash_tree_root().unwrap(),
		],
		&multi_proof_nodes,
		&[
			GeneralizedIndex(EXECUTION_PAYLOAD_STATE_ROOT_INDEX as usize),
			GeneralizedIndex(EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX as usize),
		],
	);

	let execution_payload_hash_tree_root = finalized_state
		.latest_execution_payload_header
		.clone()
		.hash_tree_root()
		.unwrap();

	assert_eq!(execution_payload_root, execution_payload_hash_tree_root);

	let execution_payload_branch = execution_payload
		.execution_payload_branch
		.iter()
		.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
		.collect::<Vec<_>>();

	let is_merkle_branch_valid = is_valid_merkle_branch(
		&execution_payload_root,
		execution_payload_branch.iter(),
		EXECUTION_PAYLOAD_INDEX.floor_log2() as usize,
		GeneralizedIndex(EXECUTION_PAYLOAD_INDEX as usize).0,
		&Node::from_bytes(finalized_header.clone().state_root.as_ref().try_into().unwrap()),
	);

	assert!(is_merkle_branch_valid);
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn test_sync_committee_update_proof() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());

	let finalized_header = sync_committee_prover.fetch_header("head").await.unwrap();

	let finalized_state = sync_committee_prover
		.fetch_beacon_state(&finalized_header.slot.to_string())
		.await
		.unwrap();

	let sync_committee_proof = prove_sync_committee_update(finalized_state.clone()).unwrap();

	let sync_committee_proof = sync_committee_proof
		.into_iter()
		.map(|node| Bytes32::try_from(node.as_bytes()).expect("Node is always 32 byte slice"))
		.collect::<Vec<_>>();
	let mut sync_committee = finalized_state.next_sync_committee;

	let calculated_finalized_root = calculate_multi_merkle_root(
		&[Node::from_bytes(sync_committee.hash_tree_root().unwrap().as_ref().try_into().unwrap())],
		&sync_committee_proof
			.iter()
			.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
			.collect::<Vec<_>>(),
		&[GeneralizedIndex(NEXT_SYNC_COMMITTEE_INDEX as usize)],
	);

	assert_eq!(calculated_finalized_root.as_bytes(), finalized_header.state_root.as_bytes());

	let next_sync_committee_branch = sync_committee_proof
		.iter()
		.map(|node| Node::from_bytes(node.as_ref().try_into().unwrap()))
		.collect::<Vec<_>>();
	let is_merkle_branch_valid = is_valid_merkle_branch(
		&Node::from_bytes(sync_committee.hash_tree_root().unwrap().as_ref().try_into().unwrap()),
		next_sync_committee_branch.iter(),
		NEXT_SYNC_COMMITTEE_INDEX.floor_log2() as usize,
		NEXT_SYNC_COMMITTEE_INDEX as usize,
		&Node::from_bytes(finalized_header.state_root.as_ref().try_into().unwrap()),
	);

	assert!(is_merkle_branch_valid);
}

// todo: cloning the beacon state might expensive in production or testnet, if we can modify
// generate_proof function take a reference for the object that would be better.
#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn test_prover() {
	env_logger::init();
	let mut stream = IntervalStream::new(time::interval(Duration::from_secs(12 * 12)));

	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());

	let block_id = "head";

	let block_header = sync_committee_prover.fetch_header(&block_id).await.unwrap();

	let state = sync_committee_prover
		.fetch_beacon_state(&block_header.slot.to_string())
		.await
		.unwrap();

	let mut client_state = LightClientState {
		finalized_header: block_header.clone(),
		current_sync_committee: state.current_sync_committee,
		next_sync_committee: state.next_sync_committee,
	};

	let mut count = 0;
	while let Some(_ts) = stream.next().await {
		let finality_checkpoint = sync_committee_prover.fetch_finalized_checkpoint().await.unwrap();
		if finality_checkpoint.finalized.root == Node::default() ||
			finality_checkpoint.finalized.epoch <=
				compute_epoch_at_slot(client_state.finalized_header.slot)
		{
			continue
		}

		println!("A new epoch has been finalized {}", finality_checkpoint.finalized.epoch);

		let block_id = {
			let mut block_id = hex::encode(finality_checkpoint.finalized.root.as_bytes());
			block_id.insert_str(0, "0x");
			block_id
		};

		let finalized_header = sync_committee_prover.fetch_header(&block_id).await.unwrap();
		let finalized_state = sync_committee_prover
			.fetch_beacon_state(finalized_header.slot.to_string().as_str())
			.await
			.unwrap();
		let execution_payload_proof = prove_execution_payload(finalized_state.clone()).unwrap();

		let attested_epoch = finality_checkpoint.finalized.epoch + 2;
		// Get attested header and the signature slot

		let mut attested_slot = attested_epoch * SLOTS_PER_EPOCH;
		let (attested_block_header, signature_block) = loop {
			if (attested_epoch * SLOTS_PER_EPOCH).saturating_add(SLOTS_PER_EPOCH - 1) ==
				attested_slot
			{
				panic!("Could not find any block from the attested epoch")
			}

			if let Ok(header) =
				sync_committee_prover.fetch_header(attested_slot.to_string().as_str()).await
			{
				let mut signature_slot = header.slot + 1;
				let mut loop_count = 0;
				let signature_block = loop {
					if loop_count == 3 {
						panic!("Could not find valid signature block for attested slot {} after three loops", attested_slot);
					}
					if (attested_epoch * SLOTS_PER_EPOCH).saturating_add(SLOTS_PER_EPOCH - 1) ==
						signature_slot
					{
						println!("Waiting for signature block for attested header");
						std::thread::sleep(Duration::from_secs(24));
						signature_slot = header.slot + 1;
						loop_count += 1;
					}
					if let Ok(signature_block) =
						sync_committee_prover.fetch_block(signature_slot.to_string().as_str()).await
					{
						break signature_block
					}
					signature_slot += 1;
				};
				// If the next block does not have sufficient sync committee participants
				if signature_block
					.body
					.sync_aggregate
					.sync_committee_bits
					.as_bitslice()
					.count_ones() < 2 / 3 * (SYNC_COMMITTEE_SIZE)
				{
					attested_slot = signature_slot + 1;
					println!("Signature block does not have sufficient sync committee participants -> participants {}", signature_block.body.sync_aggregate.sync_committee_bits.as_bitslice().count_ones());
					continue
				}
				break (header, signature_block)
			}
			attested_slot += 1
		};

		let attested_state = sync_committee_prover
			.fetch_beacon_state(attested_block_header.slot.to_string().as_str())
			.await
			.unwrap();

		let finalized_hash_tree_root = finalized_header.clone().hash_tree_root().unwrap();
		println!("{:?}, {}", attested_state.finalized_checkpoint, attested_state.slot);
		println!(
			"{:?},  {:?}, {}",
			compute_epoch_at_slot(finalized_header.slot),
			finalized_hash_tree_root,
			finalized_header.slot
		);

		assert_eq!(
			Checkpoint {
				epoch: compute_epoch_at_slot(finalized_header.slot),
				root: finalized_hash_tree_root,
			},
			attested_state.finalized_checkpoint,
		);

		let finality_branch = prove_finalized_header(attested_state.clone()).unwrap();

		let state_period = compute_sync_committee_period_at_slot(finalized_header.slot);

		let update_attested_period =
			compute_sync_committee_period_at_slot(attested_block_header.slot);

		let sync_committee_update = if state_period == update_attested_period {
			let sync_committee_proof = prove_sync_committee_update(attested_state.clone()).unwrap();

			let sync_committee_proof = sync_committee_proof
				.into_iter()
				.map(|node| {
					Bytes32::try_from(node.as_bytes()).expect("Node is always 32 byte slice")
				})
				.collect::<Vec<_>>();

			Some(SyncCommitteeUpdate {
				next_sync_committee: attested_state.next_sync_committee,
				next_sync_committee_branch: sync_committee_proof,
			})
		} else {
			None
		};

		// construct light client
		let light_client_update = LightClientUpdate {
			attested_header: attested_block_header,
			sync_committee_update,
			finalized_header,
			execution_payload: execution_payload_proof,
			finality_branch,
			sync_aggregate: signature_block.body.sync_aggregate,
			signature_slot: signature_block.slot,
			// todo: Prove some ancestry blocks
			ancestor_blocks: vec![],
		};

		client_state =
			verify_sync_committee_attestation(client_state.clone(), light_client_update).unwrap();
		println!(
			"Sucessfully verified Ethereum block at slot {:?}",
			client_state.finalized_header.slot
		);

		count += 1;
		if count == 100 {
			break
		}
	}
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn test_sync_committee_signature_verification() {
	let sync_committee_prover = SyncCommitteeProver::new(NODE_URL.to_string());
	let block = loop {
		let block = sync_committee_prover.fetch_block("head").await.unwrap();
		if block.slot < 16 {
			std::thread::sleep(Duration::from_secs(48));
			continue
		}
		break block
	};
	let sync_committee = sync_committee_prover
		.fetch_processed_sync_committee(block.slot.to_string().as_str())
		.await
		.unwrap();

	let mut attested_header = sync_committee_prover
		.fetch_header((block.slot - 1).to_string().as_str())
		.await
		.unwrap();

	let sync_committee_pubkeys = sync_committee.public_keys;

	let participant_pubkeys = block
		.body
		.sync_aggregate
		.sync_committee_bits
		.iter()
		.zip(sync_committee_pubkeys.iter())
		.filter_map(|(bit, key)| if *bit { Some(key) } else { None })
		.collect::<Vec<_>>();

	let fork_version = compute_fork_version(compute_epoch_at_slot(block.slot));

	let context = Context::for_mainnet();
	let domain = compute_domain(
		DOMAIN_SYNC_COMMITTEE,
		Some(fork_version),
		Some(Root::from_bytes(GENESIS_VALIDATORS_ROOT.try_into().unwrap())),
		&context,
	)
	.unwrap();

	let signing_root = compute_signing_root(&mut attested_header, domain);

	ethereum_consensus::crypto::fast_aggregate_verify(
		&*participant_pubkeys,
		signing_root.unwrap().as_bytes(),
		&block.body.sync_aggregate.sync_committee_signature,
	)
	.unwrap();
}
