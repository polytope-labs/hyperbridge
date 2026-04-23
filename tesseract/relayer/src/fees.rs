// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0.

//! `accumulate-fees` subcommand.
//!
//! Reads every (source, dest) delivery recorded in the local fee DB, builds a
//! claim proof against the latest Hyperbridge view of each chain, and submits
//! the resulting withdrawal extrinsic. With `--withdraw`, delegates to the
//! shared [`messaging::fees::withdraw_once`] pass instead (same code path the
//! long-running relayer's periodic auto-withdraw uses).
//!
//! Ported from `tesseract-messaging`'s `AccumulateFees` onto the consolidated
//! relayer's config model (`[<chain>]` tables with optional `[.consensus]`).

use std::{collections::HashMap, str::FromStr, sync::Arc};

use anyhow::Context;
use futures::StreamExt;
use ismp::host::StateMachine;
use tesseract_primitives::{
	observe_challenge_period, wait_for_state_machine_update, ConsensusProofSource, IsmpProvider,
};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use transaction_fees::TransactionPayment;

use crate::{config::HyperbridgeConfig, provider::OffchainProofSource};

#[derive(Debug, clap::Args)]
#[command(
	about = "Claim fees for past deliveries (and optionally sweep the resulting hyperbridge balance to each destination)."
)]
pub struct AccumulateFees {
	/// After accumulating, sweep the unclaimed balance on Hyperbridge out to
	/// each destination via the same code path as the periodic auto-withdraw.
	#[arg(short, long)]
	pub withdraw: bool,
	/// Gas limit for executing withdrawal requests on both chains. Passed
	/// through unchanged for compatibility with the legacy flag; currently
	/// unused by this path.
	#[arg(short, long)]
	pub gas_limit: Option<u64>,
	/// When `true`, block waiting for Hyperbridge to see each required source
	/// height before submitting proof. When `false`, skip (source, dest)
	/// pairs whose latest delivery height isn't yet mirrored on HB — retry on
	/// a later run.
	#[arg(long)]
	pub wait: bool,
}

impl AccumulateFees {
	/// Entry point invoked by `main.rs` when the user passes the
	/// `accumulate-fees` subcommand. Parses the consolidated config, builds a
	/// client per chain, and runs either the claim sweep or (if `--withdraw`)
	/// the withdrawal pass.
	pub async fn run(&self, config_path: &str, db: &str) -> anyhow::Result<()> {
		let config = HyperbridgeConfig::parse_conf(config_path).await?;
		let hyperbridge =
			SubstrateClient::<KeccakSubstrateChain>::new(config.hyperbridge.clone()).await?;
		let hyperbridge_provider: Arc<dyn IsmpProvider> = Arc::new(hyperbridge.clone());

		let mut clients: HashMap<StateMachine, Arc<dyn IsmpProvider>> = HashMap::new();
		for (sm, pc) in &config.chains {
			let provider = pc
				.messaging
				.clone()
				.into_client(hyperbridge_provider.clone())
				.await
				.with_context(|| format!("failed to build messaging client for {sm}"))?;
			clients.insert(*sm, provider);
		}

		let tx_payment = Arc::new(
			TransactionPayment::initialize(db)
				.await
				.context("Error initializing fee database")?,
		);

		if self.withdraw {
			let proof_source: Arc<dyn ConsensusProofSource> =
				Arc::new(OffchainProofSource::new(hyperbridge.rpc_client.clone()));
			let relayer_config: tesseract_primitives::config::RelayerConfig =
				config.relayer.clone().into();
			// Withdrawals emit a POST request from Hyperbridge back to each
			// destination, and the destination side of the flow eventually
			// asks the destination provider to sign (see
			// `tesseract-substrate::calls::withdraw_funds`). Signer-less
			// chains cannot sign, so they must be excluded here or the call
			// panics on the first one. The relayer's long-running
			// auto-withdraw applies the same filter.
			let withdraw_clients: HashMap<StateMachine, Arc<dyn IsmpProvider>> = config
				.chains
				.iter()
				.filter(|(_, pc)| pc.outbound_enabled())
				.filter_map(|(sm, _)| clients.get(sm).map(|p| (*sm, p.clone())))
				.collect();
			messaging::fees::withdraw_once(
				&hyperbridge,
				&withdraw_clients,
				&relayer_config,
				&tx_payment,
				&proof_source,
			)
			.await;
			return Ok(());
		}

		self.accumulate(&hyperbridge, clients, tx_payment).await
	}

