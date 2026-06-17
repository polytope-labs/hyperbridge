// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0.

//! `claim-rewards` subcommand.
//!
//! One-shot pass over every pending outbound consensus delivery claim in the
//! local DB. For Polkadot Hub deliveries — where Hyperbridge's state machine
//! height for the destination may not advance on its own (no parachain
//! inherents) — a parachain consensus proof is submitted to Hyperbridge first
//! before the claim proof is built.

use std::{collections::BTreeMap, sync::Arc};

use anyhow::Context;
use codec::Decode;
use futures::{stream::FuturesUnordered, StreamExt};
use ismp::host::StateMachine;
use ismp_parachain::consensus::{ASSET_HUB_MAINNET_CHAIN_ID, PASSET_HUB_TESTNET_CHAIN_ID};
use messaging::{
	outbound_claim::{partition_claimed, process_claim},
	outbound_request_claim,
};
use sp_core::Pair;
use tesseract_consensus_config::create_client_map;
use tesseract_primitives::{
	IsmpProvider, PendingConsensusDeliveryClaim, PendingRequestDeliveryClaim,
};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use tracing::Instrument;
use transaction_fees::TransactionPayment;

use crate::config::{setup_logging, HyperbridgeConfig};

const LOG_TARGET: &str = "tesseract-claim-rewards";

#[derive(Debug, clap::Args)]
#[command(about = "Manually claim outbound consensus delivery rewards for all pending rotations.")]
pub struct ClaimRewards {}

