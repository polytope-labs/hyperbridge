// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Per-L2 quorum loop: subscribe to hyperbridge's `StateMachineUpdated`
//! events, query each provider's block-header state root at the same height,
//! and submit a veto when responding providers disagree among themselves or
//! with hyperbridge's recorded root. Transport errors never produce a veto
//! on their own.

use std::sync::Arc;

use alloy::providers::Provider;
use anyhow::{anyhow, Context};
use futures::StreamExt;
use ismp::consensus::{StateMachineHeight, StateMachineId};
use polkadot_sdk::sc_service::TaskManager;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient, SubstrateConfig};

use crate::config::{is_l2, ChainSection, FishermanConfig};

pub const LOG_TARGET: &str = "fisherman";

/// Two is the floor where unanimity is a meaningful signal. Below this we
/// abstain rather than veto.
const MIN_PROVIDERS_FOR_QUORUM: usize = 2;

/// Errors here fail the collator at startup. The task uses
/// `spawn_essential_handle`, so an internal panic tears the node down.
pub async fn spawn(config: FishermanConfig, task_manager: &TaskManager) -> anyhow::Result<()> {
	config.validate()?;

	let substrate_config = SubstrateConfig {
		state_machine: None,
		hashing: None,
		consensus_state_id: None,
		rpc_ws: config.hyperbridge.rpc_ws.clone(),
		max_rpc_payload_size: None,
		signer: Some(config.hyperbridge.signer.clone()),
		initial_height: None,
		max_concurent_queries: None,
		poll_interval: None,
		fee_token_decimals: None,
	};
	let resolved = substrate_config
		.resolve()
		.await
		.context("resolving hyperbridge SubstrateConfig for fisherman")?;
	let hb_client = SubstrateClient::<KeccakSubstrateChain>::new(resolved)
		.await
		.context("creating hyperbridge SubstrateClient for fisherman")?;
	let hyperbridge: Arc<dyn IsmpProvider> = Arc::new(hb_client);

	let mut spawned = 0usize;
	for (name, chain) in &config.chains {
		if chain.host_type != "evm" {
			continue;
		}
		let Some(consensus) = &chain.consensus else { continue };
		if !is_l2(&consensus.consensus_type) {
			continue;
		}
		spawn_l2_task(name.clone(), chain.clone(), hyperbridge.clone(), task_manager).await?;
		spawned += 1;
	}

	if spawned == 0 {
		log::warn!(
			target: LOG_TARGET,
			"no L2 chains configured; fisherman is idle. Add [<chain>.consensus] sections with type op_stack or arbitrum_orbit to enable monitoring",
		);
	} else {
		log::info!(target: LOG_TARGET, "started fisherman quorum task for {spawned} L2 chain(s)");
	}

	Ok(())
}

async fn spawn_l2_task(
	chain_name: String,
	chain: ChainSection,
	hyperbridge: Arc<dyn IsmpProvider>,
	task_manager: &TaskManager,
) -> anyhow::Result<()> {
	let mut providers: Vec<Arc<EvmClient>> = Vec::with_capacity(chain.rpc_urls.len());
	for rpc_url in &chain.rpc_urls {
		let cfg = EvmConfig { rpc_urls: vec![rpc_url.clone()], ..Default::default() };
		let resolved = cfg.resolve().await.with_context(|| {
			format!("resolving evm config for chain '{chain_name}' rpc '{rpc_url}'")
		})?;
		let client = EvmClient::new(resolved).await.with_context(|| {
			format!("constructing EvmClient for chain '{chain_name}' rpc '{rpc_url}'")
		})?;
		providers.push(Arc::new(client));
	}

	// All providers point at the same chain, so any one yields the shared id.
	let l2_state_machine_id = providers
		.first()
		.expect("validate() ensured rpc_urls is non-empty")
		.state_machine_id();

	// `spawn_essential_handle` requires a `&'static str` name.
	let log_name: &'static str = Box::leak(format!("fisherman-{chain_name}").into_boxed_str());

	task_manager
		.spawn_essential_handle()
		.spawn(log_name, Some("fisherman"), async move {
			match run_quorum_task(hyperbridge, providers, l2_state_machine_id).await {
				Ok(()) => log::error!(
					target: LOG_TARGET,
					"{log_name} terminated cleanly; this should never happen — the stream should run forever",
				),
				Err(e) => log::error!(
					target: LOG_TARGET,
					"{log_name} terminated with error: {e:?}",
				),
			}
		});

	Ok(())
}

