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

//! Periodic task that claims outbound consensus delivery rewards.
//!
//! Each time the relayer delivers a mandatory BEEFY rotation to an EVM destination, the delivery
//! path writes a row to the local DB. This task wakes on a fixed interval, reads those rows,
//! skips anything already claimed on Hyperbridge, and submits the remaining claims in parallel.

use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

use anyhow::{anyhow, Context as _};
use codec::Decode;
use futures::{stream::FuturesUnordered, StreamExt};
use ismp::{consensus::StateMachineHeight, host::StateMachine, messaging::Proof};
use pallet_ismp_relayer::{
	outbound_consensus_delivery_message, OutboundConsensusDeliveryClaim, EVM_HOST_EPOCHS_SLOT,
};
use primitive_types::{H160, U256};
use sp_core::Pair;
use subxt_utils::outbound_consensus_rotations_claimed_storage_key;
use tesseract_evm::derive_map_key;
use tesseract_primitives::{
	observe_challenge_period, IsmpHost, IsmpProvider, PendingConsensusDeliveryClaim,
	StateProofQueryType,
};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use tokio::time::MissedTickBehavior;
use tracing::Instrument;
use transaction_fees::TransactionPayment;

const LOG_TARGET: &str = "messaging-outbound-claim";

pub async fn run(
	hyperbridge: SubstrateClient<KeccakSubstrateChain>,
	destinations: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	consensus_hosts: HashMap<StateMachine, Arc<dyn IsmpHost>>,
	tx_payment: Option<Arc<TransactionPayment>>,
) -> Result<(), anyhow::Error> {
	let hb_provider: Arc<dyn IsmpProvider> = Arc::new(hyperbridge.clone());
	let payee_bytes: [u8; 32] = hyperbridge.signer.public().0;

	let mut interval = tokio::time::interval(Duration::from_secs(crate::CLAIM_INTERVAL_SECS));
	interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

	loop {
		interval.tick().await;

		let pending = read_pending_claims(&tx_payment).await;
		if pending.is_empty() {
			continue;
		}

		let (claimed, unclaimed) = match partition_claimed(&hyperbridge, &pending).await {
			Ok(parts) => parts,
			Err(err) => {
				tracing::warn!(
					target: LOG_TARGET,
					?err,
					"could not check claimed status on Hyperbridge; processing all rows without filtering",
				);
				(Vec::new(), pending)
			},
		};

		if !claimed.is_empty() {
			tracing::info!(
				target: LOG_TARGET,
				count = claimed.len(),
				"dropping claims already redeemed on Hyperbridge",
			);
			if let Some(tp) = &tx_payment {
				for row in &claimed {
					if let Err(err) =
						tp.delete_rotation_claim(&row.destination.to_string(), row.set_id).await
					{
						tracing::warn!(
							target: LOG_TARGET,
							?err,
							destination = %row.destination,
							set_id = row.set_id,
							"failed to delete already-claimed row",
						);
					}
				}
			}
		}

		// Bring Hyperbridge's view of any lagging destination up to the highest
		// pending delivery height before fanning out — once per destination,
		// not per claim, so concurrent claims don't submit byte-identical
		// consensus proofs that collide in Hyperbridge's transaction pool.
		advance_lagging_destinations(
			&hb_provider,
			&destinations,
			&consensus_hosts,
			unclaimed.iter().map(|c| (c.destination, c.delivery_height)),
		)
		.await;

		let mut tasks = FuturesUnordered::new();
		for pending in unclaimed {
			let span = tracing::info_span!(
				"outbound_claim",
				destination = %pending.destination,
				delivery_height = pending.delivery_height,
				set_id = pending.set_id,
			);
			let dest = destinations.get(&pending.destination).cloned();
			let hb = hyperbridge.clone();
			let hb_view = hb_provider.clone();
			let tp = tx_payment.clone();
			tasks.push(
				async move {
					let Some(dest) = dest else {
						tracing::warn!(target: LOG_TARGET, "no provider for destination; dropping claim");
						return;
					};
					match process_claim(&hb, hb_view, dest, &pending, payee_bytes).await {
						Ok(()) => {
							tracing::info!(target: LOG_TARGET, "claim submitted");
							if let Some(tp) = &tp {
								let _ = tp
									.delete_rotation_claim(
										&pending.destination.to_string(),
										pending.set_id,
									)
									.await;
							}
						},
						Err(err) => {
							tracing::error!(
								target: LOG_TARGET,
								?err,
								"claim failed; will retry on next tick",
							);
						},
					}
				}
				.instrument(span),
			);
		}

		while tasks.next().await.is_some() {}
	}
}

