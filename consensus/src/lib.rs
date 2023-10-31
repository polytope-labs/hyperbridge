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

use std::time::Duration;

use anyhow::anyhow;
use futures::StreamExt;
use ismp::messaging::Message;
use tesseract_primitives::{reconnect_with_exponential_back_off, IsmpHost, IsmpProvider};
// Default wait period in seconds
const DEFAULT_WAIT_TIME: u64 = 1200;
/// Relays [`ConsensusMessage`] updates.
pub async fn relay<A, B>(
	chain_a: A,
	chain_b: B,
	use_wait_time_a: bool,
	use_wait_time_b: bool,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let task_a = tokio::spawn({
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		async move {
			handle_notification(chain_a, chain_b, use_wait_time_a).await;
		}
	});

	let task_b = tokio::spawn({
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		async move { handle_notification(chain_b, chain_a, use_wait_time_b).await }
	});
	let _ = futures::future::join_all(vec![task_a, task_b]).await;
	Ok(())
}

async fn handle_notification<A, B>(mut chain_a: A, mut chain_b: B, use_wait_time: bool)
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let mut consensus_stream = chain_a
		.consensus_notification(chain_b.clone())
		.await
		.expect("Fatal error, please restart relayer: Initial websocket connection failed");
	loop {
		let update = if use_wait_time {
			let time_inbetween_yields = tokio::time::sleep(Duration::from_secs(DEFAULT_WAIT_TIME));
			// We use a select to ensure that if the state machine stream stops yielding, we
			// forcefully restart
			tokio::select! {
				_ = time_inbetween_yields => {
					Some(Err(anyhow!("Consensus stream has stalled, restarting")))
				}
				res = consensus_stream.next() => {
					res
				}
			}
		} else {
			consensus_stream.next().await
		};
		let res = match update {
			None => Err(anyhow!("Stream Returned None")),
			Some(Ok(consensus_message)) => {
				log::info!(
					target: "tesseract",
					"🛰️ Transmitting consensus update message from {} to {}",
					chain_a.name(), chain_b.name()
				);
				let _ = chain_b.submit(vec![Message::Consensus(consensus_message)]).await;
				Ok(())
			},
			Some(Err(e)) => Err(e),
		};

		if let Err(e) = res {
			log::error!(
				target: "tesseract",
				"{} encountered an error in the consensus stream: {e}", chain_a.name()
			);
			log::info!("RESTARTING {}-{} consensus task", chain_a.name(), chain_b.name());
			// Reconnect counterparty first because counterparty is required in creating consensus
			// stream
			if let Err(_) =
				reconnect_with_exponential_back_off(&mut chain_b, &chain_a, None, None, 1000).await
			{
				panic!("Fatal Error, failed to reconnect")
			}

			if let Err(_) = reconnect_with_exponential_back_off(
				&mut chain_a,
				&chain_b,
				None,
				Some(&mut consensus_stream),
				1000,
			)
			.await
			{
				panic!("Fatal Error, failed to reconnect")
			}

			log::info!("RESTARTING {}-{} consensus task completed", chain_a.name(), chain_b.name());
		}
	}
}
