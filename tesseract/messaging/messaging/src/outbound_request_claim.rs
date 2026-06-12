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

//! Periodic task that claims outbound request delivery rewards.
//!
//! Each time the relayer delivers a hyperbridge-originated request, the delivery path writes a
//! row to the local DB. This task wakes on a fixed interval, reads those rows, skips anything
//! already claimed on Hyperbridge, and submits the remaining claims in parallel.

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{anyhow, Context as _};
use codec::Decode;
use futures::{stream::FuturesUnordered, StreamExt};
use ismp::{
	consensus::StateMachineHeight, host::StateMachine, messaging::Proof, router::PostRequest,
};
use pallet_ismp::child_trie::request_receipt_storage_key;
use pallet_ismp_relayer::{
	outbound_request_delivery_message, OutboundRequestDeliveryClaim, REQUEST_RECEIPTS_SLOT,
};
use primitive_types::{H160, H256};
use sp_core::Pair;
use subxt_utils::outbound_requests_claimed_storage_key;
use tesseract_evm::derive_map_key;
use tesseract_primitives::{
	IsmpHost, IsmpProvider, PendingRequestDeliveryClaim, StateProofQueryType,
};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use tokio::time::MissedTickBehavior;
use tracing::Instrument;
use transaction_fees::TransactionPayment;

const LOG_TARGET: &str = "messaging-outbound-request-claim";

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
					let key = hex::encode(row.commitment().0);
					if let Err(err) = tp.delete_request_claim(&key).await {
						tracing::warn!(
							target: LOG_TARGET,
							?err,
							commitment = %row.commitment(),
							"failed to delete already-claimed row",
						);
					}
				}
			}
		}

		let mut tasks = FuturesUnordered::new();
		for pending in unclaimed {
			let span = tracing::info_span!(
				"outbound_request_claim",
				destination = %pending.destination(),
				delivery_height = pending.delivery_height,
				commitment = %pending.commitment(),
			);
			let dest = destinations.get(&pending.destination()).cloned();
			let consensus_host = consensus_hosts.get(&pending.destination()).cloned();
			let hb = hyperbridge.clone();
			let hb_view = hb_provider.clone();
			let tp = tx_payment.clone();
			tasks.push(
				async move {
					let Some(dest) = dest else {
						tracing::warn!(
							target: LOG_TARGET,
							"no provider for destination; dropping claim"
						);
						return;
					};
					let commitment = pending.commitment();
					match process_claim(&hb, hb_view, dest, &pending, payee_bytes, consensus_host)
						.await
					{
						Ok(()) => {
							tracing::info!(target: LOG_TARGET, "claim submitted");
							if let Some(tp) = &tp {
								let _ = tp.delete_request_claim(&hex::encode(commitment.0)).await;
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
) -> Vec<PendingRequestDeliveryClaim> {
	let Some(tp) = tx_payment else { return Vec::new() };
	match tp.list_pending_request_claims().await {
		Ok(rows) => {
			let mut claims: Vec<PendingRequestDeliveryClaim> = rows
				.into_iter()
				.filter_map(|row| {
					let request = match PostRequest::decode(&mut &*row.encoded_request) {
						Ok(r) => r,
						Err(err) => {
							tracing::warn!(
								target: LOG_TARGET,
								commitment = %row.commitment,
								?err,
								"undecodable encoded_request in DB; skipping row",
							);
							return None;
						},
					};
					Some(PendingRequestDeliveryClaim {
						request,
						delivery_height: row.delivery_height as u64,
					})
				})
				.collect();
			claims.sort_by_key(|c| c.delivery_height);
			claims
		},
		Err(err) => {
			tracing::warn!(
				target: LOG_TARGET,
				?err,
				"could not read pending claims; skipping tick"
			);
			Vec::new()
		},
	}
}

pub async fn partition_claimed(
	hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
	pending: &[PendingRequestDeliveryClaim],
) -> anyhow::Result<(Vec<PendingRequestDeliveryClaim>, Vec<PendingRequestDeliveryClaim>)> {
	let block_hash = hyperbridge
		.rpc
		.chain_get_block_hash(None)
		.await?
		.ok_or_else(|| anyhow!("failed to fetch latest Hyperbridge block hash"))?;

	let mut claimed = Vec::new();
	let mut unclaimed = Vec::new();

	for item in pending {
		let commitment = item.commitment();
		let key = outbound_requests_claimed_storage_key(commitment);
		let raw = hyperbridge
			.client
			.storage()
			.at(block_hash)
			.fetch_raw(key)
			.await
			.with_context(|| format!("OutboundRequestsClaimed lookup ({})", commitment))?;

		if raw.is_some() {
			claimed.push(item.clone());
		} else {
			unclaimed.push(item.clone());
		}
	}

	Ok((claimed, unclaimed))
}

pub async fn process_claim(
	hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
	hb_provider: Arc<dyn IsmpProvider>,
	dest: Arc<dyn IsmpProvider>,
	pending: &PendingRequestDeliveryClaim,
	payee: [u8; 32],
	consensus_host: Option<Arc<dyn IsmpHost>>,
) -> anyhow::Result<()> {
	let destination = pending.destination();
	let commitment = pending.commitment();

	if let Some(host) = consensus_host {
		host.advance_counterparty_to(hb_provider.clone(), pending.delivery_height)
			.await
			.context("advance_counterparty_to")?;
	}

	let committed = hb_provider
		.query_latest_height(dest.state_machine_id())
		.await
		.context("query_latest_height")?;

	if (committed as u64) < pending.delivery_height {
		return Err(anyhow!(
			"Hyperbridge has only committed {} for {}, delivery block {} not yet reachable",
			committed,
			destination,
			pending.delivery_height,
		));
	}

	let dest_height = committed as u64;

	let key = receipt_key_for(destination, commitment, dest.ismp_host_contract())?;

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
		"submitting outbound request delivery claim",
	);

	hyperbridge
		.submit_outbound_request_delivery_claim(claim)
		.await
		.context("submit_outbound_request_delivery_claim")?;

	Ok(())
}

fn receipt_key_for(
	destination: StateMachine,
	commitment: H256,
	ismp_host: Option<H160>,
) -> anyhow::Result<Vec<u8>> {
	if destination.is_evm() {
		let host = ismp_host.ok_or_else(|| {
			anyhow!("no IsmpHost contract address for EVM destination {destination}")
		})?;
		let slot = derive_map_key(commitment.0.to_vec(), REQUEST_RECEIPTS_SLOT);
		let mut key = host.0.to_vec();
		key.extend_from_slice(&slot.0);
		Ok(key)
	} else if destination.is_substrate() {
		Ok(request_receipt_storage_key(commitment))
	} else {
		Err(anyhow!("unsupported destination state machine: {destination}"))
	}
}
