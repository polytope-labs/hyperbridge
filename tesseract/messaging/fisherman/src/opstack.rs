// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Opstack fisherman watcher. Polls Ethereum L1 for `DisputeGameCreated` events emitted by
//! configured `DisputeGameFactory` contracts and, for each new game, verifies the game's
//! claimed L2 `root_claim` against a 2/3·N+1 quorum of L2 RPC endpoints. On mismatch (or
//! quorum-of-missing-blocks) the game's proxy is permanently blacklisted via
//! `pallet-fishermen::blacklist_dispute_game` on hyperbridge.

use std::{sync::Arc, time::Duration};

use alloy::{
	eips::BlockId,
	primitives::{Address, B256},
	providers::Provider,
	rpc::types::Filter,
	sol_types::SolEvent,
};
use anyhow::anyhow;
use futures::future::join_all;
use geth_primitives::Header;
use ismp::{consensus::StateMachineId, host::StateMachine};
use op_host::abi::{DisputeGameFactory, FaultDisputeGame};
use op_verifier::calculate_output_root;
use primitive_types::{H160, H256};
use tesseract_evm::AlloyProvider;
use tesseract_primitives::{Hasher, IsmpProvider};

use crate::quorum::{decide, fetch_block_by_number, FetchOutcome, QuorumDecision};

/// Per-L2 configuration the watcher needs to evaluate a dispute game against an L2 RPC quorum.
#[derive(Clone)]
pub struct OpstackTarget {
	/// `StateMachineId` of the L2 chain (matches the on-chain blacklist key).
	pub state_machine_id: StateMachineId,
	/// `DisputeGameFactory` contract address on L1.
	pub factory: H160,
	/// `OptimismPortal2.l2ToL1MessagePasser` (a.k.a. message parser) contract on L2 — used to
	/// recompute the L2 output root.
	pub message_parser: H160,
	/// L2 RPC providers used for the 2/3·N+1 quorum check.
	pub l2_providers: Vec<Arc<AlloyProvider>>,
	/// L2 state machine (used purely for logging).
	pub state_machine: StateMachine,
}

/// Configuration for `fish_opstack`. One instance per L1 chain — the L1 provider is shared
/// across all the L2 targets it serves.
pub struct OpstackConfig {
	/// L1 execution-client provider (alloy).
	pub l1_provider: Arc<AlloyProvider>,
	/// L1 state machine (used purely for logging).
	pub l1_state_machine: StateMachine,
	/// All L2 rollups on this L1 that the fisherman should watch.
	pub targets: Vec<OpstackTarget>,
	/// Hyperbridge provider — receives the `blacklist_dispute_game` extrinsics.
	pub hyperbridge: Arc<dyn IsmpProvider>,
	/// Poll interval. Defaults to 30 s if `None`.
	pub poll_interval: Option<Duration>,
}

/// Run the opstack fisherman task. The returned future runs until either the L1 provider
/// disappears or an unrecoverable error happens; transient RPC failures are logged and
/// retried on the next poll.
pub async fn fish_opstack(cfg: OpstackConfig) -> Result<(), anyhow::Error> {
	if cfg.targets.is_empty() {
		log::info!(target: crate::LOG_TARGET, "fish_opstack on {}: no targets configured, exiting", cfg.l1_state_machine);
		return Ok(());
	}

	let interval = cfg.poll_interval.unwrap_or(Duration::from_secs(30));
	let mut last_scanned = cfg.l1_provider.get_block_number().await?;

	loop {
		tokio::time::sleep(interval).await;

		let tip = match cfg.l1_provider.get_block_number().await {
			Ok(n) => n,
			Err(e) => {
				log::warn!(target: crate::LOG_TARGET, "fish_opstack: L1 tip fetch failed: {e:?}");
				continue;
			},
		};
		if tip <= last_scanned {
			continue;
		}
		let from = last_scanned + 1;

		for target in &cfg.targets {
			if let Err(e) = scan_target(&cfg, target, from, tip).await {
				log::warn!(
					target: crate::LOG_TARGET,
					"fish_opstack {} -> {}: scan window [{from}, {tip}] failed: {e:?}",
					cfg.l1_state_machine, target.state_machine,
				);
			}
		}

		last_scanned = tip;
	}
}

