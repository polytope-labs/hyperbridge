// Copyright (C) 2023 Polytope Labs.
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
use codec::Encode;
use ethers::types::Block;
use futures::StreamExt;
use ismp::messaging::ConsensusMessage;
use jsonrpsee::{core::client::SubscriptionClientT, rpc_params};
use primitive_types::H256;
use std::time::Duration;

use crate::{notification::consensus_notification, BnbPosHost};
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider, Reconnect};

#[async_trait::async_trait]
impl IsmpHost for BnbPosHost {
	async fn consensus_notification<C>(
		&self,
		counterparty: C,
	) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
	where
		C: IsmpHost + IsmpProvider + 'static,
	{
		let client = BnbPosHost::clone(&self);
		let challenge_period =
			counterparty.query_challenge_period(self.consensus_state_id.clone()).await?;

		let sub = self
			.rpc_client
			.subscribe::<Block<H256>, _>(
				"eth_subscribe",
				rpc_params!("newHeads"),
				"eth_unsubscribe",
			)
			.await?;
		let stream = sub.filter_map(move |res| {
			let client = client.clone();
			let counterparty = counterparty.clone();
			async move {
				let last_consensus_update = counterparty
					.query_consensus_update_time(client.consensus_state_id.clone())
					.await
					.ok()?;
				let counterparty_timestamp = counterparty.query_timestamp().await.ok()?;
				let delay = counterparty_timestamp.saturating_sub(last_consensus_update);
				// If onchain timestamp has not progressed sleep
				if delay < challenge_period {
					tokio::time::sleep(delay + Duration::from_secs(12)).await;
				}
				match res {
					Ok(header) => consensus_notification(&client, counterparty, header)
						.await
						.ok()
						.flatten()
						.map(|update| {
							Ok(ConsensusMessage {
								consensus_proof: update.encode(),
								consensus_state_id: client.consensus_state_id,
							})
						}),
					_ => None,
				}
			}
		});

		Ok(Box::pin(stream))
	}
}

#[async_trait::async_trait]
impl Reconnect for BnbPosHost {
	async fn reconnect<C: IsmpProvider>(&mut self, _counterparty: &C) -> Result<(), anyhow::Error> {
		let new_host = BnbPosHost::new(&self.config).await?;
		*self = new_host;
		Ok(())
	}
}
