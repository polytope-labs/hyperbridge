//! Builds the ordered list of consensus updates to submit for a tick.

use crate::TempoHost;
use anyhow::{anyhow, Context};
use codec::Decode;
use std::sync::Arc;
use tempo_verifier::primitives::{compute_epoch, ConsensusState, TempoClientUpdate};
use tesseract_primitives::IsmpProvider;

/// Maximum updates submitted in a single tick. Each is its own transaction, so
/// a client that has fallen far behind catches up over several ticks rather
/// than building an unbounded batch.
const MAX_UPDATES_PER_TICK: usize = 16;

/// Computes the consensus updates needed to advance the on-chain client toward
/// the latest finalized block.
///
/// The client refuses to cross an epoch boundary it has not verified — that is
/// what forces every identity rotation through it — so this walks the boundary
/// blocks between the client's epoch and the tip, in order, and submits each
/// one before the tip itself.
pub async fn consensus_updates(
	client: &TempoHost,
	counterparty: Arc<dyn IsmpProvider>,
) -> anyhow::Result<Vec<TempoClientUpdate>> {
	let consensus_state_serialized =
		counterparty.query_consensus_state(None, client.consensus_state_id).await?;
	let consensus_state = ConsensusState::decode(&mut &consensus_state_serialized[..])
		.context("failed to decode on-chain ConsensusState")?;

	let latest = client.prover.finalization(None).await?;
	if latest.height <= consensus_state.finalized_height {
		log::trace!(target: crate::LOG_TARGET, "No new finalization");
		return Ok(vec![]);
	}

	let epoch_length = consensus_state.epoch_length;
	if epoch_length == 0 {
		return Err(anyhow!("on-chain consensus state has a zero epoch length"));
	}

	let client_epoch = compute_epoch(consensus_state.finalized_height, epoch_length);
	let latest_epoch = compute_epoch(latest.height, epoch_length);

	let mut updates = Vec::new();
	for epoch in client_epoch..latest_epoch {
		let boundary_height = (epoch + 1) * epoch_length - 1;
		if boundary_height <= consensus_state.finalized_height {
			// Already verified — the client is sitting on or past this boundary.
			continue;
		}
		if updates.len() >= MAX_UPDATES_PER_TICK {
			log::info!(
				target: crate::LOG_TARGET,
				"Catching up: {} boundary updates this tick, resuming from height {boundary_height} next tick",
				updates.len(),
			);
			return Ok(updates);
		}
		log::debug!(
			target: crate::LOG_TARGET,
			"Submitting epoch {epoch} boundary block {boundary_height}",
		);
		updates.push(update_for(client, boundary_height).await?);
	}

	// The tip itself, unless a boundary walk already covered it.
	if updates.last().map(|u| u.header.inner.number.low_u64()) != Some(latest.height) {
		updates.push(TempoClientUpdate {
			finalization: latest.finalization,
			header: client.prover.header(latest.height).await?,
		});
	}

	Ok(updates)
}

/// Builds a [`TempoClientUpdate`] for the given height from its archived
/// finalization certificate.
async fn update_for(client: &TempoHost, height: u64) -> anyhow::Result<TempoClientUpdate> {
	let finalization = client.prover.finalization(Some(height)).await.with_context(|| {
		format!(
			"failed to fetch the finalization for height {height}; the RPC node's archive must \
			 retain boundary certificates for the light client to follow epoch transitions"
		)
	})?;
	if finalization.height != height {
		return Err(anyhow!(
			"finalization height mismatch: requested {height}, got {}",
			finalization.height
		));
	}
	Ok(TempoClientUpdate {
		finalization: finalization.finalization,
		header: client.prover.header(height).await?,
	})
}