impl ClaimRewards {
	pub async fn run(&self, config_path: &str, db: &str) -> anyhow::Result<()> {
		let _ = setup_logging();
		let config = HyperbridgeConfig::parse_conf(config_path).await?;

		let hyperbridge =
			SubstrateClient::<KeccakSubstrateChain>::new(config.hyperbridge.substrate.clone())
				.await?;
		let hyperbridge_provider: Arc<dyn IsmpProvider> = Arc::new(hyperbridge.clone());

		let mut clients = std::collections::HashMap::new();
		for (sm, pc) in &config.chains {
			let provider = pc
				.messaging
				.clone()
				.into_client(hyperbridge_provider.clone())
				.await
				.with_context(|| format!("failed to build messaging client for {sm}"))?;
			clients.insert(*sm, provider);
		}

		let consensus_hosts = create_client_map(config.consensus_chains()).await?;

		let tx_payment = Arc::new(
			TransactionPayment::initialize(db)
				.await
				.context("error initializing fee database")?,
		);

		let payee_bytes: [u8; 32] = hyperbridge.signer.public().0;

		let all_rows = tx_payment
			.list_pending_rotation_claims()
			.await
			.context("list_pending_rotation_claims")?;

		if all_rows.is_empty() {
			tracing::info!(target: LOG_TARGET, "no pending rotation claims in DB");
		}

		let all_claims: Vec<PendingConsensusDeliveryClaim> = all_rows
			.into_iter()
			.filter_map(|row| {
				use std::str::FromStr;
				let destination = StateMachine::from_str(&row.dest)
					.map_err(|e| {
						tracing::warn!(
							target: LOG_TARGET,
							dest = %row.dest,
							?e,
							"unparseable state machine in DB; skipping row",
						)
					})
					.ok()?;
				Some(PendingConsensusDeliveryClaim {
					destination,
					delivery_height: row.rotation_height as u64,
					set_id: row.set_id as u64,
				})
			})
			.collect();

		let (already_claimed, unclaimed) = match partition_claimed(&hyperbridge, &all_claims).await
		{
			Ok(parts) => parts,
			Err(err) => {
				tracing::warn!(
					target: LOG_TARGET,
					?err,
					"OutboundConsensusRotationsClaimed lookup failed; processing all rows",
				);
				(Vec::new(), all_claims)
			},
		};

		if !already_claimed.is_empty() {
			tracing::info!(
				target: LOG_TARGET,
				count = already_claimed.len(),
				"skipping claims already redeemed on Hyperbridge",
			);
			if let Err(err) = cleanup_claimed(&tx_payment, &already_claimed).await {
				tracing::warn!(target: LOG_TARGET, ?err, "failed to clean up already-claimed rows");
			}
		}

		// For Polkadot Hub, Hyperbridge's state machine height may not advance
		// automatically without parachain inherents. Submit a consensus proof to
		// Hyperbridge first so the claim proof can be anchored against a known
		// state commitment.
		let polkadot_hub_claims = group_by_polkadot_hub(&unclaimed);
		for (sm, claims) in &polkadot_hub_claims {
			let Some(host) = consensus_hosts.get(sm) else {
				tracing::warn!(
					target: LOG_TARGET,
					%sm,
					"no consensus host configured for Polkadot Hub; claim may fail if \
					 Hyperbridge has not seen the delivery height",
				);
				continue;
			};

			let Some(messaging) = clients.get(sm) else {
				tracing::warn!(
					target: LOG_TARGET,
					%sm,
					"no messaging client configured for Polkadot Hub; cannot query \
					 Hyperbridge's state machine height",
				);
				continue;
			};

			let max_delivery_height =
				claims.iter().map(|c| c.delivery_height).max().unwrap_or_default();

			let hb_height = hyperbridge_provider
				.query_latest_height(messaging.state_machine_id())
				.await
				.unwrap_or(0) as u64;

			if hb_height < max_delivery_height {
				tracing::info!(
					target: LOG_TARGET,
					%sm,
					hb_height,
					max_delivery_height,
					"advancing Hyperbridge's state machine height for Polkadot Hub",
				);
				if let Err(err) = host
					.advance_counterparty_to(hyperbridge_provider.clone(), max_delivery_height)
					.await
				{
					tracing::warn!(
						target: LOG_TARGET,
						%sm,
						?err,
						"failed to advance state machine height; claims may fail",
					);
				}
			}
		}

		let mut tasks = FuturesUnordered::new();
		for pending in unclaimed {
			let span = tracing::info_span!(
				"claim_rewards",
				destination = %pending.destination,
				delivery_height = pending.delivery_height,
				set_id = pending.set_id,
			);
			let dest = clients.get(&pending.destination).cloned();
			let hb = hyperbridge.clone();
			let hb_view = hyperbridge_provider.clone();
			let tp = tx_payment.clone();
			tasks.push(
				async move {
					let Some(dest) = dest else {
						tracing::warn!(
							target: LOG_TARGET,
							destination = %pending.destination,
							"no messaging client for destination; skipping claim",
						);
						return;
					};
					match process_claim(&hb, hb_view, dest, &pending, payee_bytes).await {
						Ok(()) => {
							tracing::info!(
								target: LOG_TARGET,
								destination = %pending.destination,
								set_id = pending.set_id,
								"claim submitted",
							);
							let _ = tp
								.delete_rotation_claim(
									&pending.destination.to_string(),
									pending.set_id,
								)
								.await;
						},
						Err(err) => {
							tracing::error!(
								target: LOG_TARGET,
								destination = %pending.destination,
								set_id = pending.set_id,
								?err,
								"claim failed",
							);
						},
					}
				}
				.instrument(span),
			);
		}

		while tasks.next().await.is_some() {}

		// --- request delivery claims ---
		let request_rows = tx_payment
			.list_pending_request_claims()
			.await
			.context("list_pending_request_claims")?;

		if request_rows.is_empty() {
			tracing::info!(target: LOG_TARGET, "no pending request claims in DB");
		}

		let all_request_claims: Vec<PendingRequestDeliveryClaim> = request_rows
			.into_iter()
			.filter_map(|row| {
				use ismp::router::PostRequest;
				let request = PostRequest::decode(&mut &*row.encoded_request)
					.map_err(|e| {
						tracing::warn!(
							target: LOG_TARGET,
							commitment = %row.commitment,
							?e,
							"undecodable request in DB; skipping row",
						)
					})
					.ok()?;
				Some(PendingRequestDeliveryClaim {
					request,
					delivery_height: row.delivery_height as u64,
				})
			})
			.collect();

		let hub_request_claims = group_request_claims_by_polkadot_hub(&all_request_claims);
		for (sm, claims) in &hub_request_claims {
			let Some(host) = consensus_hosts.get(sm) else {
				tracing::warn!(
					target: LOG_TARGET,
					%sm,
					"no consensus host configured for Polkadot Hub; request claim may fail",
				);
				continue;
			};

			let Some(messaging) = clients.get(sm) else {
				tracing::warn!(
					target: LOG_TARGET,
					%sm,
					"no messaging client configured for Polkadot Hub; cannot query \
					 Hyperbridge's state machine height for request claims",
				);
				continue;
			};

			let max_delivery_height =
				claims.iter().map(|c| c.delivery_height).max().unwrap_or_default();

			let hb_height = hyperbridge_provider
				.query_latest_height(messaging.state_machine_id())
				.await
				.unwrap_or(0) as u64;

			if hb_height < max_delivery_height {
				tracing::info!(
					target: LOG_TARGET,
					%sm,
					hb_height,
					max_delivery_height,
					"advancing Hyperbridge's state machine height for Polkadot Hub request claims",
				);
				if let Err(err) = host
					.advance_counterparty_to(hyperbridge_provider.clone(), max_delivery_height)
					.await
				{
					tracing::warn!(
						target: LOG_TARGET,
						%sm,
						?err,
						"failed to advance state machine height; request claims may fail",
					);
				}
			}
		}

		let (already_claimed_reqs, unclaimed_reqs) =
			match outbound_request_claim::partition_claimed(&hyperbridge, &all_request_claims).await
			{
				Ok(parts) => parts,
				Err(err) => {
					tracing::warn!(
						target: LOG_TARGET,
						?err,
						"OutboundRequestsClaimed lookup failed; processing all rows",
					);
					(Vec::new(), all_request_claims)
				},
			};

		if !already_claimed_reqs.is_empty() {
			tracing::info!(
				target: LOG_TARGET,
				count = already_claimed_reqs.len(),
				"skipping request claims already redeemed on Hyperbridge",
			);
			for row in &already_claimed_reqs {
				let _ = tx_payment.delete_request_claim(&hex::encode(row.commitment().0)).await;
			}
		}

		let mut req_tasks = FuturesUnordered::new();
		for pending in unclaimed_reqs {
			let span = tracing::info_span!(
				"claim_request_rewards",
				destination = %pending.destination(),
				delivery_height = pending.delivery_height,
				commitment = %pending.commitment(),
			);
			let dest = clients.get(&pending.destination()).cloned();
			let hb = hyperbridge.clone();
			let hb_view = hyperbridge_provider.clone();
			let tp = tx_payment.clone();
			req_tasks.push(
				async move {
					let Some(dest) = dest else {
						tracing::warn!(
							target: LOG_TARGET,
							destination = %pending.destination(),
							"no messaging client for destination; skipping request claim",
						);
						return;
					};
					match outbound_request_claim::process_claim(
						&hb,
						hb_view,
						dest,
						&pending,
						payee_bytes,
					)
					.await
					{
						Ok(()) => {
							tracing::info!(
								target: LOG_TARGET,
								commitment = %pending.commitment(),
								"request claim submitted",
							);
							let _ =
								tp.delete_request_claim(&hex::encode(pending.commitment().0)).await;
						},
						Err(err) => {
							tracing::error!(
								target: LOG_TARGET,
								commitment = %pending.commitment(),
								?err,
								"request claim failed",
							);
						},
					}
				}
				.instrument(span),
			);
		}

		while req_tasks.next().await.is_some() {}

		tracing::info!(target: LOG_TARGET, "claim-rewards complete");
		Ok(())
	}
}

