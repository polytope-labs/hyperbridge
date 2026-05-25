// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Collator side fisherman. Parses the operator's tesseract toml, builds a
//! hyperbridge provider and one provider per supported L2, and hands each
//! pair to [`tesseract_fisherman::fish`].
//!
//! Also spawns the L1 rollup-claim watchers ([`tesseract_fisherman::fish_opstack`] /
//! [`fish_arbitrum`]) when the operator's config has consensus sections for
//! opstack or arbitrum chains.
//!
//! Startup ordering matters. [`load_and_validate`] only reads the file and
//! checks fields, so it is safe to call before chain init and gives the
//! operator a fast error on a bad config. [`spawn`] parses the toml through
//! [`HyperbridgeConfig::parse_conf`], which dials `[hyperbridge].rpc_ws`.
//! On a normal collator that URL points back at this node, so [`spawn`]
//! must run after `sc_service::spawn_tasks` has opened the RPC port. Doing
//! it earlier blocks the task that is supposed to open that port.
//!
//! The RPC port being open is not enough: until the node has finished
//! syncing, that RPC serves stale chain state and the fisherman would build
//! its providers against it. [`spawn_when_synced`] holds the spawn back
//! behind a sync-status poll so the providers are only constructed once the
//! local node has caught up.

pub mod config;

use std::{collections::HashMap, future::Future, path::Path, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{anyhow, Context};
use ismp::{
	consensus::{ConsensusStateId, StateMachineId},
	host::StateMachine,
};
use polkadot_sdk::sc_service::{SpawnEssentialTaskHandle, TaskManager};
use tesseract::config::HyperbridgeConfig;
use tesseract_config::AnyConfig;
use tesseract_consensus_config::AnyConfig as ConsensusConfig;
use tesseract_fisherman::{
	fish_arbitrum, fish_opstack, ArbitrumConfig, ArbitrumKind, ArbitrumTarget, OpstackConfig,
	OpstackTarget,
};
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};

pub const LOG_TARGET: &str = "fisherman";

/// Interval between sync-status polls while the fisherman startup is deferred.
const SYNC_POLL_INTERVAL: Duration = Duration::from_secs(15);

/// Read the tesseract toml at `path` and run the sync preflight checks.
/// Performs no network I/O so the call is safe to make before any chain
/// init runs.
pub async fn load_and_validate(path: &Path) -> anyhow::Result<()> {
	let toml_str = tokio::fs::read_to_string(path)
		.await
		.with_context(|| format!("reading tesseract config at {}", path.display()))?;
	config::preflight(&toml_str)
		.with_context(|| format!("validating tesseract config at {}", path.display()))?;
	Ok(())
}

/// Spawn a watcher that holds the fisherman back until the local node has
/// finished syncing, then runs [`spawn`].
///
/// The config has already been checked by [`load_and_validate`], so only the
/// network-dependent startup is deferred. `is_synced` is polled every
/// [`SYNC_POLL_INTERVAL`] and the fisherman starts on the first poll that
/// reports the node synced. The watcher itself is a non-essential task that
/// exits once the fisherman is running; the per-L2 tasks spawned by [`spawn`]
/// stay essential.
pub fn spawn_when_synced<C, Fut>(task_manager: &TaskManager, path: PathBuf, is_synced: C)
where
	C: Fn() -> Fut + Send + 'static,
	Fut: Future<Output = bool> + Send + 'static,
{
	let spawn_handle = task_manager.spawn_essential_handle();
	task_manager.spawn_handle().spawn("fisherman-sync-watcher", "fisherman", async move {
		log::info!(
			target: LOG_TARGET,
			"deferring fisherman startup until the local node has finished syncing",
		);
		while !is_synced().await {
			log::debug!(target: LOG_TARGET, "local node still syncing; fisherman startup deferred");
			tokio::time::sleep(SYNC_POLL_INTERVAL).await;
		}
		log::info!(target: LOG_TARGET, "local node synced; starting fisherman");
		if let Err(e) = spawn(&path, spawn_handle).await {
			log::error!(target: LOG_TARGET, "fisherman failed to start after the node synced: {e:?}");
		}
	});
}

