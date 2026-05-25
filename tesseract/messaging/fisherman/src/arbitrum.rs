// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Arbitrum fisherman watcher. Polls Ethereum L1 for `NodeCreated` (Orbit / AnyTrust) and
//! `AssertionCreated` (BoLD) events emitted by configured `RollupCore` contracts and, for
//! each new event, verifies the asserted L2 block (state root and send root) against a
//! 2/3·N+1 quorum of L2 RPC endpoints. On mismatch (or quorum-of-missing-blocks) the claim
//! is permanently blacklisted via `pallet-fishermen::blacklist_arbitrum_claim` on
//! hyperbridge — keyed by the BoLD `assertionHash` or the Orbit `orbit_claim_hash`.

use std::{sync::Arc, time::Duration};

use alloy::{
	primitives::Address,
	providers::Provider,
	rpc::types::Filter,
	sol_types::SolEvent,
};
use anyhow::anyhow;
use arbitrum_verifier::{
	compute_assertion_hash, get_state_hash, orbit_claim_hash, AssertionState, GlobalState,
	MachineStatus,
};
use futures::future::join_all;
use geth_primitives::Header;
use ismp::{consensus::StateMachineId, host::StateMachine};
use primitive_types::{H256, U256};
use tesseract_evm::AlloyProvider;
use tesseract_primitives::{Hasher, IsmpProvider};

use crate::quorum::{decide, fetch_block_by_hash, FetchOutcome, QuorumDecision};

alloy::sol! {
	struct GlobalStateSol {
		bytes32[2] bytes32Vals;
		uint64[2] u64Vals;
	}

	struct AssertionStateSol {
		GlobalStateSol globalState;
		uint8 machineStatus;
		bytes32 endHistoryRoot;
	}

	struct AssertionSol {
		AssertionStateSol beforeState;
		AssertionStateSol afterState;
		uint64 numBlocks;
	}

	struct AssertionLite {
		GlobalStateSol globalStateAfter;
		uint8 machineStatusAfter;
	}

	#[sol(rpc)]
	contract IRollupOrbit {
		event NodeCreated(
			uint64 indexed nodeNum,
			bytes32 indexed parentNodeHash,
			bytes32 indexed nodeHash,
			bytes32 executionHash,
			AssertionSol assertion,
			bytes32 afterInboxBatchAcc,
			bytes32 wasmModuleRoot,
			uint256 inboxMaxCount
		);
	}

	#[sol(rpc)]
	contract IRollupBold {
		event AssertionCreated(
			bytes32 indexed assertionHash,
			bytes32 indexed parentAssertionHash,
			AssertionSol assertion,
			bytes32 afterInboxBatchAcc,
			bytes32 inboxAcc,
			bytes32 wasmModuleRoot,
			uint256 requiredStake,
			address challengeManager,
			uint64 challengePeriodBlocks
		);
	}
}

/// Per-L2 configuration for the arbitrum fisherman.
#[derive(Clone)]
pub struct ArbitrumTarget {
	pub state_machine_id: StateMachineId,
	pub rollup_core: H160Addr,
	pub l2_providers: Vec<Arc<AlloyProvider>>,
	pub state_machine: StateMachine,
	pub kind: ArbitrumKind,
}

#[derive(Clone, Copy, Debug)]
pub enum ArbitrumKind {
	Orbit,
	Bold,
}

/// Re-export of `primitive_types::H160` to keep config-struct surface unambiguous in callers.
pub type H160Addr = primitive_types::H160;

pub struct ArbitrumConfig {
	pub l1_provider: Arc<AlloyProvider>,
	pub l1_state_machine: StateMachine,
	pub targets: Vec<ArbitrumTarget>,
	pub hyperbridge: Arc<dyn IsmpProvider>,
	pub poll_interval: Option<Duration>,
	pub l1_finality_lag: u64,
}

/// Run the arbitrum fisherman task. Runs forever; transient RPC failures are logged.
pub async fn fish_arbitrum(cfg: ArbitrumConfig) -> Result<(), anyhow::Error> {
	if cfg.targets.is_empty() {
		log::info!(target: crate::LOG_TARGET, "fish_arbitrum on {}: no targets configured, exiting", cfg.l1_state_machine);
		return Ok(());
	}

	let interval = cfg.poll_interval.unwrap_or(Duration::from_secs(30));
	let mut last_scanned =
		cfg.l1_provider.get_block_number().await?.saturating_sub(cfg.l1_finality_lag);

	loop {
		tokio::time::sleep(interval).await;

		let tip = match cfg.l1_provider.get_block_number().await {
			Ok(n) => n,
			Err(e) => {
				log::warn!(target: crate::LOG_TARGET, "fish_arbitrum: L1 tip fetch failed: {e:?}");
				continue;
			},
		};
		let scan_to = tip.saturating_sub(cfg.l1_finality_lag);
		if scan_to <= last_scanned {
			continue;
		}
		let from = last_scanned + 1;

		for target in &cfg.targets {
			if let Err(e) = scan_target(&cfg, target, from, scan_to).await {
				log::warn!(
					target: crate::LOG_TARGET,
					"fish_arbitrum {} -> {} ({:?}): scan window [{from}, {scan_to}] failed: {e:?}",
					cfg.l1_state_machine, target.state_machine, target.kind,
				);
			}
		}

		last_scanned = scan_to;
	}
}

