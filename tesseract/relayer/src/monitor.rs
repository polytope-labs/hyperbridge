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

//! Passive liveness monitor for the consensus update flow.
//!
//! For every `(StateMachineId, max_interval_secs)` entry in
//! `relayer.maximum_update_intervals`, the loop periodically checks two
//! freshness signals on hyperbridge:
//!
//! - **Inbound consensus** (chain → hyperbridge): time since hyperbridge last accepted a consensus
//!   update from the listed chain.
//! - **Outbound HB → substrate consensus** (hyperbridge → substrate counterparty): time since the
//!   substrate counterparty last accepted a hyperbridge consensus update. EVM destinations are
//!   skipped because their HB consensus path runs through the BEEFY proofs pipeline rather than a
//!   per-chain consensus task we can watch with these queries.
//!
//! When either signal lags by more than the configured interval, the function
//! returns `Ok(())` so the surrounding `spawn_essential_handle` shuts the
//! task manager down. An external supervisor (docker, systemd, k8s) is then
//! responsible for restarting the relayer.

use std::{collections::HashMap, sync::Arc, time::Duration};

use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
};
use tesseract_primitives::IsmpProvider;

/// How long to wait at startup before the first liveness check, so the
/// relayer's own consensus tasks have time to ship at least one update.
const STARTUP_GRACE: Duration = Duration::from_secs(600);

/// Cadence of the periodic liveness check.
const POLL_INTERVAL: Duration = Duration::from_secs(180);

pub async fn monitor_clients(
	hyperbridge: Arc<dyn IsmpProvider>,
	providers: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	configs: Vec<(StateMachineId, u64)>,
) -> anyhow::Result<()> {
	let hyperbridge_id = hyperbridge.state_machine_id();

	let check = || async {
		for (id, max_interval) in configs.iter().cloned() {
			if id == hyperbridge_id {
				continue;
			}

			// Inbound consensus side: how stale is HB's view of `id`?
			let latest_height = hyperbridge.query_latest_height(id).await?;
			let height = StateMachineHeight { id, height: latest_height.into() };
			let last_update = hyperbridge.query_state_machine_update_time(height).await?;
			let now = hyperbridge.query_timestamp().await?;
			if now.as_secs().saturating_sub(last_update.as_secs()) >= max_interval {
				tracing::error!(
					target: crate::LOG_TARGET,
					chain = %id.state_id,
					hb = %hyperbridge.name(),
					max_interval,
					"inbound consensus has stalled — restarting",
				);
				return Ok::<bool, anyhow::Error>(true);
			}

			// Outbound HB → substrate consensus side. Skip EVM destinations:
			// their HB consensus updates flow through the BEEFY proofs
			// pipeline, not a per-chain task that this monitor can watch.
			if id.state_id.is_evm() {
				continue;
			}
			let Some(provider) = providers.get(&id.state_id).cloned() else {
				tracing::warn!(
					target: crate::LOG_TARGET,
					chain = %id.state_id,
					"monitor entry references a chain not in [chains.*]; skipping outbound check",
				);
				continue;
			};
			let latest_height = provider.query_latest_height(hyperbridge_id).await?;
			let height = StateMachineHeight { id: hyperbridge_id, height: latest_height.into() };
			let last_update = provider.query_state_machine_update_time(height).await?;
			let now = provider.query_timestamp().await?;
			if now.as_secs().saturating_sub(last_update.as_secs()) >= max_interval {
				tracing::error!(
					target: crate::LOG_TARGET,
					chain = %id.state_id,
					hb = %hyperbridge.name(),
					max_interval,
					"outbound HB → substrate consensus has stalled — restarting",
				);
				return Ok(true);
			}
		}

		Ok(false)
	};

	tokio::time::sleep(STARTUP_GRACE).await;

	loop {
		match check().await {
			Ok(true) => break,
			Ok(false) => tokio::time::sleep(POLL_INTERVAL).await,
			Err(err) => {
				tracing::warn!(
					target: crate::LOG_TARGET,
					?err,
					"monitor query failed; will retry on the next tick",
				);
				tokio::time::sleep(POLL_INTERVAL).await;
			},
		}
	}

	Ok(())
}