/// Build the hyperbridge and per L2 providers and spawn one
/// [`tesseract_fisherman::fish`] task per supported L2, plus the L1
/// rollup-claim watchers for each L1 chain that has opstack or arbitrum
/// consensus sections configured. Must run after `sc_service::spawn_tasks`
/// has brought the local RPC server up and the node has finished syncing
/// (see [`spawn_when_synced`]).
pub async fn spawn(path: &Path, spawn_handle: SpawnEssentialTaskHandle) -> anyhow::Result<()> {
	let path_str = path
		.to_str()
		.ok_or_else(|| anyhow!("tesseract config path is not valid UTF-8: {}", path.display()))?;
	let config = HyperbridgeConfig::parse_conf(path_str)
		.await
		.with_context(|| format!("parsing tesseract config at {}", path.display()))?;
	config::validate(&config)?;

	let hyperbridge_substrate = config
		.hyperbridge
		.substrate
		.clone()
		.resolve()
		.await
		.context("resolving hyperbridge SubstrateConfig for fisherman")?;
	let hyperbridge_state_machine = hyperbridge_substrate.state_machine();
	let hb_client = SubstrateClient::<KeccakSubstrateChain>::new(hyperbridge_substrate)
		.await
		.context("creating hyperbridge SubstrateClient for fisherman")?;
	let hyperbridge: Arc<dyn IsmpProvider> = Arc::new(hb_client);

	let mut spawned = 0usize;

	// Pair-based byzantine fisherman, one task per supported L2.
	for (state_machine, per_chain) in &config.chains {
		let AnyConfig::Evm(evm_cfg) = &per_chain.messaging else { continue };
		let StateMachine::Evm(chain_id) = evm_cfg.state_machine() else { continue };
		if !tesseract_evm::registry::is_supported_l2(chain_id as u64) {
			continue;
		}

		let l2: Arc<dyn IsmpProvider> = per_chain
			.messaging
			.clone()
			.into_client(hyperbridge.clone())
			.await
			.with_context(|| format!("constructing IsmpProvider for L2 {state_machine}"))?;

		tesseract_fisherman::fish(hyperbridge.clone(), l2, &spawn_handle, hyperbridge_state_machine)
			.await
			.with_context(|| format!("spawning fisherman task for L2 {state_machine}"))?;
		spawned += 1;
	}

	// L1 rollup-claim watchers, one task per (L1, kind).
	spawn_rollup_watchers(&config, &hyperbridge, &spawn_handle).await?;

	if spawned == 0 {
		log::warn!(
			target: LOG_TARGET,
			"no L2 chains configured for the pair fisherman; only L1 rollup watchers (if any) will run",
		);
	} else {
		log::info!(target: LOG_TARGET, "started fisherman quorum task for {spawned} L2 chain(s)");
	}
	Ok(())
}

