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

//! Outbound consensus delivery reward claim task.
//!
//! Modeled on the fee accumulation task. For every batch of
//! [`PendingConsensusDeliveryClaim`]s pushed by the outbound delivery task:
//!
//! 1. Pull every persisted row from the local DB (`list_pending_rotation_claims`) and merge it
//!    with the incoming trigger vec, deduplicated on `(destination, set_id)`. The DB is the
//!    source of truth for surviving a crash; the trigger vec carries the freshly-delivered
//!    rotations that may not yet have been queried back from the DB.
//! 2. Filter against `pallet_ismp_relayer::OutboundConsensusRotationsClaimed` on Hyperbridge: any
//!    `(destination, set_id)` already present there has been claimed by some other relayer. Those
//!    rows are queued for deletion from the local DB and skipped — there is no reward left to
//!    collect, and resubmitting would just waste a Hyperbridge extrinsic on a guaranteed revert.
//! 3. For each surviving (still-unclaimed) row:
//!    - Wait for Hyperbridge's consensus client for the destination to verify a height >=
//!      `delivery_height`.
//!    - Build an EIP-1186 storage proof of `HandlerV2._epochs[set_id]` at that height.
//!    - Sign `outbound_consensus_delivery_message(set_id, destination, payee)` with the
//!      destination's EVM key.
//!    - Submit `pallet_ismp_relayer::claim_outbound_consensus_delivery_reward` (unsigned) on
//!      Hyperbridge.
//!    - Delete the persisted claim row.

use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Context as _};
use codec::Decode;
use ismp::{consensus::StateMachineHeight, host::StateMachine, messaging::Proof};
use pallet_ismp_relayer::{
	outbound_consensus_delivery_message, OutboundConsensusDeliveryClaim, HANDLER_V2_EPOCHS_SLOT,
};
use primitive_types::{H160, U256};
use sp_core::{keccak_256, Pair};
use subxt_utils::outbound_consensus_rotations_claimed_storage_key;
use tesseract_primitives::{
	wait_for_state_machine_update, IsmpProvider, PendingConsensusDeliveryClaim, StateProofQueryType,
};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use tokio::sync::mpsc::Receiver;
use tracing::Instrument;
use transaction_fees::TransactionPayment;

const LOG_TARGET: &str = "tesseract-messaging-outbound-claim";

/// Drive a single relayer's outbound consensus delivery claims. Mirrors
/// [`fee_accumulation`](crate::fee_accumulation) shape: receive a trigger
/// (now a vector of newly-delivered rotations), pull every pending row
/// from the DB, merge both, drop anything Hyperbridge already records as
/// claimed, then process whatever's left.
///
/// The payee is always the relayer's own Hyperbridge sr25519 account
/// (`hyperbridge.signer.public()`) — the same account that already
/// receives messaging fees, so all relayer earnings on HB land in one
/// place.
pub async fn run(
	hyperbridge: SubstrateClient<KeccakSubstrateChain>,
	destinations: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	mut receiver: Receiver<Vec<PendingConsensusDeliveryClaim>>,
	tx_payment: Option<Arc<TransactionPayment>>,
) -> Result<(), anyhow::Error> {
	let hb_provider: Arc<dyn IsmpProvider> = Arc::new(hyperbridge.clone());
	let payee_bytes: [u8; 32] = hyperbridge.signer.public().0;

	while let Some(trigger) = receiver.recv().await {
		// Merge the live trigger with everything still pending in the DB,
		// deduped on `(destination, set_id)`. The DB is the source of
		// truth for surviving a crash; the trigger vec covers the
		// just-delivered rotations that may not have round-tripped
		// through the DB yet (e.g. if the persist write failed).
		let merged = merge_pending(&tx_payment, trigger).await;
		if merged.is_empty() {
			continue;
		}

		// Split into "already claimed by some other relayer" (skip + queue
		// for delete) and "still claimable" (process). The former group
		// would just revert with `OutboundRotationAlreadyClaimed` if we
		// submitted, so short-circuiting saves a Hyperbridge round-trip
		// per stale row.
		let (claimed, unclaimed) = match partition_claimed(&hyperbridge, &merged).await {
			Ok(parts) => parts,
			Err(err) => {
				tracing::warn!(
					target: LOG_TARGET,
					?err,
					"OutboundConsensusRotationsClaimed lookup failed; processing all rows \
					 without filtering — duplicates will revert on Hyperbridge",
				);
				(Vec::new(), merged)
			},
		};

		// Queue already-claimed rows for deletion. Best-effort: a failure
		// here just means the row will be revisited on the next trigger,
		// where this same partition step will catch it again.
		if !claimed.is_empty() {
			tracing::info!(
				target: LOG_TARGET,
				count = claimed.len(),
				"dropping claims already redeemed on Hyperbridge",
			);
			if let Some(tp) = &tx_payment {
				for pending in &claimed {
					if let Err(err) = tp
						.delete_rotation_claim(&pending.destination.to_string(), pending.set_id)
						.await
					{
						tracing::warn!(
							target: LOG_TARGET,
							?err,
							destination = %pending.destination,
							set_id = pending.set_id,
							"failed to delete already-claimed row; will retry next trigger",
						);
					}
				}
			}
		}

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
			let tx_payment = tx_payment.clone();
			async move {
				let Some(dest) = dest else {
					tracing::warn!(target: LOG_TARGET, "no provider for destination; dropping claim");
					return;
				};
				match process_claim(&hb, hb_view, dest, &pending, payee_bytes).await {
					Ok(()) => {
						tracing::info!(target: LOG_TARGET, "claim submitted");
						if let Some(tx_payment) = &tx_payment {
							let _ = tx_payment
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
							"claim submission failed; row left in DB for next trigger",
						);
					},
				}
			}
			.instrument(span)
			.await;
		}
	}

	Err(anyhow!("outbound-claim channel closed"))
}

