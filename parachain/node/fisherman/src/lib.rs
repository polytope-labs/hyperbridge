// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Collator-side fisherman wrapper. Parses the consolidated relayer's
//! [`HyperbridgeConfig`], constructs a Hyperbridge [`IsmpProvider`] and one
//! per-L2 [`IsmpProvider`] from the existing canonical config types, and
//! hands each pair to [`tesseract_fisherman::fish`] — the same task
//! implementation the relayer used to spawn. Multi-RPC quorum and 3-attempt
//! retry on transport errors live inside [`tesseract_evm::byzantine`], not
//! here.
//!
//! The signer used to sign veto extrinsics is always read from the local
//! AURA keystore — collators already have that key loaded for block
//! authoring, so the fisherman rides along on the same identity. Anything
//! the operator put in `[hyperbridge].signer` is the relayer's concern and
//! is not consulted here.

pub mod config;

use std::{path::Path, sync::Arc};

use anyhow::{anyhow, Context};
use ismp::host::StateMachine;
use polkadot_sdk::{
	sc_keystore::LocalKeystore,
	sc_service::TaskManager,
	sp_application_crypto::key_types::AURA,
	sp_consensus_aura::sr25519::{AuthorityId, AuthorityPair},
	sp_core::crypto::Pair as PairT,
	sp_keystore::Keystore,
};
use tesseract::config::HyperbridgeConfig;
use tesseract_config::AnyConfig;
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};

pub const LOG_TARGET: &str = "fisherman";

/// Parse the operator's tesseract toml at `path` (same shape the relayer
/// consumes), validate it for collator use, build the Hyperbridge provider
/// and per-L2 providers, and spawn one [`tesseract_fisherman::fish`] task
/// per supported L2.
///
/// Errors here fail the collator at startup. The downstream tasks use
/// `spawn_essential_handle`, so any internal panic tears the node down —
/// which is the desired behavior: a collator that can't fish is a collator
/// that shouldn't be producing blocks.
pub async fn spawn(
	path: &Path,
	keystore: Arc<LocalKeystore>,
	task_manager: &TaskManager,
) -> anyhow::Result<()> {
	let path_str = path
		.to_str()
		.ok_or_else(|| anyhow!("tesseract config path is not valid UTF-8: {}", path.display()))?;
	let mut config = HyperbridgeConfig::parse_conf(path_str)
		.await
		.with_context(|| format!("parsing tesseract config at {}", path.display()))?;

	// The fisherman's signer is always the local AURA key
	config.hyperbridge.substrate.signer = Some(aura_seed_from_keystore(&keystore)?);

	config::validate(&config)?;

	let hyperbridge_substrate = config.hyperbridge.substrate.clone().resolve().await.context(
		"resolving hyperbridge SubstrateConfig (rpc_ws / state_machine lookup) for fisherman",
	)?;
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

		// Match the relayer's argument order: `chain_a` is the chain we
		// subscribe to for `StateMachineUpdated` events (hyperbridge), and
		// `chain_b` is the L2 whose `check_for_byzantine_attack` runs the
		// quorum across its rpc providers and sends a veto on hyperbridge
		// (the counterparty).
		tesseract_fisherman::fish(hyperbridge.clone(), l2, task_manager, hyperbridge_state_machine)
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

/// Read the first AURA sr25519 key out of the local keystore and return its
/// seed as a `0x`-prefixed hex string — the same shape
/// [`HyperbridgeConfig::hyperbridge.substrate.signer`] expects when the
/// operator sets it explicitly.
fn aura_seed_from_keystore(keystore: &LocalKeystore) -> anyhow::Result<String> {
	let public = Keystore::sr25519_public_keys(keystore, AURA)
		.into_iter()
		.next()
		.ok_or_else(|| anyhow!(
			"[hyperbridge].signer not set in tesseract config and no AURA sr25519 key in the local keystore — load one (e.g. via `key insert`) or set the signer explicitly"
		))?;
	let authority_id: AuthorityId = public.into();
	let pair = keystore
		.key_pair::<AuthorityPair>(&authority_id)
		.map_err(|e| anyhow!("reading AURA key pair from local keystore: {e:?}"))?
		.ok_or_else(|| {
			anyhow!(
			"AURA public key {authority_id:?} present in keystore but its private seed is not retrievable"
		)
		})?;
	Ok(format!("0x{}", hex::encode(pair.to_raw_vec())))
}
