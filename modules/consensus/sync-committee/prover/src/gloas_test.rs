//! Tests against a beacon chain that has already forked to Gloas: the Kurtosis devnet CI starts
//! from `sync-committee-devnet.yaml`, which is on Gloas from genesis. Run with `--ignored` once it
//! has finalized a few epochs. No feature flag: the state shape is chosen from the fork the beacon
//! api reports.

use super::*;
use sync_committee_primitives::{
	constants::{
		devnet::KurtosisDevnet, ETH1_DATA_VOTES_BOUND_ETH, GLOAS_EXECUTION_PAYLOAD_INDEX,
		PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
	},
	execution_header::{execution_block_hash, ExecutionHeader},
	util::compute_epoch_at_slot,
};
use sync_committee_verifier::{error::Error, verify_sync_committee_attestation};
use tree_hash::{
	proof::{is_valid_merkle_branch, TreeHashFields},
	Hash256, TreeHash,
};

fn setup_prover() -> SyncCommitteeProver<
	KurtosisDevnet,
	ETH1_DATA_VOTES_BOUND_ETH,
	PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
> {
	dotenv::dotenv().ok();
	let consensus_url =
		std::env::var("CONSENSUS_NODE_URL").unwrap_or("http://localhost:53001".to_string());
	let execution_url =
		std::env::var("EXECUTION_NODE_URL").unwrap_or("http://localhost:52003".to_string());

	SyncCommitteeProver::<
		KurtosisDevnet,
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
	// Resolve the header first. `finalized` always names a block, whereas the finalized state's
	// slot may have been skipped, in which case there is no header to fetch at that slot number.
	let header = prover.fetch_header("finalized").await.unwrap();
	let mut state = prover.fetch_beacon_state(&header.slot.to_string()).await.unwrap();

	assert_eq!(state.tree_hash_root(), Hash256::from(&header.state_root));
}

/// The execution state root is no longer proven directly, so this walks the path that replaces it:
/// the beacon state commits to a block hash, the block hash is the keccak of the header, and the
/// header carries the state root.
#[tokio::test]
#[ignore]
async fn execution_header_recovers_the_execution_state_root() {
	let prover = setup_prover();
	// As above: go through the header so a skipped finalized slot does not 404.
	let finalized_header = prover.fetch_header("finalized").await.unwrap();
	let mut finalized_state =
		prover.fetch_beacon_state(&finalized_header.slot.to_string()).await.unwrap();

	let block_hash = H256::from_slice(finalized_state.execution_block_hash().as_ref());
	let header = prover.fetch_execution_header(block_hash).await.unwrap();

	let proof = prove_execution_payload::<
		KurtosisDevnet,
		ETH1_DATA_VOTES_BOUND_ETH,
		PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
	>(&finalized_state, Some(header.clone()))
	.unwrap();

	let execution_header = proof.execution_header().expect("gloas proof carries the rlp header");

	// the header we ship is the preimage of the block hash the beacon state committed to
	assert_eq!(execution_block_hash(execution_header), block_hash.0);

	// and that block hash really does sit inside the state the sync committee signed over
	let payload_branch: Vec<Hash256> =
		proof.execution_payload_branch.iter().map(Into::into).collect();
	assert!(is_valid_merkle_branch(
		Hash256::from(execution_block_hash(execution_header)),
		&payload_branch,
		GLOAS_EXECUTION_PAYLOAD_INDEX,
		Hash256::from(&finalized_header.state_root),
	));

	// so the fields the bridge consumes can be read straight off the header
	let decoded = ExecutionHeader::decode(execution_header).unwrap();
	assert_eq!(decoded, header);
}