async fn scan_target(
	cfg: &OpstackConfig,
	target: &OpstackTarget,
	from: u64,
	to: u64,
) -> Result<(), anyhow::Error> {
	let factory_addr = Address::from_slice(&target.factory.0);
	let filter = Filter::new()
		.address(factory_addr)
		.event_signature(DisputeGameFactory::DisputeGameCreated::SIGNATURE_HASH)
		.from_block(from)
		.to_block(to);
	let logs = cfg.l1_provider.get_logs(&filter).await?;

	for log in logs {
		let event = match DisputeGameFactory::DisputeGameCreated::decode_log(&log.inner) {
			Ok(decoded) => decoded.data,
			Err(_) => continue,
		};
		let proxy_h160 = H160(event.disputeProxy.0 .0);

		match evaluate(cfg, target, event.rootClaim, proxy_h160, to).await {
			Ok(true) => {
				log::trace!(
					target: crate::LOG_TARGET,
					"fish_opstack: proxy {proxy_h160:?} on {} agrees with L2 quorum",
					target.state_machine,
				);
			},
			Ok(false) => {
				log::warn!(
					target: crate::LOG_TARGET,
					"fish_opstack: blacklisting opstack dispute-game proxy {:?} on {} \
					(rootClaim {:?}, gameType {})",
					proxy_h160, target.state_machine, event.rootClaim, event.gameType,
				);
				if let Err(e) = cfg
					.hyperbridge
					.blacklist_dispute_game(target.state_machine_id, proxy_h160)
					.await
				{
					log::error!(
						target: crate::LOG_TARGET,
						"fish_opstack: submit blacklist_dispute_game for {proxy_h160:?} failed: {e:?}",
					);
				}
			},
			Err(e) => {
				log::warn!(
					target: crate::LOG_TARGET,
					"fish_opstack: evaluation of proxy {proxy_h160:?} failed: {e:?}. Abstaining for this poll.",
				);
			},
		}
	}

	Ok(())
}

/// Verify a dispute game's claimed L2 output root against an L2 RPC quorum. Returns
/// `Ok(true)` if the quorum agrees with the on-chain claim, `Ok(false)` if it disagrees or
/// reports the L2 block as missing.
async fn evaluate(
	cfg: &OpstackConfig,
	target: &OpstackTarget,
	root_claim: B256,
	proxy: H160,
	l1_block: u64,
) -> Result<bool, anyhow::Error> {
	if target.l2_providers.is_empty() {
		return Err(anyhow!(
			"fish_opstack: no l2 providers configured for {}",
			target.state_machine
		));
	}

	let l2_block_number = read_l2_block_number(&cfg.l1_provider, proxy, l1_block).await?;

	let outcomes = join_all(target.l2_providers.iter().map(|p| async move {
		compute_quorum_root(p.as_ref(), &target.message_parser, l2_block_number).await
	}))
	.await;

	let claim = H256(root_claim.0);
	let decision = decide(outcomes, |computed: &H256| computed.0 == claim.0);
	Ok(match decision {
		QuorumDecision::Verified => true,
		QuorumDecision::Mismatch | QuorumDecision::MissingFromQuorum => false,
		QuorumDecision::InsufficientQuorum => {
			log::trace!(
				target: crate::LOG_TARGET,
				"fish_opstack: insufficient quorum for proxy {proxy:?} at l2 block {l2_block_number}, abstaining",
			);
			true
		},
	})
}

/// Read the proxy's claimed L2 block number from L1 storage. The pinned `FaultDisputeGame`
/// ABI exposes this as `l2SequenceNumber()` (the newer name; older deployments used
/// `l2BlockNumber()` but the op-host ABI binding is generated from the current upstream and
/// only carries `l2SequenceNumber`).
async fn read_l2_block_number(
	l1_provider: &AlloyProvider,
	proxy: H160,
	at_block: u64,
) -> Result<u64, anyhow::Error> {
	let proxy_addr = Address::from_slice(&proxy.0);
	let contract = FaultDisputeGame::new(proxy_addr, l1_provider);
	let n = contract
		.l2SequenceNumber()
		.block(BlockId::number(at_block))
		.call()
		.await
		.map_err(|e| anyhow!("fish_opstack: proxy {proxy:?}.l2SequenceNumber() failed: {e:?}"))?;
	Ok(n.to::<u64>())
}

/// Per-provider: fetch the L2 block at `height`, fetch the message-parser storage proof at
/// that block, and compute the expected `output_root`.
async fn compute_quorum_root(
	provider: &AlloyProvider,
	message_parser: &H160,
	height: u64,
) -> FetchOutcome<H256> {
	let block = match fetch_block_by_number(provider, height).await {
		FetchOutcome::Found(b) => b,
		FetchOutcome::Missing => return FetchOutcome::Missing,
		FetchOutcome::Errored => return FetchOutcome::Errored,
	};
	let parser_addr = Address::from_slice(&message_parser.0);
	let proof = match provider
		.get_proof(parser_addr, vec![])
		.block_id(BlockId::number(height))
		.await
	{
		Ok(p) => p,
		Err(e) => {
			log::warn!(
				target: crate::LOG_TARGET,
				"fish_opstack: message-parser proof at L2 block {height} failed: {e:?}",
			);
			return FetchOutcome::Errored;
		},
	};
	let l2_header: geth_primitives::CodecHeader = block.into();
	let l2_block_hash = Header::from(&l2_header).hash::<Hasher>();
	let computed = calculate_output_root::<Hasher>(
		H256::zero(),
		l2_header.state_root,
		proof.storage_hash.0.into(),
		l2_block_hash,
	);
	FetchOutcome::Found(computed)
}

// Silence the unused state machine field warning on `OpstackTarget` when only used for logs.
#[allow(dead_code)]
fn _state_machine_used(s: StateMachine) -> StateMachine {
	s
}