async fn read_pending_claims(
	tx_payment: &Option<Arc<TransactionPayment>>,
) -> Vec<PendingConsensusDeliveryClaim> {
	let Some(tp) = tx_payment else { return Vec::new() };
	match tp.list_pending_rotation_claims().await {
		Ok(rows) => {
			let mut claims: Vec<PendingConsensusDeliveryClaim> = rows
				.into_iter()
				.filter_map(|row| {
					let destination = match StateMachine::from_str(&row.dest) {
						Ok(sm) => sm,
						Err(err) => {
							tracing::warn!(
								target: LOG_TARGET,
								dest = %row.dest,
								?err,
								"unparseable state machine in DB row; skipping",
							);
							return None;
						},
					};
					Some(PendingConsensusDeliveryClaim {
						destination,
						delivery_height: row.rotation_height as u64,
						set_id: row.set_id as u64,
					})
				})
				.collect();
			claims.sort_by_key(|c| (c.delivery_height, c.set_id));
			claims
		},
		Err(err) => {
			tracing::warn!(target: LOG_TARGET, ?err, "could not read pending claims; skipping tick");
			Vec::new()
		},
	}
}

pub async fn partition_claimed(
	hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
	pending: &[PendingConsensusDeliveryClaim],
) -> anyhow::Result<(Vec<PendingConsensusDeliveryClaim>, Vec<PendingConsensusDeliveryClaim>)> {
	let block_hash = hyperbridge
		.rpc
		.chain_get_block_hash(None)
		.await?
		.ok_or_else(|| anyhow!("failed to fetch latest Hyperbridge block hash"))?;

	let mut claimed = Vec::new();
	let mut unclaimed = Vec::new();

	for item in pending {
		let key = outbound_consensus_rotations_claimed_storage_key(item.destination, item.set_id);
		let raw = hyperbridge.client.storage().at(block_hash).fetch_raw(key).await.with_context(
			|| {
				format!(
					"OutboundConsensusRotationsClaimed lookup ({:?}, {})",
					item.destination, item.set_id,
				)
			},
		)?;

		// The stored value is `()` — any presence (including empty bytes from OptionQuery)
		// means the slot is taken.
		let is_claimed = match raw {
			Some(bytes) => <()>::decode(&mut &*bytes).is_ok() || bytes.is_empty(),
			None => false,
		};

		if is_claimed {
			claimed.push(item.clone());
		} else {
			unclaimed.push(item.clone());
		}
	}

	Ok((claimed, unclaimed))
}

/// Log target for [`advance_lagging_destinations`], shared by both claim
/// tasks. `tracing` requires a const target, so the helper can't log under
/// each caller's own target.
const ADVANCE_LOG_TARGET: &str = "messaging-claim-advance";