async fn scan_target(
	cfg: &ArbitrumConfig,
	target: &ArbitrumTarget,
	from: u64,
	to: u64,
) -> Result<(), anyhow::Error> {
	let rollup_addr = Address::from_slice(&target.rollup_core.0);
	let sig = match target.kind {
		ArbitrumKind::Orbit => IRollupOrbit::NodeCreated::SIGNATURE_HASH,
		ArbitrumKind::Bold => IRollupBold::AssertionCreated::SIGNATURE_HASH,
	};
	let filter = Filter::new()
		.address(rollup_addr)
		.event_signature(sig)
		.from_block(from)
		.to_block(to);
	let logs = cfg.l1_provider.get_logs(&filter).await?;

	for log in logs {
		let (claim, after) = match target.kind {
			ArbitrumKind::Orbit => {
				let Ok(decoded) = IRollupOrbit::NodeCreated::decode_log(&log.inner) else {
					continue;
				};
				let ev = decoded.data;
				let after = decode_after_state(&ev.assertion.afterState);
				// inbox_max_count is in the event payload for Orbit.
				let state_hash = get_state_hash::<Hasher>(
					after.global_state,
					after.machine_status,
					U256::from_big_endian(&ev.inboxMaxCount.to_be_bytes::<32>()),
				);
				(orbit_claim_hash::<Hasher>(state_hash, ev.nodeNum), after)
			},
			ArbitrumKind::Bold => {
				let Ok(decoded) = IRollupBold::AssertionCreated::decode_log(&log.inner) else {
					continue;
				};
				let ev = decoded.data;
				let after = decode_after_state(&ev.assertion.afterState);
				let assertion_state = AssertionState {
					global_state: after.global_state,
					machine_status: after.machine_status,
					end_history_root: H256(ev.assertion.afterState.endHistoryRoot.0),
				};
				let assertion_hash = compute_assertion_hash(
					H256(ev.parentAssertionHash.0),
					assertion_state.hash(),
					H256(ev.afterInboxBatchAcc.0),
				);
				(assertion_hash, after)
			},
		};

		match evaluate(target, &after).await {
			Ok(true) => {
				log::trace!(
					target: crate::LOG_TARGET,
					"fish_arbitrum: claim {claim:?} on {} agrees with L2 quorum",
					target.state_machine,
				);
			},
			Ok(false) => {
				log::warn!(
					target: crate::LOG_TARGET,
					"fish_arbitrum: blacklisting arbitrum claim {claim:?} on {} ({:?})",
					target.state_machine, target.kind,
				);
				if let Err(e) = cfg
					.hyperbridge
					.blacklist_arbitrum_claim(target.state_machine_id, claim)
					.await
				{
					log::error!(
						target: crate::LOG_TARGET,
						"fish_arbitrum: submit blacklist_arbitrum_claim for {claim:?} failed: {e:?}",
					);
				}
			},
			Err(e) => {
				log::warn!(
					target: crate::LOG_TARGET,
					"fish_arbitrum: evaluation of claim {claim:?} failed: {e:?}. Abstaining for this poll.",
				);
			},
		}
	}

	Ok(())
}

struct AfterState {
	global_state: GlobalState,
	machine_status: MachineStatus,
}

fn decode_after_state(sol: &AssertionStateSol) -> AfterState {
	let global_state = GlobalState {
		block_hash: H256(sol.globalState.bytes32Vals[0].0),
		send_root: H256(sol.globalState.bytes32Vals[1].0),
		inbox_position: sol.globalState.u64Vals[0],
		position_in_message: sol.globalState.u64Vals[1],
	};
	let machine_status = match sol.machineStatus {
		0 => MachineStatus::Running,
		1 => MachineStatus::Finished,
		_ => MachineStatus::Errored,
	};
	AfterState { global_state, machine_status }
}

/// Verify the claim's L2 block (`globalState.block_hash`) exists on the L2 quorum and that
/// its header `state_root` matches what the assertion implies. We compare on `block_hash`
/// since that is the assertion's primary commitment over L2 state. Returns `Ok(true)` if
/// the quorum agrees, `Ok(false)` if it disagrees or reports the block as missing.
async fn evaluate(target: &ArbitrumTarget, after: &AfterState) -> Result<bool, anyhow::Error> {
	if target.l2_providers.is_empty() {
		return Err(anyhow!(
			"fish_arbitrum: no l2 providers configured for {}",
			target.state_machine
		));
	}

	let claimed_block_hash = alloy::primitives::B256::from(after.global_state.block_hash.0);
	let claimed_send_root = after.global_state.send_root;

	let outcomes = join_all(target.l2_providers.iter().map(|p| async move {
		match fetch_block_by_hash(p.as_ref(), claimed_block_hash).await {
			FetchOutcome::Found(block) => {
				let l2_header: geth_primitives::CodecHeader = block.into();
				let computed_hash = Header::from(&l2_header).hash::<Hasher>();
				FetchOutcome::Found(L2View {
					hash: computed_hash,
					extra_data: H256({
						let mut buf = [0u8; 32];
						let src = &l2_header.extra_data;
						let copy = src.len().min(32);
						buf[..copy].copy_from_slice(&src[..copy]);
						buf
					}),
				})
			},
			FetchOutcome::Missing => FetchOutcome::Missing,
			FetchOutcome::Errored => FetchOutcome::Errored,
		}
	}))
	.await;

	let claimed_hash = H256(claimed_block_hash.0);
	let decision = decide(outcomes, |v: &L2View| {
		v.hash == claimed_hash && v.extra_data == claimed_send_root
	});
	Ok(match decision {
		QuorumDecision::Verified => true,
		QuorumDecision::Mismatch | QuorumDecision::MissingFromQuorum => false,
		QuorumDecision::InsufficientQuorum => {
			log::trace!(
				target: crate::LOG_TARGET,
				"fish_arbitrum: insufficient quorum for {:?}, abstaining",
				claimed_hash,
			);
			true
		},
	})
}

struct L2View {
	hash: H256,
	extra_data: H256,
}
