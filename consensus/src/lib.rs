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
use ismp::messaging::Message;
use tesseract_primitives::{IsmpHost, IsmpProvider};

/// Relays [`ConsensusMessage`] updates.
pub async fn relay<A, B>(chain_a: A, chain_b: B) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let task_a = {
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		Box::pin(handle_notification(chain_a, chain_b))
	};

	let task_b = {
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		Box::pin(handle_notification(chain_b, chain_a))
	};

	// if one task completes, abort the other
	tokio::select! {
		result_a = task_a => {
			result_a?
		}
		result_b = task_b => {
			result_b?
		}
	};

	Ok(())
}

async fn handle_notification<A, B>(chain_a: A, chain_b: B) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let mut consensus_stream = chain_a
		.consensus_notification(chain_b.clone())
		.await
		.map_err(|err| anyhow!("ConsensusMessage stream subscription failed: {err:?}"))?;

	while let Some(item) = consensus_stream.next().await {
		match item {
			Ok(consensus_message) => {
				log::info!(
					target: "tesseract",
					"🛰️ Transmitting consensus message from {} to {}",
					chain_a.name(), chain_b.name()
				);
				if let Err(err) = chain_b.submit(vec![Message::Consensus(consensus_message)]).await
				{
					log::error!("Failed to submit transaction to {}: {err:?}", chain_b.name())
				}
			},
			Err(e) => {
				log::error!(target: "tesseract","Consensus task {}->{} encountered an error: {e:?}", chain_a.name(), chain_b.name())
			},
		}
	}

	Err(anyhow!(
		"{}-{} consensus task has failed, Please restart relayer",
		chain_a.name(),
		chain_b.name()
	))?
}