/// Group unclaimed rows that target Polkadot Hub, keyed by state machine.
fn group_by_polkadot_hub(
	claims: &[PendingConsensusDeliveryClaim],
) -> BTreeMap<StateMachine, Vec<PendingConsensusDeliveryClaim>> {
	let mut out: BTreeMap<StateMachine, Vec<PendingConsensusDeliveryClaim>> = BTreeMap::new();
	for claim in claims {
		if is_polkadot_hub(claim.destination) {
			out.entry(claim.destination).or_default().push(claim.clone());
		}
	}
	out
}

fn is_polkadot_hub(sm: StateMachine) -> bool {
	matches!(sm, StateMachine::Evm(id) if id == PASSET_HUB_TESTNET_CHAIN_ID || id == ASSET_HUB_MAINNET_CHAIN_ID)
}

fn group_request_claims_by_polkadot_hub(
	claims: &[PendingRequestDeliveryClaim],
) -> BTreeMap<StateMachine, Vec<PendingRequestDeliveryClaim>> {
	let mut out: BTreeMap<StateMachine, Vec<PendingRequestDeliveryClaim>> = BTreeMap::new();
	for claim in claims {
		if is_polkadot_hub(claim.destination()) {
			out.entry(claim.destination()).or_default().push(claim.clone());
		}
	}
	out
}

async fn cleanup_claimed(
	tx_payment: &TransactionPayment,
	claimed: &[PendingConsensusDeliveryClaim],
) -> anyhow::Result<()> {
	for pending in claimed {
		tx_payment
			.delete_rotation_claim(&pending.destination.to_string(), pending.set_id)
			.await?;
	}
	Ok(())
}