/// Collects all opstack and arbitrum consensus sections from the operator's
/// config, groups them by L1 state machine, builds an L1 provider per group,
/// and spawns one `fish_opstack` and up to two `fish_arbitrum` tasks per L1.
async fn spawn_rollup_watchers(
	config: &HyperbridgeConfig,
	hyperbridge: &Arc<dyn IsmpProvider>,
	spawn_handle: &SpawnEssentialTaskHandle,
) -> anyhow::Result<()> {
	let mut opstack_by_l1: HashMap<StateMachine, (Vec<String>, Vec<OpstackTarget>)> =
		HashMap::new();
	let mut arbitrum_by_l1: HashMap<StateMachine, (Vec<String>, Vec<ArbitrumTarget>)> =
		HashMap::new();

	for (state_machine, per_chain) in &config.chains {
		let Some(consensus) = &per_chain.consensus else { continue };
		// L2 RPCs come from the messaging EVM config (same endpoints messaging uses for inbound).
		let AnyConfig::Evm(evm_cfg) = &per_chain.messaging else { continue };
		let l2_providers = match tesseract_evm::create_provider(&evm_cfg.rpc_urls) {
			Ok(p) => vec![Arc::new(p)],
			Err(e) => {
				log::warn!(
					target: LOG_TARGET,
					"fisherman: skipping {state_machine} — failed to build L2 provider: {e:?}",
				);
				continue;
			},
		};

		match consensus {
			ConsensusConfig::OpStack { inner } => {
				let host = &inner.host;
				let Some(factory) = host.dispute_game_factory else {
					log::debug!(
						target: LOG_TARGET,
						"fisherman: {state_machine} has opstack consensus but no dispute_game_factory; skipping",
					);
					continue;
				};
				let entry = opstack_by_l1
					.entry(host.l1_state_machine)
					.or_insert_with(|| (host.ethereum_rpc_url.clone(), Vec::new()));
				entry.1.push(OpstackTarget {
					state_machine_id: StateMachineId {
						state_id: *state_machine,
						consensus_state_id: consensus_state_id_from_str(&host.consensus_state_id)?,
					},
					factory,
					message_parser: host.message_parser,
					l2_providers: l2_providers.clone(),
					state_machine: *state_machine,
				});
			},
			ConsensusConfig::ArbitrumOrbit { inner } => {
				// Only BoLD assertions are watched. Pre-BoLD Orbit chains have been migrated;
				// spawning an `ArbitrumKind::Orbit` watcher would just emit empty-event noise
				// while still costing one L1 `eth_getLogs` per poll.
				let host = &inner.host;
				let entry = arbitrum_by_l1
					.entry(host.l1_state_machine)
					.or_insert_with(|| (host.ethereum_rpc_url.clone(), Vec::new()));
				entry.1.push(ArbitrumTarget {
					state_machine_id: StateMachineId {
						state_id: *state_machine,
						consensus_state_id: consensus_state_id_from_str(&host.consensus_state_id)?,
					},
					rollup_core: host.rollup_core,
					l2_providers: l2_providers.clone(),
					state_machine: *state_machine,
					kind: ArbitrumKind::Bold,
				});
			},
			_ => {},
		}
	}

	for (l1_state_machine, (rpc_urls, targets)) in opstack_by_l1 {
		let l1_provider = Arc::new(tesseract_evm::create_provider(&rpc_urls).with_context(|| {
			format!("fisherman: building L1 provider for opstack watcher on {l1_state_machine}")
		})?);
		let cfg = OpstackConfig {
			l1_provider,
			l1_state_machine,
			targets,
			hyperbridge: hyperbridge.clone(),
			poll_interval: None,
		};
		let name = format!("fisherman-opstack-{l1_state_machine}");
		spawn_handle.spawn_blocking(
			Box::leak(Box::new(name.clone())),
			"fisherman",
			async move {
				let res = fish_opstack(cfg).await;
				log::error!(target: LOG_TARGET, "{name} terminated: {res:?}");
			},
		);
	}

	for (l1_state_machine, (rpc_urls, targets)) in arbitrum_by_l1 {
		let l1_provider = Arc::new(tesseract_evm::create_provider(&rpc_urls).with_context(|| {
			format!("fisherman: building L1 provider for arbitrum watcher on {l1_state_machine}")
		})?);
		let cfg = ArbitrumConfig {
			l1_provider,
			l1_state_machine,
			targets,
			hyperbridge: hyperbridge.clone(),
			poll_interval: None,
		};
		let name = format!("fisherman-arbitrum-{l1_state_machine}");
		spawn_handle.spawn_blocking(
			Box::leak(Box::new(name.clone())),
			"fisherman",
			async move {
				let res = fish_arbitrum(cfg).await;
				log::error!(target: LOG_TARGET, "{name} terminated: {res:?}");
			},
		);
	}

	Ok(())
}

fn consensus_state_id_from_str(s: &str) -> anyhow::Result<ConsensusStateId> {
	let bytes = s.as_bytes();
	if bytes.len() != 4 {
		return Err(anyhow!(
			"consensus_state_id must be exactly 4 bytes, got {} for {:?}",
			bytes.len(),
			s
		));
	}
	let mut id = [0u8; 4];
	id.copy_from_slice(bytes);
	Ok(id)
}