async fn run_quorum_task(
	hyperbridge: Arc<dyn IsmpProvider>,
	providers: Vec<Arc<EvmClient>>,
	l2_state_machine_id: StateMachineId,
) -> anyhow::Result<()> {
	let mut update_stream = hyperbridge
		.state_machine_updates(l2_state_machine_id)
		.await
		.map_err(|e| anyhow!("subscribing to state_machine_updates failed: {e}"))?;

	log::info!(
		target: LOG_TARGET,
		"watching {l2_state_machine_id:?} across {} providers",
		providers.len(),
	);

	while let Some(item) = update_stream.next().await {
		let updates = match item {
			Ok(updates) => updates,
			Err(e) => {
				log::error!(
					target: LOG_TARGET,
					"stream error for {l2_state_machine_id:?}: {e:?}",
				);
				continue;
			},
		};

		for update in updates {
			let height =
				StateMachineHeight { id: l2_state_machine_id, height: update.latest_height };

			// Three response classes: state-root-present, definitive `Ok(None)`
			// (the height does not exist on the L2), and transport error.
			// Transport errors never produce a veto on their own.
			let mut state_roots = Vec::with_capacity(providers.len());
			let mut missing = 0usize;
			for provider in &providers {
				match provider.client.get_block(update.latest_height.into()).await {
					Ok(Some(header)) => state_roots.push(header.header.state_root),
					Ok(None) => {
						log::warn!(
							target: LOG_TARGET,
							"provider reports no block at height {} for {l2_state_machine_id:?}",
							update.latest_height,
						);
						missing += 1;
					},
					Err(e) => log::warn!(
						target: LOG_TARGET,
						"provider RPC error at height {} for {l2_state_machine_id:?}: {e:?}",
						update.latest_height,
					),
				}
			}

			let recorded = match hyperbridge.query_state_machine_commitment(height).await {
				Ok(commitment) => commitment,
				Err(e) => {
					log::warn!(
						target: LOG_TARGET,
						"could not fetch hyperbridge's recorded commitment at {height:?}: {e:?}",
					);
					continue;
				},
			};

			let responding = state_roots.len() + missing;
			if responding < MIN_PROVIDERS_FOR_QUORUM {
				log::warn!(
					target: LOG_TARGET,
					"only {responding} provider(s) responded for {l2_state_machine_id:?} at height {} (need at least {}), abstaining",
					update.latest_height,
					MIN_PROVIDERS_FOR_QUORUM,
				);
				continue;
			}

			// Quorum agrees the height does not exist on the L2 yet hyperbridge
			// has a commitment for it: fraud.
			if state_roots.is_empty() {
				log::error!(
					target: LOG_TARGET,
					"{missing} providers report no block at height {} for {l2_state_machine_id:?}; hyperbridge recorded a commitment anyway. Submitting veto.",
					update.latest_height,
				);
				if let Err(e) = hyperbridge.veto_state_commitment(height).await {
					log::error!(target: LOG_TARGET, "veto submission failed: {e:?}");
				}
				continue;
			}

			// Some providers see the block, others say it does not exist: no
			// quorum either way, abstain.
			if state_roots.len() < MIN_PROVIDERS_FOR_QUORUM {
				log::warn!(
					target: LOG_TARGET,
					"mixed responses for {l2_state_machine_id:?} at height {}: {} state_roots, {missing} missing. Abstaining.",
					update.latest_height,
					state_roots.len(),
				);
				continue;
			}

			let first = state_roots[0];
			let unanimous = state_roots.iter().all(|r| *r == first);
			if !unanimous {
				log::error!(
					target: LOG_TARGET,
					"providers disagree on {l2_state_machine_id:?} at height {}: {state_roots:?}. Submitting veto.",
					update.latest_height,
				);
				if let Err(e) = hyperbridge.veto_state_commitment(height).await {
					log::error!(target: LOG_TARGET, "veto submission failed: {e:?}");
				}
				continue;
			}

			if first.0 != recorded.state_root.0 {
				log::error!(
					target: LOG_TARGET,
					"hyperbridge recorded a state root that disagrees with quorum on {l2_state_machine_id:?} at height {} (recorded={:?}, quorum={:?}). Submitting veto.",
					update.latest_height,
					recorded.state_root,
					first,
				);
				if let Err(e) = hyperbridge.veto_state_commitment(height).await {
					log::error!(target: LOG_TARGET, "veto submission failed: {e:?}");
				}
			}
		}
	}

	Err(anyhow!("state_machine_updates stream for {l2_state_machine_id:?} terminated unexpectedly"))
}
