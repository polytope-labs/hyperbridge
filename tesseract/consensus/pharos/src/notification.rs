// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Consensus notification logic for Pharos relayer.

use crate::PharosHost;
use codec::Decode;
use ismp_pharos::ConsensusState;
use pharos_primitives::{Config, VerifierStateUpdate};
use std::sync::Arc;
use tesseract_primitives::IsmpProvider;

pub async fn consensus_notification<C: Config>(
	client: &PharosHost<C>,
	counterparty: Arc<dyn IsmpProvider>,
) -> Result<Option<VerifierStateUpdate>, anyhow::Error> {
	let counterparty_finalized = counterparty.query_finalized_height().await?;
	let consensus_state_bytes = counterparty
		.query_consensus_state(Some(counterparty_finalized), client.consensus_state_id)
		.await?;

	let consensus_state = ConsensusState::decode(&mut &consensus_state_bytes[..])
		.map_err(|e| anyhow::anyhow!("Failed to decode consensus state: {:?}", e))?;

	let latest_block = client.prover.get_latest_block().await?;

	if latest_block <= consensus_state.finalized_height {
		log::trace!(
			target: "tesseract-pharos",
			"No new blocks to sync. Latest: {}, Finalized: {}",
			latest_block,
			consensus_state.finalized_height
		);
		return Ok(None);
	}

	let current_epoch = consensus_state.current_epoch;
	let latest_epoch = C::compute_epoch(latest_block);

	log::trace!(
		target: "tesseract-pharos",
		"New block available. Latest: {} (epoch {}), Finalized: {} (epoch {})",
		latest_block,
		latest_epoch,
		consensus_state.finalized_height,
		current_epoch
	);

	// Determine the target block for the update
	// If we're still in the same epoch, just get the latest block
	// If we've crossed epoch boundaries, we need to sync epoch by epoch
	let target_block = if latest_epoch > current_epoch {
		// Find the first epoch boundary block we need to sync
		// Epoch boundary is at (epoch + 1) * epoch_length - 1
		let next_epoch_boundary = (current_epoch + 1) * C::EPOCH_LENGTH_BLOCKS - 1;
		if next_epoch_boundary <= latest_block {
			log::trace!(
				target: "tesseract-pharos",
				"Syncing epoch boundary block {} for epoch transition {} -> {}",
				next_epoch_boundary,
				current_epoch,
				current_epoch + 1
			);
			next_epoch_boundary
		} else {
			latest_block
		}
	} else {
		latest_block
	};

	let update = client.prover.fetch_block_update(target_block).await?;

	log::trace!(
		target: "tesseract-pharos",
		"Fetched update for block {}{}",
		target_block,
		if update.validator_set_proof.is_some() { " (with validator set proof)" } else { "" }
	);

	Ok(Some(update))
}
