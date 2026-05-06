// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Collator-side fisherman wrapper. Parses the operator's tesseract toml,
//! constructs a Hyperbridge [`IsmpProvider`] and one per-L2 [`IsmpProvider`]
//! from canonical [`tesseract_config::AnyConfig`] entries, and hands each
//! pair to [`tesseract_fisherman::fish`] — the same task implementation the
//! relayer used to spawn. Multi-RPC quorum and 3-attempt retry on transport
//! errors live inside [`tesseract_evm::byzantine`], not here.

pub mod config;

use std::sync::Arc;

use anyhow::{anyhow, Context};
use ismp::host::StateMachine;
use polkadot_sdk::sc_service::TaskManager;
use tesseract_config::AnyConfig;
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};

pub use config::FishermanConfig;

pub const LOG_TARGET: &str = "fisherman";

/// Build the Hyperbridge provider, build per-L2 providers from the toml,
/// and spawn one [`tesseract_fisherman::fish`] task per L2.
///
/// Errors here fail the collator at startup. The downstream tasks use
/// `spawn_essential_handle`, so any internal panic tears the node down —
/// which is the desired behavior: a collator that can't fish is a collator
/// that shouldn't be producing blocks.
pub async fn spawn(config: FishermanConfig, task_manager: &TaskManager) -> anyhow::Result<()> {
	config.validate()?;

	let hyperbridge_substrate = config.hyperbridge.resolve().await.context(
		"resolving hyperbridge SubstrateConfig (rpc_ws / state_machine lookup) for fisherman",
	)?;
	let hyperbridge_state_machine = hyperbridge_substrate.state_machine();
	let hb_client = SubstrateClient::<KeccakSubstrateChain>::new(hyperbridge_substrate)
		.await
		.context("creating hyperbridge SubstrateClient for fisherman")?;
	let hyperbridge: Arc<dyn IsmpProvider> = Arc::new(hb_client);

	let mut spawned = 0usize;
	for (name, cfg) in config.chains {
		let AnyConfig::Evm(evm_cfg) = &cfg else { continue };
		let StateMachine::Evm(chain_id) = evm_cfg.state_machine() else { continue };
		if !tesseract_evm::registry::is_supported_l2(chain_id as u64) {
			continue;
		}

		let l2: Arc<dyn IsmpProvider> = cfg
			.into_client(hyperbridge.clone())
			.await
			.with_context(|| format!("constructing IsmpProvider for L2 chain '{name}'"))?;

		// Match the relayer's argument order: `chain_a` is the chain we
		// subscribe to for `StateMachineUpdated` events (hyperbridge), and
		// `chain_b` is the L2 whose `check_for_byzantine_attack` runs the
		// quorum across its rpc providers and sends a veto on hyperbridge
		// (the counterparty).
		tesseract_fisherman::fish(hyperbridge.clone(), l2, task_manager, hyperbridge_state_machine)
			.await
			.with_context(|| format!("spawning fisherman task for L2 chain '{name}'"))?;
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
