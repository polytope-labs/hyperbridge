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

//! Outbound request delivery reward claim task.
//!
//! Sibling to [`outbound_claim`](crate::outbound_claim). Where that task
//! claims rewards for delivering BEEFY rotations, this one claims rewards
//! for delivering hyperbridge-originated requests (system messages from
//! host-executive, intents-coprocessor, token-governor, etc.). Supports
//! both EVM and substrate destinations.
//!
//! For every batch of [`PendingRequestDeliveryClaim`]s pushed by the
//! outbound delivery task:
//!
//! 1. Merge with whatever is still pending in the local DB, deduplicated on `commitment`.
//! 2. Filter against `pallet_ismp_relayer::OutboundRequestsClaimed` on Hyperbridge: anything
//!    already present has been claimed by some other relayer; queue for delete and skip.
//! 3. For each surviving row:
//!    - Wait for Hyperbridge's consensus client for the destination to verify a height >=
//!      `delivery_height`.
//!    - Build a state proof of `RequestReceipts[commitment]` at that height (32-byte EVM slot
//!      routed to the ISMP host contract, or substrate child-trie key).
//!    - Sign `outbound_request_delivery_message(commitment, destination, payee)` with the
//!      destination's signing key.
//!    - Submit `pallet_ismp_relayer::claim_outbound_request_delivery_reward` (unsigned) on
//!      Hyperbridge.
//!    - Delete the persisted row.

use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
};

use anyhow::{anyhow, Context as _};
use codec::Decode;
use ismp::{
	consensus::StateMachineHeight, host::StateMachine, messaging::Proof, router::PostRequest,
};
use pallet_ismp::child_trie::request_receipt_storage_key;
use pallet_ismp_relayer::{
	outbound_request_delivery_message, OutboundRequestDeliveryClaim, REQUEST_RECEIPTS_SLOT,
};
use primitive_types::H256;
use sp_core::Pair;
use subxt_utils::outbound_requests_claimed_storage_key;
use tesseract_evm::derive_map_key;
use tesseract_primitives::{
	wait_for_state_machine_update, IsmpProvider, PendingRequestDeliveryClaim, StateProofQueryType,
};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use tokio::sync::mpsc::Receiver;
use tracing::Instrument;
use transaction_fees::TransactionPayment;

const LOG_TARGET: &str = "messaging-outbound-request-claim";

