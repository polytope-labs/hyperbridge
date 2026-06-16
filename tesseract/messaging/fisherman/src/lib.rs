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

//! Tesseract Fisherman
//!
//! Two flavours of fisherman live here:
//!
//! 1. The classic per-pair byzantine watcher: [`fish`] subscribes to `StateMachineUpdated`
//!    events on `chain_a` and calls `chain_b.check_for_byzantine_attack(...)` for each one.
//!    If quorum disagrees with the recorded commitment, `chain_b` vetoes via
//!    `pallet-fishermen::veto_state_commitment`.
//!
//! 2. L1 rollup-claim watchers: [`opstack::fish_opstack`] and [`arbitrum::fish_arbitrum`]
//!    poll Ethereum L1 (up to the latest block — no finality lag) for new dispute games /
//!    assertions, verify each against a 2/3·N+1 L2 RPC quorum, and submit
//!    `pallet-fishermen::blacklist_dispute_game` / `blacklist_arbitrum_claim` extrinsics
//!    for any fraudulent claim. Reacting against the latest block trades reorg-safety for
//!    timeliness: a blacklist landed against a reorged event will stick, since the on-chain
//!    blacklist has no undo path.

/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "messaging-fisherman";

pub mod arbitrum;
pub mod opstack;
pub mod quorum;

pub use arbitrum::{fish_arbitrum, ArbitrumConfig, ArbitrumKind, ArbitrumTarget};
pub use opstack::{fish_opstack, OpstackConfig, OpstackTarget};

use std::sync::Arc;

use anyhow::anyhow;
use futures::StreamExt;
use ismp::host::StateMachine;
use polkadot_sdk::sc_service::SpawnEssentialTaskHandle;
use tesseract_primitives::IsmpProvider;

pub async fn fish(
	chain_a: Arc<dyn IsmpProvider>,
	chain_b: Arc<dyn IsmpProvider>,
	spawn_handle: &SpawnEssentialTaskHandle,
	coprocessor: StateMachine,
) -> Result<(), anyhow::Error> {
	{
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		let coprocessor = coprocessor.clone();
		let name = format!("fisherman-{}-{}", chain_a.name(), chain_b.name());
		spawn_handle.spawn_blocking(
			Box::leak(Box::new(name.clone())),
			"fisherman",
			async move {
				let res = handle_notification(chain_a, chain_b, coprocessor).await;
				tracing::error!(target: LOG_TARGET, "{name} has terminated with result {res:?}")
			},
		)
	}

	Ok(())
}

async fn handle_notification(
	chain_a: Arc<dyn IsmpProvider>,
	chain_b: Arc<dyn IsmpProvider>,
	coprocessor: StateMachine,
) -> Result<(), anyhow::Error> {
	let mut state_machine_update_stream = chain_a
		.state_machine_updates(chain_b.state_machine_id())
		.await
		.map_err(|err| anyhow!("StateMachineUpdated stream subscription failed: {err:?}"))?;

	while let Some(item) = state_machine_update_stream.next().await {
		match item {
			Ok(state_machine_updates) =>
				for state_machine_update in state_machine_updates {
					let res = chain_b
						.check_for_byzantine_attack(
							coprocessor,
							chain_a.clone(),
							state_machine_update,
						)
						.await;
					if let Err(err) = res {
						log::error!(target: LOG_TARGET, "Failed to check for byzantine behavior: {err:?}")
					}
				},
			Err(e) => {
				log::error!(target: LOG_TARGET,"Fisherman task {}-{} encountered an error: {e:?}", chain_a.name(), chain_b.name())
			},
		}
	}

	Err(anyhow!(
		"{}-{} fisherman task has failed, Please restart relayer",
		chain_a.name(),
		chain_a.name()
	))?
}
