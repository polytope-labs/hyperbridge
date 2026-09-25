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

use crate::ArcHost;
use arc_primitives::{VerifierState, VerifierStateUpdate};
use arc_prover::{header_hash, Keccak256Hasher};
use arc_verifier::{verify_arc_update, verify_certificate};
use codec::Decode;
use ismp_arc::ConsensusState;
use std::sync::Arc;
use tesseract_primitives::IsmpProvider;

/// How many heights to walk back looking for a certificate that the on-chain
/// validator set still signs, before giving up until the next tick.
const MAX_WALK_BACK: u64 = 512;

/// Notification logic for the Arc relayer.
///
/// Finds the highest finalized block whose commit certificate verifies against
/// the counterparty's trusted validator set. When the active set rotated since
/// the last submitted update, newer certificates are signed by a set the
/// client hasn't adopted yet, so we walk back towards the trusted height until
/// a certificate matches; submitting it advances the client past the rotation.
pub async fn consensus_notification(
	client: &ArcHost,
	counterparty: Arc<dyn IsmpProvider>,
) -> anyhow::Result<Option<VerifierStateUpdate>> {
	let latest_height = client.prover.latest_height().await?;

	let consensus_state_serialized: Vec<u8> =
		counterparty.query_consensus_state(None, client.consensus_state_id).await?;
	let consensus_state: ConsensusState =
		ConsensusState::decode(&mut &consensus_state_serialized[..])?;
	let trusted: VerifierState = consensus_state.into();

	if latest_height <= trusted.finalized_height {
		log::trace!(target: crate::LOG_TARGET, "No new finalized blocks");
		return Ok(None);
	}

	// Primary path: a tip-anchored update, which works against RPC nodes with
	// reth's default zero `eth_getProof` window. Fails verification only when
	// the validator set rotated past the client's trusted set.
	match client.prover.fetch_latest_update().await {
		Ok(update) if update.certificate.height > trusted.finalized_height =>
			match verify_arc_update::<Keccak256Hasher>(trusted.clone(), update.clone()) {
				Ok(_) => return Ok(Some(update)),
				Err(e) => log::debug!(
					target: crate::LOG_TARGET,
					"Tip update doesn't verify against the trusted validator set ({e}), \
					 walking back to the rotation boundary"
				),
			},
		Ok(_) => return Ok(None),
		Err(e) => log::debug!(
			target: crate::LOG_TARGET,
			"Failed to capture a tip-anchored update ({e}), walking back"
		),
	}

	// Fallback: walk back towards the trusted height looking for a
	// certificate the trusted set still signs. Requires an RPC node with a
	// proof window covering the target blocks.
	let stop_height = trusted.finalized_height.max(latest_height.saturating_sub(MAX_WALK_BACK));
	let mut height = latest_height;
	while height > stop_height {
		// Fetch just the header and certificate first; the validator set
		// proof is only worth fetching for a certificate we can verify.
		let certificate = match client.prover.fetch_certificate(height).await {
			Ok(certificate) => certificate,
			Err(e) => {
				log::trace!(
					target: crate::LOG_TARGET,
					"No certificate at {height}, walking back: {e}"
				);
				height -= 1;
				continue;
			},
		};
		let header = client.prover.rpc.get_block_by_number(height).await?;

		if header_hash(&header) != certificate.block_hash ||
			verify_certificate(&trusted.current_validators, &certificate).is_err()
		{
			log::trace!(
				target: crate::LOG_TARGET,
				"Certificate at {height} doesn't verify against the trusted validator set, walking back"
			);
			height -= 1;
			continue;
		}

		let validator_set_proof = client.prover.fetch_validator_set_proof(height).await?;
		let update = VerifierStateUpdate { header, certificate, validator_set_proof };

		// Dry-run the full on-chain verification before submitting.
		verify_arc_update::<Keccak256Hasher>(trusted.clone(), update.clone())?;

		return Ok(Some(update));
	}

	log::error!(
		target: crate::LOG_TARGET,
		"Failed to find a certificate verifiable against the on-chain validator set \
		 within {MAX_WALK_BACK} blocks of the tip"
	);
	Ok(None)
}