/// Drive a single relayer's outbound request delivery claims. Same shape
/// as [`outbound_claim::run`](crate::outbound_claim::run); see that module
/// for the high-level pipeline.
///
/// The payee is the relayer's Hyperbridge sr25519 account, so all relayer
/// earnings on HB (messaging fees, consensus-delivery rewards, request-
/// delivery rewards) land in the same place.
pub async fn run(
	hyperbridge: SubstrateClient<KeccakSubstrateChain>,
	destinations: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	mut receiver: Receiver<Vec<PendingRequestDeliveryClaim>>,
	tx_payment: Option<Arc<TransactionPayment>>,
) -> Result<(), anyhow::Error> {
	let hb_provider: Arc<dyn IsmpProvider> = Arc::new(hyperbridge.clone());
	let payee_bytes: [u8; 32] = hyperbridge.signer.public().0;

	while let Some(trigger) = receiver.recv().await {
		let merged = merge_pending(&tx_payment, trigger).await;
		if merged.is_empty() {
			continue;
		}

		let (claimed, unclaimed) = match partition_claimed(&hyperbridge, &merged).await {
			Ok(parts) => parts,
			Err(err) => {
				tracing::warn!(
					target: LOG_TARGET,
					?err,
					"OutboundRequestsClaimed lookup failed; processing all rows without \
					 filtering. Duplicates will revert on Hyperbridge",
				);
				(Vec::new(), merged)
			},
		};

		if !claimed.is_empty() {
			tracing::info!(
				target: LOG_TARGET,
				count = claimed.len(),
				"dropping claims already redeemed on Hyperbridge",
			);
			if let Some(tp) = &tx_payment {
				for pending in &claimed {
					let commitment = pending.commitment();
					let key = hex::encode(commitment.0);
					if let Err(err) = tp.delete_request_claim(&key).await {
						tracing::warn!(
							target: LOG_TARGET,
							?err,
							commitment = %commitment,
							"failed to delete already-claimed row; will retry next trigger",
						);
					}
				}
			}
		}

		for pending in unclaimed {
			let destination = pending.destination();
			let commitment = pending.commitment();
			let span = tracing::info_span!(
				"outbound_request_claim",
				destination = %destination,
				delivery_height = pending.delivery_height,
				commitment = %commitment,
			);
			let dest = destinations.get(&destination).cloned();
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
							let _ =
								tx_payment.delete_request_claim(&hex::encode(commitment.0)).await;
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

	Err(anyhow!("outbound-request-claim channel closed"))
}

/// Read `list_pending_request_claims` and union-merge it with the trigger
/// vec, deduplicated on the request commitment. DB rows take precedence on
/// conflict. BTreeMap gives deterministic iteration over the result.
async fn merge_pending(
	tx_payment: &Option<Arc<TransactionPayment>>,
	trigger: Vec<PendingRequestDeliveryClaim>,
) -> Vec<PendingRequestDeliveryClaim> {
	let mut merged: BTreeMap<H256, PendingRequestDeliveryClaim> = BTreeMap::new();

	if let Some(tp) = tx_payment {
		match tp.list_pending_request_claims().await {
			Ok(rows) =>
				for row in rows {
					let request = match PostRequest::decode(&mut &*row.encoded_request) {
						Ok(r) => r,
						Err(err) => {
							tracing::warn!(
								target: LOG_TARGET,
								commitment = %row.commitment,
								?err,
								"undecodable encoded_request in DB; skipping row",
							);
							continue;
						},
					};
					let claim = PendingRequestDeliveryClaim {
						request,
						delivery_height: row.delivery_height as u64,
					};
					merged.insert(claim.commitment(), claim);
				},
			Err(err) => {
				tracing::warn!(
					target: LOG_TARGET,
					?err,
					"list_pending_request_claims failed; trigger-only this cycle",
				);
			},
		}
	}

	for pending in trigger {
		merged.entry(pending.commitment()).or_insert(pending);
	}

	merged.into_values().collect()
}

/// Split `pending` into `(already_claimed, still_unclaimed)` by reading
/// `pallet_ismp_relayer::OutboundRequestsClaimed[commitment]` on Hyperbridge.
async fn partition_claimed(
	hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
	pending: &[PendingRequestDeliveryClaim],
) -> anyhow::Result<(Vec<PendingRequestDeliveryClaim>, Vec<PendingRequestDeliveryClaim>)> {
	let block_hash = hyperbridge
		.rpc
		.chain_get_block_hash(None)
		.await?
		.ok_or_else(|| anyhow!("Failed to query latest hyperbridge block hash"))?;

	let mut claimed = Vec::new();
	let mut unclaimed = Vec::new();

	for pending in pending {
		let commitment = pending.commitment();
		let key = outbound_requests_claimed_storage_key(commitment);
		let raw = hyperbridge
			.client
			.storage()
			.at(block_hash)
			.fetch_raw(key)
			.await
			.with_context(|| format!("OutboundRequestsClaimed lookup ({})", commitment))?;

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
	pending: &PendingRequestDeliveryClaim,
	payee: [u8; 32],
) -> anyhow::Result<()> {
	let destination = pending.destination();
	let commitment = pending.commitment();

	let dest_height = wait_for_state_machine_update(
		dest.state_machine_id(),
		hb_provider.clone(),
		dest.clone(),
		pending.delivery_height,
	)
	.await
	.context("wait_for_state_machine_update")?;

	let key = receipt_key_for(destination, commitment)?;

	let proof_bytes = dest
		.query_state_proof(dest_height, StateProofQueryType::Arbitrary(vec![key]))
		.await
		.context("query_state_proof on destination")?;

	let msg = outbound_request_delivery_message(commitment, destination, payee);
	let signature = dest.sign(&msg);

	let claim = OutboundRequestDeliveryClaim {
		request: pending.request.clone(),
		state_proof: Proof {
			height: StateMachineHeight { id: dest.state_machine_id(), height: dest_height },
			proof: proof_bytes,
		},
		payee,
		signature,
	};

	tracing::info!(
		target: LOG_TARGET,
		destination = %destination,
		commitment = %commitment,
		dest_height,
		"submitting outbound request delivery claim to hyperbridge",
	);
	hyperbridge
		.submit_outbound_request_delivery_claim(claim)
		.await
		.context("submit_outbound_request_delivery_claim")?;
	Ok(())
}

/// Destination-side storage key for `RequestReceipts[commitment]`. Matches
/// the pallet-side derivation in `Pallet::request_receipt_key`.
fn receipt_key_for(destination: StateMachine, commitment: H256) -> anyhow::Result<Vec<u8>> {
	if destination.is_evm() {
		Ok(derive_map_key(commitment.0.to_vec(), REQUEST_RECEIPTS_SLOT).0.to_vec())
	} else if destination.is_substrate() {
		Ok(request_receipt_storage_key(commitment))
	} else {
		Err(anyhow!("unsupported destination state machine: {destination}"))
	}
}
