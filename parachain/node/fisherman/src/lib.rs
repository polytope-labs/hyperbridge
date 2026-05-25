// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Collator side fisherman. Parses the operator's tesseract toml, builds a
//! hyperbridge provider and one provider per supported L2, and hands each
//! pair to [`tesseract_fisherman::fish`].
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

use std::{future::Future, path::Path, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{anyhow, Context};
use ismp::host::StateMachine;
use polkadot_sdk::sc_service::{SpawnEssentialTaskHandle, TaskManager};
use tesseract::config::HyperbridgeConfig;
use tesseract_config::AnyConfig;
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
/// [`tesseract_fisherman::fish`] task per supported L2. Must run after
/// `sc_service::spawn_tasks` has brought the local RPC server up and the
/// node has finished syncing (see [`spawn_when_synced`]).
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
	for (state_machine, per_chain) in config.chains {
		let AnyConfig::Evm(evm_cfg) = &per_chain.messaging else { continue };
		let StateMachine::Evm(chain_id) = evm_cfg.state_machine() else { continue };
		if !tesseract_evm::registry::is_supported_l2(chain_id as u64) {
			continue;
		}

		let l2: Arc<dyn IsmpProvider> = per_chain
			.messaging
			.into_client(hyperbridge.clone())
			.await
			.with_context(|| format!("constructing IsmpProvider for L2 {state_machine}"))?;

		tesseract_fisherman::fish(hyperbridge.clone(), l2, &spawn_handle, hyperbridge_state_machine)
			.await
			.with_context(|| format!("spawning fisherman task for L2 {state_machine}"))?;
		spawned += 1;
	}

	if spawned == 0 {
		return Err(anyhow!(
			"no L2 chains configured for fisherman; at least one supported L2 section is required"
		));
	}
	log::info!(target: LOG_TARGET, "started fisherman quorum task for {spawned} L2 chain(s)");
	Ok(())
}
