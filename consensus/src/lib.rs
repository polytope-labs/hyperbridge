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

use anyhow::anyhow;
use futures::StreamExt;
use ismp::messaging::{ConsensusMessage, Message};
use tesseract_primitives::{reconnect_with_exponential_back_off, IsmpHost, IsmpProvider};
/// Relays [`ConsensusMessage`] updates.
pub async fn relay<A, B>(chain_a: A, chain_b: B) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let task_a = tokio::spawn({
		let mut chain_a = chain_a.clone();
		let mut chain_b = chain_b.clone();
		async move {
			let mut consensus_stream = chain_a
				.consensus_notification(chain_b.clone())
				.await
				.expect("Failed to create consensus stream");
			loop {
				let item = consensus_stream.next().await;
				let res = handle_notification(&mut chain_a, &mut chain_b, item).await;

				if let Err(_) = res {
					log::info!("RESTARTING {} consensus task", chain_a.name());
					if let Err(_) =
						reconnect_with_exponential_back_off(&mut chain_a, &mut chain_b, 1000).await
					{
						panic!("Fatal Error, failed to reconnect")
					}
					if let Err(_) =
						reconnect_with_exponential_back_off(&mut chain_b, &mut chain_a, 1000).await
					{
						panic!("Fatal Error, failed to reconnect")
					}
					consensus_stream = chain_a
						.consensus_notification(chain_b.clone())
						.await
						.expect("Failed to create consensus stream");
					log::info!("RESTARTING completed");
				}
			}
		}
	});

	let task_b = tokio::spawn({
		let mut chain_a = chain_a.clone();
		let mut chain_b = chain_b.clone();
		async move {
			let mut consensus_stream = chain_b
				.consensus_notification(chain_a.clone())
				.await
				.expect("Failed to create consensus stream");
			loop {
				let item = consensus_stream.next().await;
				let res = handle_notification(&chain_b, &chain_a, item).await;
				if let Err(_) = res {
					log::info!("RESTARTING {} consensus task", chain_b.name());
					if let Err(_) =
						reconnect_with_exponential_back_off(&mut chain_a, &mut chain_b, 1000).await
					{
						panic!("Fatal Error, failed to reconnect")
					}
					if let Err(_) =
						reconnect_with_exponential_back_off(&mut chain_b, &mut chain_a, 1000).await
					{
						panic!("Fatal Error, failed to reconnect")
					}
					consensus_stream = chain_b
						.consensus_notification(chain_a.clone())
						.await
						.expect("Failed to create consensus stream");
					log::info!("RESTARTING completed");
				}
			}
		}
	});
	let _ = futures::future::join_all(vec![task_a, task_b]).await;
	Ok(())
}

async fn handle_notification<A, B>(
	chain_a: &A,
	chain_b: &B,
	update: Option<Result<ConsensusMessage, anyhow::Error>>,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider,
	B: IsmpHost + IsmpProvider,
{
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
		Some(Err(e)) => {
			log::error!(
				target: "tesseract",
				"{} encountered an error in the consensus stream: {e}", chain_a.name()
			);
			Err(e)
		},
	};
	res
}
