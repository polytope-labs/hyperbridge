//! Builds the consensus update to submit for a tick.

use crate::KaiaHost;
use anyhow::Context;
use codec::Decode;
use kaia_verifier::primitives::{ConsensusState, KaiaClientUpdate};
use std::sync::Arc;
use tesseract_primitives::IsmpProvider;

/// Computes the consensus update advancing the on-chain client toward the
/// latest finalized block, if it has fallen behind.
///
/// Kaia finalizes every block, so a single update suffices: the target header
/// plus any headers in the gap that the client must not skip (validator votes
/// and epoch governance). While catching up after downtime the scanned gap is
/// capped, advancing over multiple ticks.
pub async fn consensus_update(
	client: &KaiaHost,
	counterparty: Arc<dyn IsmpProvider>,
) -> anyhow::Result<Option<KaiaClientUpdate>> {
	let consensus_state_serialized =
		counterparty.query_consensus_state(None, client.consensus_state_id).await?;
	let consensus_state = ConsensusState::decode(&mut &consensus_state_serialized[..])
		.context("failed to decode on-chain ConsensusState")?;

	let latest = client.prover.latest_block_number().await?;
	if latest <= consensus_state.finalized_height {
		log::trace!(target: crate::LOG_TARGET, "No new finalized headers");
		return Ok(None);
	}

	let target = latest.min(consensus_state.finalized_height + client.max_scan_range());
	if target < latest {
		log::info!(
			target: crate::LOG_TARGET,
			"Catching up: advancing to {target}, {} blocks remaining",
			latest - target,
		);
	}

	client.prover.fetch_kaia_update(&consensus_state, target).await
}