/// Read `list_pending_rotation_claims` and union-merge it with the
/// trigger vec. The merged vec is deduplicated on `(destination, set_id)`
/// so each pair is processed at most once per wake-up. DB rows take
/// precedence on conflict — they carry the persisted `delivery_height`
/// in the same `rotation_height` column the writer used.
async fn merge_pending(
	tx_payment: &Option<Arc<TransactionPayment>>,
	trigger: Vec<PendingConsensusDeliveryClaim>,
) -> Vec<PendingConsensusDeliveryClaim> {
	let mut merged: HashMap<(StateMachine, u64), PendingConsensusDeliveryClaim> = HashMap::new();

	if let Some(tp) = tx_payment {
		match tp.list_pending_rotation_claims().await {
			Ok(rows) =>
				for row in rows {
					use std::str::FromStr;
					let destination = match StateMachine::from_str(&row.dest) {
						Ok(sm) => sm,
						Err(err) => {
							tracing::warn!(
								target: LOG_TARGET,
								dest = %row.dest,
								?err,
								"unparseable state machine in DB; skipping row",
							);
							continue;
						},
					};
					let set_id = row.set_id as u64;
					merged.insert(
						(destination, set_id),
						PendingConsensusDeliveryClaim {
							destination,
							delivery_height: row.rotation_height as u64,
							set_id,
						},
					);
				},
			Err(err) => {
				tracing::warn!(
					target: LOG_TARGET,
					?err,
					"list_pending_rotation_claims failed; trigger-only this cycle",
				);
			},
		}
	}

	for pending in trigger {
		merged.entry((pending.destination, pending.set_id)).or_insert(pending);
	}

	merged.into_values().collect()
}

