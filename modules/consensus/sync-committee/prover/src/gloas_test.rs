//! Tests against a beacon chain that has already forked to Gloas. Point `CONSENSUS_NODE_URL` and
//! `EXECUTION_NODE_URL` at an ethpandaops glamsterdam devnet, or a local devnet running the same
//! preset, and run with `--features glamsterdam --ignored`.

use super::*;
use ssz_rs::{is_valid_merkle_branch, Merkleized};
use sync_committee_primitives::{
	constants::{
		devnet::GlamsterdamDevnet, ETH1_DATA_VOTES_BOUND_ETH, PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
	},
	execution_header::{execution_block_hash, ExecutionHeader},
	util::compute_epoch_at_slot,
};
use sync_committee_verifier::verify_sync_committee_attestation;

fn setup_prover() -> SyncCommitteeProver<
	GlamsterdamDevnet,
	ETH1_DATA_VOTES_BOUND_ETH,
	PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
> {
	dotenv::dotenv().ok();
	let consensus_url =
		std::env::var("CONSENSUS_NODE_URL").unwrap_or("http://localhost:53001".to_string());
	let execution_url =
		std::env::var("EXECUTION_NODE_URL").unwrap_or("http://localhost:8545".to_string());

	SyncCommitteeProver::<
		GlamsterdamDevnet,
		ETH1_DATA_VOTES_BOUND_ETH,
		PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
	>::new(vec![consensus_url], execution_url)
}

/// The whole point of this one is the container layout. If a single field of the Gloas
/// `BeaconState` is out of order, wrongly sized or missing, the root will not match and every
/// proof the prover generates would be rejected on chain.
#[tokio::test]
#[ignore]
async fn beacon_state_hashes_to_the_signed_header() {
	let prover = setup_prover();
	let mut state = prover.fetch_beacon_state("finalized").await.unwrap();
	let header = prover.fetch_header(&state.slot.to_string()).await.unwrap();

	assert_eq!(state.hash_tree_root().unwrap(), header.state_root);
}

/// The execution state root is no longer proven directly, so this walks the path that replaces it:
/// the beacon state commits to a block hash, the block hash is the keccak of the header, and the
/// header carries the state root.
#[tokio::test]
#[ignore]
async fn execution_header_recovers_the_execution_state_root() {
	let prover = setup_prover();
	let mut finalized_state = prover.fetch_beacon_state("finalized").await.unwrap();
	let finalized_header = prover.fetch_header(&finalized_state.slot.to_string()).await.unwrap();

	let block_hash = H256::from_slice(finalized_state.latest_block_hash.as_slice());
	let header = prover.fetch_execution_header(block_hash).await.unwrap();

	let proof = prove_execution_payload::<
		GlamsterdamDevnet,
		ETH1_DATA_VOTES_BOUND_ETH,
		PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
	>(&mut finalized_state, header)
	.unwrap();

	// the header we ship is the preimage of the block hash the beacon state committed to
	assert_eq!(execution_block_hash(&proof.execution_header), block_hash.0);

	// and that block hash really does sit inside the state the sync committee signed over
	assert!(is_valid_merkle_branch(
		&Node::from_bytes(execution_block_hash(&proof.execution_header)),
		proof.execution_payload_branch.iter(),
		GlamsterdamDevnet::EXECUTION_PAYLOAD_INDEX_LOG2 as usize,
		GlamsterdamDevnet::EXECUTION_PAYLOAD_INDEX as usize,
		&finalized_header.state_root,
	));

	// so the fields the bridge consumes can be read straight off the header
	let decoded = ExecutionHeader::decode(&proof.execution_header).unwrap();
	assert_eq!(decoded.state_root.as_slice(), proof.state_root.as_bytes());
	assert_eq!(decoded.number, proof.block_number);
	assert_eq!(decoded.timestamp, proof.timestamp);
}

/// The one that runs the code that actually ships. The two tests above check the pieces in
/// isolation; this drives a real Gloas update through `verify_sync_committee_attestation`, which
/// is where the block hash branch and the keccak preimage check run on chain.
///
/// It bootstraps a trusted state from a finalized checkpoint a few epochs back, then verifies the
/// update that advances to the current finalized checkpoint. The older checkpoint is looked up via
/// the state endpoint, which tolerates skipped slots, and its block is fetched by root, which does
/// not 404, so the test is one shot rather than polling for a fresh finalization.
#[tokio::test]
#[ignore]
async fn verifier_accepts_a_real_gloas_update() -> anyhow::Result<()> {
	let prover = setup_prover();

	let block_id = |root: Root| format!("0x{}", hex::encode(root.0));

	let current = prover.fetch_finalized_checkpoint(None).await?.finalized;
	let current_header = prover.fetch_header(&block_id(current.root)).await?;

	// A few epochs back, comfortably inside the same sync committee period.
	let trusted_slot = current_header.slot.saturating_sub(3 * GlamsterdamDevnet::SLOTS_PER_EPOCH);
	let trusted = prover
		.fetch_finalized_checkpoint(Some(&trusted_slot.to_string()))
		.await?
		.finalized;
	let trusted_header = prover.fetch_header(&block_id(trusted.root)).await?;
	let trusted_state_state = prover.fetch_beacon_state(&trusted_header.slot.to_string()).await?;

	let trusted_state = VerifierState {
		finalized_header: trusted_header.clone(),
		latest_finalized_epoch: compute_epoch_at_slot::<GlamsterdamDevnet>(trusted_header.slot),
		current_sync_committee: trusted_state_state.current_sync_committee,
		next_sync_committee: trusted_state_state.next_sync_committee,
		state_period: compute_sync_committee_period_at_slot::<GlamsterdamDevnet>(
			trusted_header.slot,
		),
	};

	let update = prover
		.fetch_light_client_update(trusted_state.clone(), current, None)
		.await?
		.ok_or_else(|| {
			anyhow::anyhow!("no update produced between the two finalized checkpoints")
		})?;

	let new_state =
		verify_sync_committee_attestation::<GlamsterdamDevnet>(trusted_state, update.clone())
			.map_err(|e| anyhow::anyhow!("verifier rejected a valid gloas update: {e:?}"))?;

	assert_eq!(new_state.finalized_header, update.finalized_header);
	Ok(())
}