/// Submit a consensus proof for every destination whose committed height on
/// Hyperbridge is behind a pending delivery height, targeting the highest
/// height pending for that destination.
///
/// Called once per tick before the per-claim fan-out. Advancing inside each
/// claim future would submit byte-identical consensus proofs concurrently,
/// which collide in Hyperbridge's transaction pool (the unsigned extrinsic's
/// `provides` tag hashes the proof content) and fail all but one of the
/// claims for that tick; it would also submit a redundant proof even when
/// Hyperbridge is already past the delivery height.
///
/// Destinations whose consensus host can't advance the counterparty are
/// covered by the default no-op `advance_counterparty_to`. Failures here are
/// logged and left for the per-claim height check to retry on the next tick.
pub(crate) async fn advance_lagging_destinations(
	hb_provider: &Arc<dyn IsmpProvider>,
	destinations: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	consensus_hosts: &HashMap<StateMachine, Arc<dyn IsmpHost>>,
	pending: impl Iterator<Item = (StateMachine, u64)>,
) {
	let mut max_heights: HashMap<StateMachine, u64> = HashMap::new();
	for (destination, delivery_height) in pending {
		let entry = max_heights.entry(destination).or_default();
		*entry = (*entry).max(delivery_height);
	}

	for (destination, target_height) in max_heights {
		let Some(host) = consensus_hosts.get(&destination) else { continue };
		let Some(dest) = destinations.get(&destination) else { continue };

		let committed = match hb_provider.query_latest_height(dest.state_machine_id()).await {
			Ok(height) => height as u64,
			Err(err) => {
				tracing::warn!(
					target: ADVANCE_LOG_TARGET,
					%destination,
					?err,
					"could not query committed height; skipping advance this tick",
				);
				continue;
			},
		};
		if committed >= target_height {
			continue;
		}

		tracing::info!(
			target: ADVANCE_LOG_TARGET,
			%destination,
			committed,
			target_height,
			"advancing Hyperbridge's view of destination for pending claims",
		);
		if let Err(err) = host.advance_counterparty_to(hb_provider.clone(), target_height).await {
			tracing::warn!(
				target: ADVANCE_LOG_TARGET,
				%destination,
				?err,
				"failed to advance destination height; claims will retry on next tick",
			);
		}
	}
}

pub async fn process_claim(
	hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
	hb_provider: Arc<dyn IsmpProvider>,
	dest: Arc<dyn IsmpProvider>,
	pending: &PendingConsensusDeliveryClaim,
	payee: [u8; 32],
) -> anyhow::Result<()> {
	let committed = hb_provider
		.query_latest_height(dest.state_machine_id())
		.await
		.context("query_latest_height")?;

	if (committed as u64) < pending.delivery_height {
		return Err(anyhow!(
			"Hyperbridge has only committed {} for {}, delivery block {} not yet reachable",
			committed,
			pending.destination,
			pending.delivery_height,
		));
	}

	let dest_height = committed as u64;

	observe_challenge_period(dest.clone(), hb_provider.clone(), dest_height)
		.await
		.context("observe_challenge_period")?;

	let evm_host = dest
		.ismp_host_contract()
		.ok_or_else(|| anyhow!("destination {} has no EvmHost address", pending.destination))?;

	let proof_bytes = dest
		.query_state_proof(
			dest_height,
			StateProofQueryType::Arbitrary(vec![epochs_slot_key(evm_host, pending.set_id)]),
		)
		.await
		.context("query_state_proof")?;

	let msg = outbound_consensus_delivery_message(pending.set_id, pending.destination, payee);
	let claim = OutboundConsensusDeliveryClaim {
		state_proof: Proof {
			height: StateMachineHeight { id: dest.state_machine_id(), height: dest_height },
			proof: proof_bytes,
		},
		set_id: pending.set_id,
		payee,
		signature: dest.sign(&msg),
	};

	tracing::info!(
		target: LOG_TARGET,
		destination = %pending.destination,
		set_id = pending.set_id,
		dest_height,
		"submitting outbound consensus delivery claim",
	);

	hyperbridge
		.submit_outbound_consensus_delivery_claim(claim)
		.await
		.context("submit_outbound_consensus_delivery_claim")
}

/// Builds the 52-byte EIP-1186 storage key for `EvmHost._epochs[set_id]`.
fn epochs_slot_key(evm_host: H160, set_id: u64) -> Vec<u8> {
	let slot_hash =
		derive_map_key(U256::from(set_id).to_big_endian().to_vec(), EVM_HOST_EPOCHS_SLOT);
	let mut key = Vec::with_capacity(52);
	key.extend_from_slice(&evm_host.0);
	key.extend_from_slice(slot_hash.as_bytes());
	key
}