	/// Per-pair accumulation: fan out over every distinct (source, dest) pair
	/// the DB has recorded deliveries for, and submit claim proofs in both
	/// directions.
	async fn accumulate(
		&self,
		hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
		clients: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
		tx_payment: Arc<TransactionPayment>,
	) -> anyhow::Result<()> {
		tracing::trace!(target: crate::LOG_TARGET, "accumulate-fees starting");
		let wait = self.wait;
		let stream = futures::stream::iter(tx_payment.distinct_deliveries().await?.into_iter());
		stream
			.for_each_concurrent(None, |delivery| {
				let clients = &clients;
				let tx_payment = tx_payment.clone();
				let hyperbridge = hyperbridge.clone();
				async move {
					let source_chain = StateMachine::from_str(&delivery.source_chain)
						.expect("invalid source state machine in DB");
					let dest_chain = StateMachine::from_str(&delivery.dest_chain)
						.expect("invalid dest state machine in DB");
					let (Some(source), Some(dest)) =
						(clients.get(&source_chain).cloned(), clients.get(&dest_chain).cloned())
					else {
						tracing::warn!(
							target: crate::LOG_TARGET,
							source = %source_chain,
							dest = %dest_chain,
							"skipping — no client configured for source or dest",
						);
						return;
					};

					let lambda = || async {
						let source_height = hyperbridge
							.query_latest_height(source.state_machine_id())
							.await? as u64;
						let dest_height =
							hyperbridge.query_latest_height(dest.state_machine_id()).await? as u64;

						let highest_to_dest = tx_payment
							.highest_delivery_height(
								source.state_machine_id().state_id,
								dest.state_machine_id().state_id,
							)
							.await?;
						let highest_to_source = tx_payment
							.highest_delivery_height(
								dest.state_machine_id().state_id,
								source.state_machine_id().state_id,
							)
							.await?;

						if highest_to_dest.is_none() && highest_to_source.is_none() {
							tracing::trace!(
								target: crate::LOG_TARGET,
								source = %source_chain,
								dest = %dest_chain,
								"no deliveries recorded; skipping",
							);
							return Ok::<_, anyhow::Error>(());
						}

						submit_pair(
							&hyperbridge,
							source.clone(),
							dest.clone(),
							source_height,
							dest_height,
							highest_to_dest,
							&tx_payment,
							wait,
						)
						.await?;

						submit_pair(
							&hyperbridge,
							dest.clone(),
							source.clone(),
							dest_height,
							source_height,
							highest_to_source,
							&tx_payment,
							wait,
						)
						.await?;

						Ok(())
					};
					if let Err(err) = lambda().await {
						tracing::error!(
							target: crate::LOG_TARGET,
							source = %source_chain,
							dest = %dest_chain,
							?err,
							"fee accumulation for pair failed",
						);
					}
				}
			})
			.await;

		tracing::info!(target: crate::LOG_TARGET, "accumulate-fees complete");
		Ok(())
	}
}

/// Create + submit the claim proof for deliveries going `source → dest`.
///
/// `source_height` is HB's latest view of `source`; `dest_height` is HB's
/// latest view of `dest`. `highest_delivery_height` is the maximum delivery
/// height to `dest` recorded in the local DB — if it's ahead of HB's view,
/// either wait (when `wait` is true) or skip until a later run.
#[allow(clippy::too_many_arguments)]
async fn submit_pair(
	hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
	source: Arc<dyn IsmpProvider>,
	dest: Arc<dyn IsmpProvider>,
	source_height: u64,
	dest_height: u64,
	highest_delivery_height: Option<u64>,
	tx_payment: &TransactionPayment,
	wait: bool,
) -> anyhow::Result<()> {
	let Some(highest) = highest_delivery_height else {
		return Ok(());
	};

	// Pick the height to anchor proof against: either wait until HB sees it,
	// or fall back to HB's current view (and skip the pair if neither works).
	let height = if highest > dest_height && wait {
		Some(
			wait_for_state_machine_update(
				dest.state_machine_id(),
				Arc::new(hyperbridge.clone()),
				dest.clone(),
				highest,
			)
			.await?,
		)
	} else if highest <= dest_height {
		Some(dest_height)
	} else {
		None
	};

	let Some(height) = height else {
		tracing::info!(
			target: crate::LOG_TARGET,
			source = %source.state_machine_id().state_id,
			dest = %dest.state_machine_id().state_id,
			"skipping — HB has no state machine update for dest at the required height yet",
		);
		return Ok(());
	};

	tracing::info!(
		target: crate::LOG_TARGET,
		source = %source.state_machine_id().state_id,
		dest = %dest.state_machine_id().state_id,
		"building claim proof from db",
	);
	let proofs = tx_payment
		.create_claim_proof(source_height, height, source.clone(), dest.clone(), hyperbridge)
		.await?;

	if proofs.is_empty() {
		tracing::trace!(
			target: crate::LOG_TARGET,
			source = %source.state_machine_id().state_id,
			dest = %dest.state_machine_id().state_id,
			"all fees already claimed in a previous run",
		);
		return Ok(());
	}

	observe_challenge_period(dest.clone(), Arc::new(hyperbridge.clone()), height).await?;

	use tesseract_primitives::HyperbridgeClaim;
	for proof in proofs {
		hyperbridge.accumulate_fees(proof.clone()).await?;
		if let Err(err) = tx_payment.delete_claimed_entries(proof.commitments).await {
			tracing::error!(
				target: crate::LOG_TARGET,
				?err,
				"failed to delete claimed fees from db; will be retried next run",
			);
		}
	}
	tracing::info!(
		target: crate::LOG_TARGET,
		source = %source.state_machine_id().state_id,
		dest = %dest.state_machine_id().state_id,
		"claim proofs submitted to hyperbridge",
	);
	Ok(())
}