/// Split `pending` into `(already_claimed, still_unclaimed)` by reading
/// `pallet_ismp_relayer::OutboundConsensusRotationsClaimed[(destination,
/// set_id)]` on Hyperbridge. The storage value is `()` so a non-empty
/// raw fetch (i.e. `Some(_)`) means the `(destination, set_id)` is
/// closed.
///
/// Lookups are sequential — the surface is small (typically a handful
/// of rows per trigger) and parallelism would just dogpile the HB RPC
/// for no real win.
async fn partition_claimed(
	hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
	pending: &[PendingConsensusDeliveryClaim],
) -> anyhow::Result<(Vec<PendingConsensusDeliveryClaim>, Vec<PendingConsensusDeliveryClaim>)> {
	let block_hash = hyperbridge
		.rpc
		.chain_get_block_hash(None)
		.await?
		.ok_or_else(|| anyhow!("Failed to query latest hyperbridge block hash"))?;

	let mut claimed = Vec::new();
	let mut unclaimed = Vec::new();

	for pending in pending {
		let key =
			outbound_consensus_rotations_claimed_storage_key(pending.destination, pending.set_id);
		let raw = hyperbridge
			.client
			.storage()
			.at(block_hash)
			.fetch_raw(key)
			.await
			.with_context(|| {
				format!(
					"OutboundConsensusRotationsClaimed lookup ({:?}, {})",
					pending.destination, pending.set_id,
				)
			})?;

		// Stored value is `()`, so any presence — even an empty `Vec` —
		// means the entry exists. `OptionQuery` ensures absence is
		// `None`. `Decode` against `()` is the strict version of this
		// check; we keep it cheap with the `is_some` shortcut and only
		// fall through to decode if some chain encoded the unit value
		// explicitly.
		let is_claimed = match raw {
			Some(bytes) => <()>::decode(&mut &*bytes).is_ok() || bytes.is_empty(),
			None => false,
		};

		if is_claimed {
			claimed.push(pending.clone());
		} else {
			unclaimed.push(pending.clone());
		}
	}

	Ok((claimed, unclaimed))
}

async fn process_claim(
	hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
	hb_provider: Arc<dyn IsmpProvider>,
	dest: Arc<dyn IsmpProvider>,
	pending: &PendingConsensusDeliveryClaim,
	payee: [u8; 32],
) -> anyhow::Result<()> {
	// Same shape fee_accumulation uses: wait for HB's view of the
	// destination to cross the delivery block.
	let dest_height = wait_for_state_machine_update(
		dest.state_machine_id(),
		hb_provider.clone(),
		dest.clone(),
		pending.delivery_height,
	)
	.await
	.context("wait_for_state_machine_update")?;

	// Build the 52-byte EIP-1186 key the EVM verifier expects:
	// `handler_v2 (20) || keccak256(set_id || HANDLER_V2_EPOCHS_SLOT) (32)`.
	// The EVM provider reads HandlerV2 from `EvmHost.hostParams().handler`
	// so it stays in sync with whatever governance has set on chain.
	// Substrate destinations report `None` and are skipped — the claim
	// flow is EVM-only.
	let handler = dest.handler_v2_address().await.ok_or_else(|| {
		anyhow!(
			"destination has no HandlerV2 address (non-EVM, or hostParams() RPC failed); \
			 cannot derive _epochs[set_id] key",
		)
	})?;
	let key = epochs_slot_key(handler, pending.set_id);

	let proof_bytes = dest
		.query_state_proof(dest_height, StateProofQueryType::Arbitrary(vec![key]))
		.await
		.context("query_state_proof on destination")?;

	let msg = outbound_consensus_delivery_message(pending.set_id, pending.destination, payee);
	let signature = dest.sign(&msg);

	let claim = OutboundConsensusDeliveryClaim {
		state_proof: Proof {
			height: StateMachineHeight { id: dest.state_machine_id(), height: dest_height },
			proof: proof_bytes,
		},
		set_id: pending.set_id,
		payee,
		signature,
	};

	hyperbridge
		.submit_outbound_consensus_delivery_claim(claim)
		.await
		.context("submit_outbound_consensus_delivery_claim")?;
	Ok(())
}

/// `keccak256(set_id || HANDLER_V2_EPOCHS_SLOT)` prefixed with the
/// HandlerV2 contract address. Matches the pallet-side derivation in
/// `process_outbound_consensus_delivery_claim`.
fn epochs_slot_key(handler: H160, set_id: u64) -> Vec<u8> {
	let mut input = [0u8; 64];
	input[..32].copy_from_slice(&U256::from(set_id).to_big_endian());
	input[32..].copy_from_slice(&U256::from(HANDLER_V2_EPOCHS_SLOT).to_big_endian());
	let slot_hash = keccak_256(&input);

	let mut key = Vec::with_capacity(52);
	key.extend_from_slice(&handler.0);
	key.extend_from_slice(&slot_hash);
	key
}
