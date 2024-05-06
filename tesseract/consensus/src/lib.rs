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

//! Consensus message relay

use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use futures::StreamExt;
use ismp::messaging::Message;
use tesseract_primitives::{config::RelayerConfig, IsmpHost};

/// Relays [`ConsensusMessage`] updates.
pub async fn relay(
	chain_a: Arc<dyn IsmpHost>,
	chain_b: Arc<dyn IsmpHost>,
	config: RelayerConfig,
) -> Result<(), anyhow::Error> {
	let task_a = {
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		let config = config.clone();
		tokio::spawn(async move {
			let _ = handle_notification(chain_a, chain_b, config).await?;
			Ok::<_, anyhow::Error>(())
		})
	};

	let task_b = {
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		tokio::spawn(async move {
			let _ = handle_notification(chain_b, chain_a, config).await?;
			Ok::<_, anyhow::Error>(())
		})
	};

	// if one task completes, abort the other
	tokio::select! {
		result_a = task_a => {
			result_a??
		}
		result_b = task_b => {
			result_b??
		}
	}

	Ok(())
}

async fn handle_notification(
	chain_a: Arc<dyn IsmpHost>,
	chain_b: Arc<dyn IsmpHost>,
	config: RelayerConfig,
) -> Result<(), anyhow::Error> {
	let chain_a_provider = chain_a.provider();
	let chain_b_provider = chain_b.provider();
	let mut consensus_stream = chain_a
		.consensus_notification(chain_b_provider.clone())
		.await
		.map_err(|err| anyhow!("ConsensusMessage stream subscription failed: {err:?}"))?;
	loop {
		let timeout =
			tokio::time::sleep(Duration::from_secs(config.consensus_stream_timeout.unwrap_or(600)));
		tokio::select! {
			_ = timeout => {
				// If timeout elapses and consensus stream has not yielded recreate the stream
				log::trace!("Recreating consensus stream for {:?}-{:?}", chain_a_provider.state_machine_id().state_id, chain_b_provider.state_machine_id().state_id);
				consensus_stream = chain_a.consensus_notification(chain_b_provider.clone()).await.map_err(|err| anyhow!("ConsensusMessage stream subscription failed: {err:?}"))?;
			}
			item = consensus_stream.next() => {
				match item {
					Some(Ok(consensus_message)) => {
						log::info!(
							target: "tesseract",
							"🛰️ Transmitting consensus message from {} to {}",
							chain_a_provider.name(), chain_b_provider.name()
						);
						let res = chain_b_provider.submit(vec![Message::Consensus(consensus_message)]).await;
						if let Err(err) = res {
							log::error!("Failed to submit transaction to {}: {err:?}", chain_b_provider.name())
						}
					}
					Some(Err(e)) => {
						log::error!(target: "tesseract","Consensus task {}->{} encountered an error: {e:?}", chain_a_provider.name(), chain_b_provider.name())
					}

					None => break
				}
			}
		}
	}

	Err(anyhow!(
		"{}-{} consensus task has failed, Please restart relayer",
		chain_a_provider.name(),
		chain_b_provider.name()
	))?
}
