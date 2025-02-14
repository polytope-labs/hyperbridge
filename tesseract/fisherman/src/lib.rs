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

use std::sync::Arc;

use anyhow::anyhow;
use futures::StreamExt;
use ismp::host::StateMachine;
use polkadot_sdk::sc_service::TaskManager;
use tesseract_primitives::IsmpProvider;

pub async fn fish(
	chain_a: Arc<dyn IsmpProvider>,
	chain_b: Arc<dyn IsmpProvider>,
	task_manager: &TaskManager,
	coprocessor: StateMachine,
) -> Result<(), anyhow::Error> {
	{
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		let coprocessor = coprocessor.clone();
		let name = format!("fisherman-{}-{}", chain_a.name(), chain_b.name());
		task_manager.spawn_essential_handle().spawn_blocking(
			Box::leak(Box::new(name.clone())),
			"fisherman",
			async move {
				let res = handle_notification(chain_a, chain_b, coprocessor).await;
				tracing::error!(target: "tesseract", "{name} has terminated with result {res:?}")
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
						log::error!("Failed to check for byzantine behavior: {err:?}")
					}
				},
			Err(e) => {
				log::error!(target: "tesseract","Fisherman task {}-{} encountered an error: {e:?}", chain_a.name(), chain_b.name())
			},
		}
	}

	Err(anyhow!(
		"{}-{} fisherman task has failed, Please restart relayer",
		chain_a.name(),
		chain_a.name()
	))?
}
