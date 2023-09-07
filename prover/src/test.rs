use super::*;
use base2::Base2;
use ethers::{
	prelude::{Http, Middleware, ProviderExt},
	providers::Provider,
};
use ssz_rs::{
	calculate_multi_merkle_root, get_generalized_index, is_valid_merkle_branch, GeneralizedIndex,
	Merkleized, SszVariableOrIndex,
};
use std::time::Duration;
use sync_committee_primitives::{
	constants::{Root, DOMAIN_SYNC_COMMITTEE, GENESIS_FORK_VERSION, GENESIS_VALIDATORS_ROOT},
	types::LightClientState,
	util::{compute_domain, compute_fork_version, compute_signing_root},
};
use sync_committee_verifier::{
	signature_verification::verify_aggregate_signature, verify_sync_committee_attestation,
};
use tokio::time;
use tokio_stream::{wrappers::IntervalStream, StreamExt};

const CONSENSUS_NODE_URL: &'static str = "http://localhost:3500";
const EL_NODE_URL: &'static str = "http://localhost:8545";

async fn wait_for_el() {
	let provider = Provider::<Http>::connect(EL_NODE_URL).await;
	let sub = provider.watch_blocks().await.unwrap();
	let _ = sub.take(10).collect::<Vec<_>>();
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_block_header_works() {
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());
	let block_header = sync_committee_prover.fetch_header("head").await;
	assert!(block_header.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_block_works() {
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());
	let block = sync_committee_prover.fetch_block("head").await;
	assert!(block.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_validator_works() {
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());
	let validator = sync_committee_prover.fetch_validator("head", "0").await;
	assert!(validator.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_processed_sync_committee_works() {
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());
	let validator = sync_committee_prover.fetch_processed_sync_committee("head").await;
	assert!(validator.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn generate_indexes() {
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());
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
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_beacon_state_works() {
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());
	let beacon_state = sync_committee_prover.fetch_beacon_state("head").await;
	assert!(beacon_state.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn state_root_and_block_header_root_matches() {
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());
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
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());
	let finality_checkpoint = sync_committee_prover.fetch_finalized_checkpoint().await;
	assert!(finality_checkpoint.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn test_finalized_header() {
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());
	let mut state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();

	let proof = ssz_rs::generate_proof(&mut state, &vec![FINALIZED_ROOT_INDEX as usize]);

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
#[tokio::test]
async fn test_execution_payload_proof() {
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());

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
		EXECUTION_PAYLOAD_INDEX.floor_log2() as usize,
		GeneralizedIndex(EXECUTION_PAYLOAD_INDEX as usize).0,
		&finalized_header.state_root,
	);

	assert!(is_merkle_branch_valid);
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn test_sync_committee_update_proof() {
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());

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
		&Node::from_bytes(sync_committee.hash_tree_root().unwrap().as_ref().try_into().unwrap()),
		sync_committee_proof.iter(),
		NEXT_SYNC_COMMITTEE_INDEX.floor_log2() as usize,
		NEXT_SYNC_COMMITTEE_INDEX as usize,
		&Node::from_bytes(finalized_header.state_root.as_ref().try_into().unwrap()),
	);

	assert!(is_merkle_branch_valid);
}

#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn test_prover() {
	use log::LevelFilter;
	env_logger::builder()
		.filter_module("prover", LevelFilter::Debug)
		.format_module_path(false)
		.init();
	wait_for_el().await;
	let mut stream = IntervalStream::new(time::interval(Duration::from_secs(12 * 12)));

	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());

	let block_id = "head";

	let block_header = sync_committee_prover.fetch_header(&block_id).await.unwrap();

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
	while let Some(_ts) = stream.next().await {
		let light_client_update = if let Some(update) = sync_committee_prover
			.fetch_light_client_update(client_state.clone(), "prover")
			.await
			.unwrap()
		{
			update
		} else {
			continue
		};

		client_state =
			verify_sync_committee_attestation(client_state.clone(), light_client_update).unwrap();
		debug!(
			target: "prover",
			"Sucessfully verified Ethereum block at slot {:?}",
			client_state.finalized_header.slot
		);

		count += 1;
		// For CI purposes we test finalization of three epochs
		if count == 3 {
			break
		}
	}
}

#[ignore]
#[cfg(test)]
#[allow(non_snake_case)]
#[tokio::test]
async fn test_sync_committee_signature_verification() {
	wait_for_el().await;
	let sync_committee_prover = SyncCommitteeProver::new(CONSENSUS_NODE_URL.to_string());
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
