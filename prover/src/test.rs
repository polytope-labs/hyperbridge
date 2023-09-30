use super::*;
use base2::Base2;
use reqwest_eventsource::EventSource;
use ssz_rs::{
	calculate_multi_merkle_root, get_generalized_index, is_valid_merkle_branch, GeneralizedIndex,
	Merkleized, SszVariableOrIndex,
};
use std::time::Duration;
use sync_committee_primitives::{
	constants::{
		Root, DOMAIN_SYNC_COMMITTEE, EXECUTION_PAYLOAD_INDEX_LOG2, GENESIS_FORK_VERSION,
		GENESIS_VALIDATORS_ROOT, NEXT_SYNC_COMMITTEE_INDEX_LOG2,
	},
	types::LightClientState,
	util::{compute_domain, compute_fork_version, compute_signing_root},
};
use sync_committee_verifier::{
	signature_verification::verify_aggregate_signature, verify_sync_committee_attestation,
};
use tokio_stream::StreamExt;

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_block_header_works() {
	let sync_committee_prover = setup_prover();
	let block_header = sync_committee_prover.fetch_header("head").await;
	assert!(block_header.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_block_works() {
	let sync_committee_prover = setup_prover();
	let block = sync_committee_prover.fetch_block("head").await;
	assert!(block.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_validator_works() {
	let sync_committee_prover = setup_prover();
	let validator = sync_committee_prover.fetch_validator("head", "0").await;
	assert!(validator.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn fetch_processed_sync_committee_works() {
	let sync_committee_prover = setup_prover();
	let validator = sync_committee_prover.fetch_processed_sync_committee("head").await;
	assert!(validator.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn generate_indexes() {
	let sync_committee_prover = setup_prover();
	let beacon_state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();
	let execution_payload_index = get_generalized_index(
		&beacon_state,
		&[SszVariableOrIndex::Name("latest_execution_payload_header")],
	);
	let next_sync =
		get_generalized_index(&beacon_state, &[SszVariableOrIndex::Name("next_sync_committee")]);
	let finalized =
		get_generalized_index(&beacon_state, &[SszVariableOrIndex::Name("finalized_checkpoint")]);
	let execution_payload_root = get_generalized_index(
		&beacon_state.latest_execution_payload_header,
		&[SszVariableOrIndex::Name("state_root")],
	);
	let block_number = get_generalized_index(
		&beacon_state.latest_execution_payload_header,
		&[SszVariableOrIndex::Name("block_number")],
	);
	let timestamp = get_generalized_index(
		&beacon_state.latest_execution_payload_header,
		&[SszVariableOrIndex::Name("timestamp")],
	);
	dbg!(execution_payload_index);
	dbg!(next_sync);
	dbg!(finalized);
	dbg!(execution_payload_root);
	dbg!(block_number);
	dbg!(timestamp);

	dbg!(next_sync.floor_log2());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_beacon_state_works() {
	let sync_committee_prover = setup_prover();
	let beacon_state = sync_committee_prover.fetch_beacon_state("head").await;
	assert!(beacon_state.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn state_root_and_block_header_root_matches() {
	let sync_committee_prover = setup_prover();
	let mut beacon_state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();

	let block_header = sync_committee_prover.fetch_header(&beacon_state.slot.to_string()).await;
	assert!(block_header.is_ok());

	let block_header = block_header.unwrap();
	let hash_tree_root = beacon_state.hash_tree_root();

	assert_eq!(block_header.state_root, hash_tree_root.unwrap());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_finality_checkpoints_work() {
	let sync_committee_prover = setup_prover();
	let finality_checkpoint = sync_committee_prover.fetch_finalized_checkpoint().await;
	assert!(finality_checkpoint.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn test_finalized_header() {
	let sync_committee_prover = setup_prover();
	let mut state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();

	let proof = ssz_rs::generate_proof(&mut state, &vec![FINALIZED_ROOT_INDEX as usize]).unwrap();

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
		&proof,
		&[GeneralizedIndex(FINALIZED_ROOT_INDEX as usize)],
	);
	assert_eq!(root, state.hash_tree_root().unwrap());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn test_execution_payload_proof() {
	let sync_committee_prover = setup_prover();

	let mut finalized_state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();
	let block_id = finalized_state.slot.to_string();
	let execution_payload_proof = prove_execution_payload(&mut finalized_state).unwrap();

	let finalized_header = sync_committee_prover.fetch_header(&block_id).await.unwrap();

	// verify the associated execution header of the finalized beacon header.
	let mut execution_payload = execution_payload_proof.clone();
	let multi_proof_vec = execution_payload.multi_proof;
	let execution_payload_root = calculate_multi_merkle_root(
		&[
			Node::from_bytes(execution_payload.state_root.as_ref().try_into().unwrap()),
			execution_payload.block_number.hash_tree_root().unwrap(),
			execution_payload.timestamp.hash_tree_root().unwrap(),
		],
		&multi_proof_vec,
		&[
			GeneralizedIndex(EXECUTION_PAYLOAD_STATE_ROOT_INDEX as usize),
			GeneralizedIndex(EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX as usize),
			GeneralizedIndex(EXECUTION_PAYLOAD_TIMESTAMP_INDEX as usize),
		],
	);

	let execution_payload_hash_tree_root = finalized_state
		.latest_execution_payload_header
		.clone()
		.hash_tree_root()
		.unwrap();

	assert_eq!(execution_payload_root, execution_payload_hash_tree_root);

	let execution_payload_branch = execution_payload.execution_payload_branch.iter();

	let is_merkle_branch_valid = is_valid_merkle_branch(
		&execution_payload_root,
		execution_payload_branch,
		EXECUTION_PAYLOAD_INDEX_LOG2 as usize,
		EXECUTION_PAYLOAD_INDEX as usize,
		&finalized_header.state_root,
	);

	assert!(is_merkle_branch_valid);
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn test_sync_committee_update_proof() {
	let sync_committee_prover = setup_prover();

	let mut finalized_state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();
	let block_id = finalized_state.slot.to_string();
	let finalized_header = sync_committee_prover.fetch_header(&block_id).await.unwrap();

	let sync_committee_proof = prove_sync_committee_update(&mut finalized_state).unwrap();

	let mut sync_committee = finalized_state.next_sync_committee;

	let calculated_finalized_root = calculate_multi_merkle_root(
		&[sync_committee.hash_tree_root().unwrap()],
		&sync_committee_proof,
		&[GeneralizedIndex(NEXT_SYNC_COMMITTEE_INDEX as usize)],
	);

	assert_eq!(calculated_finalized_root.as_bytes(), finalized_header.state_root.as_bytes());

	let is_merkle_branch_valid = is_valid_merkle_branch(
		&sync_committee.hash_tree_root().unwrap(),
		sync_committee_proof.iter(),
		NEXT_SYNC_COMMITTEE_INDEX_LOG2 as usize,
		NEXT_SYNC_COMMITTEE_INDEX as usize,
		&finalized_header.state_root,
	);

	assert!(is_merkle_branch_valid);
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn test_prover() {
	use log::LevelFilter;
	use parity_scale_codec::{Decode, Encode};
	env_logger::builder()
		.filter_module("prover", LevelFilter::Debug)
		.format_module_path(false)
		.init();

	let sync_committee_prover = setup_prover();
	let node_url =
		format!("{}/eth/v1/events?topics=finalized_checkpoint", sync_committee_prover.node_url);
	let block_header = sync_committee_prover.fetch_header("head").await.unwrap();

	let state = sync_committee_prover
		.fetch_beacon_state(&block_header.slot.to_string())
		.await
		.unwrap();

	let mut client_state = LightClientState {
		finalized_header: block_header.clone(),
		latest_finalized_epoch: 0,
		current_sync_committee: state.current_sync_committee,
		next_sync_committee: state.next_sync_committee,
	};

	let mut count = 0;

	let mut es = EventSource::get(node_url);
	while let Some(event) = es.next().await {
		match event {
			Ok(reqwest_eventsource::Event::Message(msg)) => {
				let message: EventResponse = serde_json::from_str(&msg.data).unwrap();
				let checkpoint =
					Checkpoint { epoch: message.epoch.parse().unwrap(), root: message.block };
				let light_client_update = if let Some(update) = sync_committee_prover
					.fetch_light_client_update(client_state.clone(), checkpoint, "prover")
					.await
					.unwrap()
				{
					update
				} else {
					continue
				};

				let encoded = light_client_update.encode();
				let decoded = LightClientUpdate::decode(&mut &*encoded).unwrap();
				assert_eq!(light_client_update, decoded);

				client_state =
					verify_sync_committee_attestation(client_state.clone(), light_client_update)
						.unwrap();
				debug!(
					target: "prover",
					"Sucessfully verified Ethereum block at slot {:?}",
					client_state.finalized_header.slot
				);

				count += 1;
				// For CI purposes we test finalization of 3 epochs
				if count == 4 {
					break
				}
			},
			Err(err) => {
				panic!("Encountered Error and closed stream {err:?}");
			},
			_ => continue,
		}
	}
}

#[ignore]
#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn test_sync_committee_signature_verification() {
	let sync_committee_prover = setup_prover();
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

	let non_participant_pubkeys = block
		.body
		.sync_aggregate
		.sync_committee_bits
		.iter()
		.zip(sync_committee_pubkeys.iter())
		.filter_map(|(bit, key)| if !(*bit) { Some(key.clone()) } else { None })
		.collect::<Vec<_>>();

	let fork_version = compute_fork_version(compute_epoch_at_slot(block.slot));

	let domain = compute_domain(
		DOMAIN_SYNC_COMMITTEE,
		Some(fork_version),
		Some(Root::from_bytes(GENESIS_VALIDATORS_ROOT.try_into().unwrap())),
		GENESIS_FORK_VERSION,
	)
	.unwrap();

	let signing_root = compute_signing_root(&mut attested_header, domain).unwrap();

	verify_aggregate_signature(
		&sync_committee.aggregate_public_key,
		&non_participant_pubkeys,
		signing_root.as_bytes().to_vec(),
		&block.body.sync_aggregate.sync_committee_signature,
	)
	.unwrap();
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventResponse {
	pub block: Root,
	pub state: Root,
	pub epoch: String,
	pub execution_optimistic: bool,
}

fn setup_prover() -> SyncCommitteeProver {
	dotenv::dotenv().ok();
	let consensus_url =
		std::env::var("CONSENSUS_NODE_URL").unwrap_or("http://localhost:3500".to_string());
	SyncCommitteeProver::new(consensus_url)
}