/// Trust the current finalized checkpoint and wait for the next one, then produce the real update
/// that advances to it. Only states from the latest finalized checkpoint onwards are needed, which
/// is all a node that does not keep historical states can serve.
async fn bootstrap_trusted_state_and_update(
	prover: &SyncCommitteeProver<
		KurtosisDevnet,
		ETH1_DATA_VOTES_BOUND_ETH,
		PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
	>,
) -> anyhow::Result<(VerifierState, VerifierStateUpdate)> {
	let block_id = |root: Root| format!("0x{}", hex::encode(root.as_ref()));

	let trusted = prover.fetch_finalized_checkpoint(None).await?.finalized;
	let trusted_header = prover.fetch_header(&block_id(trusted.root.clone())).await?;
	let trusted_beacon_state = prover.fetch_beacon_state(&trusted_header.slot.to_string()).await?;

	let trusted_state = VerifierState {
		finalized_header: trusted_header.clone(),
		latest_finalized_epoch: compute_epoch_at_slot::<KurtosisDevnet>(trusted_header.slot),
		current_sync_committee: trusted_beacon_state.current_sync_committee().clone(),
		next_sync_committee: trusted_beacon_state.next_sync_committee().clone(),
		state_period: compute_sync_committee_period_at_slot::<KurtosisDevnet>(trusted_header.slot),
	};

	let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10 * 60);
	loop {
		if std::time::Instant::now() > deadline {
			anyhow::bail!("no finalization past epoch {} in 10 minutes", trusted.epoch)
		}
		tokio::time::sleep(std::time::Duration::from_secs(8)).await;

		let current = prover.fetch_finalized_checkpoint(None).await?.finalized;
		if current.epoch <= trusted.epoch {
			continue
		}
		if let Some(update) =
			prover.fetch_light_client_update(trusted_state.clone(), current, None).await?
		{
			return Ok((trusted_state, update))
		}
	}
}

/// The one that runs the code that actually ships. The two tests above check the pieces in
/// isolation; this drives a real Gloas update through `verify_sync_committee_attestation`, which
/// is where the block hash branch and the keccak preimage check run on chain.
#[tokio::test]
#[ignore]
async fn verifier_accepts_a_real_gloas_update() -> anyhow::Result<()> {
	let prover = setup_prover();
	let (trusted_state, update) = bootstrap_trusted_state_and_update(&prover).await?;

	let (new_state, execution_payload) =
		verify_sync_committee_attestation::<KurtosisDevnet>(trusted_state, update.clone())
			.map_err(|e| anyhow::anyhow!("verifier rejected a valid gloas update: {e:?}"))?;

	assert_eq!(new_state.finalized_header, update.finalized_header);

	// the fields the caller gets back are the header's own
	let header =
		ExecutionHeader::decode(update.execution_payload.execution_header().expect("gloas proof"))?;
	assert_eq!(execution_payload.state_root.as_bytes(), header.state_root.as_slice());
	assert_eq!(execution_payload.block_number, header.number);
	assert_eq!(execution_payload.timestamp, header.timestamp);
	Ok(())
}

/// The security of the whole approach rests on keccak binding the execution header to the block
/// hash the sync committee signed. This tampers with a real, otherwise valid update to make sure
/// that binding rejects, both for a header that no longer decodes and for a well formed one that
/// describes a different block.
#[tokio::test]
#[ignore]
async fn verifier_rejects_tampered_gloas_updates() -> anyhow::Result<()> {
	let prover = setup_prover();
	let (trusted_state, update) = bootstrap_trusted_state_and_update(&prover).await?;

	// Flipping a byte of the header changes its keccak, so it no longer matches the block hash the
	// branch proves against.
	let mut tampered_header = update.clone();
	tampered_header.execution_payload.execution_header_mut().expect("gloas proof")[0] ^= 0xff;
	assert!(
		matches!(
			verify_sync_committee_attestation::<KurtosisDevnet>(
				trusted_state.clone(),
				tampered_header,
			),
			Err(Error::InvalidMerkleBranch(_))
		),
		"a header whose keccak does not match the block hash must be rejected",
	);

	// Lying about the state root means re-encoding the header. The bytes still decode, but they
	// hash to a different block, so the branch catches it.
	let mut tampered_root = update.clone();
	let header_bytes = tampered_root.execution_payload.execution_header_mut().expect("gloas proof");
	let mut header = ExecutionHeader::decode(&header_bytes[..])?;
	header.state_root = Default::default();
	*header_bytes = header.encode();
	assert!(
		matches!(
			verify_sync_committee_attestation::<KurtosisDevnet>(trusted_state, tampered_root),
			Err(Error::InvalidMerkleBranch(_))
		),
		"a state root that disagrees with the block hash must be rejected",
	);

	Ok(())
}

