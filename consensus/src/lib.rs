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

use futures::StreamExt;
use ismp::messaging::Message;
use tesseract_primitives::{IsmpHost, IsmpProvider};
/// Relays [`ConsensusMessage`] updates.
pub async fn relay<A, B>(chain_a: A, chain_b: B) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let task_a = tokio::spawn({
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		async move {
			handle_notification(chain_a, chain_b).await;
		}
	});

	let task_b = tokio::spawn({
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		async move { handle_notification(chain_b, chain_a).await }
	});
	let _ = futures::future::join_all(vec![task_a, task_b]).await;
	Ok(())
}

async fn handle_notification<A, B>(chain_a: A, chain_b: B)
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let mut consensus_stream = chain_a
		.consensus_notification(chain_b.clone())
		.await
		.expect("Fatal error, please restart relayer: Initial websocket connection failed");
	loop {
		match consensus_stream.next().await {
			None => {
				panic!(
					"{}-{} consensus task has failed, Please restart relayer",
					chain_a.name(),
					chain_b.name()
				)
			},
			Some(Ok(consensus_message)) => {
				log::info!(
					target: "tesseract",
					"🛰️ Transmitting consensus update message from {} to {}",
					chain_a.name(), chain_b.name()
				);
				let _ = chain_b.submit(vec![Message::Consensus(consensus_message)]).await;
			},
			Some(Err(e)) => {
				log::error!(target: "tesseract","Consensus {}-{} {e:?}", chain_a.name(), chain_b.name())
			},
		};
	}
}