/// Follow a devnet through its Gloas fork the way the on chain client does, verifying each new
/// finalized update against the state the previous one produced. Across the fork the attested
/// state can already be Gloas while the finalized one is not, and the finality and sync committee
/// branches are then proven at the Gloas indices while the execution branch still uses the legacy
/// one. The CI devnet is on Gloas from genesis, so this needs a local devnet whose Gloas epoch is
/// a few epochs after genesis instead: set `gloas_fork_epoch` in `sync-committee-devnet.yaml` and
/// `KurtosisDevnet::GLOAS_FORK_EPOCH` to the same epoch, start this before the fork, and it
/// returns once it has verified an update on each side and one straddling the boundary. The first
/// finalized checkpoint after the fork is the one to watch: its state still commits to the last
/// execution block from before Amsterdam.
#[tokio::test]
#[ignore]
async fn verifier_follows_the_chain_across_the_gloas_fork() -> anyhow::Result<()> {
	let prover = setup_prover();
	let block_id = |root: Root| format!("0x{}", hex::encode(root.as_ref()));
	let is_gloas = |slot: u64| {
		compute_epoch_at_slot::<KurtosisDevnet>(slot) >= KurtosisDevnet::GLOAS_FORK_EPOCH
	};

	let mut trusted_state: Option<VerifierState> = None;
	let (mut before, mut straddling, mut after) = (false, false, false);
	let deadline = std::time::Instant::now() + std::time::Duration::from_secs(40 * 60);

	while !(before && straddling && after) {
		if std::time::Instant::now() > deadline {
			anyhow::bail!(
				"timed out, verified before={before} straddling={straddling} after={after}; \
				 start this before the fork"
			);
		}
		tokio::time::sleep(std::time::Duration::from_secs(8)).await;

		let current = prover.fetch_finalized_checkpoint(None).await?.finalized;
		if current.epoch == 0 {
			continue
		}

		let Some(state) = trusted_state.clone() else {
			let header = prover.fetch_header(&block_id(current.root.clone())).await?;
			let beacon_state = prover.fetch_beacon_state(&header.slot.to_string()).await?;
			println!("trusting finalized epoch {} at slot {}", current.epoch, header.slot);
			trusted_state = Some(VerifierState {
				latest_finalized_epoch: compute_epoch_at_slot::<KurtosisDevnet>(header.slot),
				current_sync_committee: beacon_state.current_sync_committee().clone(),
				next_sync_committee: beacon_state.next_sync_committee().clone(),
				state_period: compute_sync_committee_period_at_slot::<KurtosisDevnet>(header.slot),
				finalized_header: header,
			});
			continue
		};

		if current.epoch <= state.latest_finalized_epoch {
			continue
		}

		let Some(update) = prover.fetch_light_client_update(state.clone(), current, None).await?
		else {
			continue
		};

		let attested_gloas = is_gloas(update.attested_header.slot);
		let finalized_gloas = is_gloas(update.finalized_header.slot);
		assert_eq!(
			update.execution_payload.execution_header().is_some(),
			finalized_gloas,
			"the prover built a proof for the wrong side of the fork"
		);

		let (new_state, _) =
			verify_sync_committee_attestation::<KurtosisDevnet>(state, update.clone()).map_err(
				|e| {
					anyhow::anyhow!(
				"verifier rejected the update attested at slot {} (gloas {attested_gloas}) \
				 finalizing slot {} (gloas {finalized_gloas}): {e:?}",
				update.attested_header.slot,
				update.finalized_header.slot,
			)
				},
			)?;

		println!(
			"verified update attested at slot {} (gloas {attested_gloas}) finalizing slot {} \
			 (gloas {finalized_gloas})",
			update.attested_header.slot, update.finalized_header.slot,
		);
		match (attested_gloas, finalized_gloas) {
			(false, false) => before = true,
			(true, false) => straddling = true,
			(true, true) => after = true,
			(false, true) =>
				unreachable!("the finalized header cannot be newer than the attested one"),
		}
		trusted_state = Some(new_state);
	}

	Ok(())
}
