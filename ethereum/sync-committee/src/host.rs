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
use crate::SyncCommitteeHost;
use codec::Encode;
use ismp::messaging::ConsensusMessage;

use crate::notification::consensus_notification;
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider};

// todo: Figure out the issue with the stream
#[cfg(feature = "finality-events")]
#[async_trait::async_trait]
impl IsmpHost for SyncCommitteeHost {
	async fn consensus_notification<C>(
		&self,
		counterparty: C,
	) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
	where
		C: IsmpHost + IsmpProvider + 'static,
	{
		use eventsource_client::Client;
		use primitives::consensus_types::Checkpoint;
		let client = SyncCommitteeHost::clone(&self);
		let challenge_period = counterparty.query_challenge_period(self.consensus_state_id).await?;
		let node_url =
			format!("{}/eth/v1/events?topics=finalized_checkpoint", client.beacon_node_rpc);
		let ev_source = std::sync::Arc::new(
			eventsource_client::ClientBuilder::for_url(&node_url)
				.expect("Failed to create stream")
				.build(),
		);

		let stream = ev_source.stream().filter_map(move |event| {
			let counterparty = counterparty.clone();
			let client = client.clone();
			async move {
				let last_consensus_update = counterparty
					.query_consensus_update_time(client.consensus_state_id)
					.await
					.ok()
					.unwrap_or_else(|| {
						log::error!(
							"Failed to fetch last consensus update time: from {}",
							counterparty.name()
						);
						Default::default()
					});
				match event {
					Ok(eventsource_client::SSE::Event(ev)) => {
						if let Ok(message) = serde_json::from_str::<EventResponse>(&ev.data) {
							tokio::time::sleep(std::time::Duration::from_secs(150)).await;
							let checkpoint = Checkpoint {
								epoch: message.epoch.parse().expect("Epoch is always available"),
								root: message.block,
							};
							let counterparty_timestamp =
								counterparty.query_timestamp().await.ok().unwrap_or_else(|| {
									log::error!(
										"Failed to fetch consensus update time from: {}",
										counterparty.name(),
									);
									Default::default()
								});

							if counterparty_timestamp - last_consensus_update < challenge_period {
								return None
							}

							if let Ok(Some(beacon_message)) =
								consensus_notification(&client, counterparty.clone(), checkpoint)
									.await
							{
								Some(Ok(ConsensusMessage {
									consensus_proof: beacon_message.encode(),
									consensus_state_id: client.consensus_state_id,
								}))
							} else {
								None
							}
						} else {
							None
						}
					},
					Err(err) => {
						log::error!("SyncCommittee: Consensus stream encountered error {err:?}");
						None
					},
					_ => return None,
				}
			}
		});

		Ok(Box::pin(stream))
	}
}

#[cfg(not(feature = "finality-events"))]
#[async_trait::async_trait]
impl IsmpHost for SyncCommitteeHost {
	async fn consensus_notification<C>(
		&self,
		counterparty: C,
	) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
	where
		C: IsmpHost + IsmpProvider + 'static,
	{
		let client = SyncCommitteeHost::clone(&self);
		let challenge_period =
			counterparty.query_challenge_period(self.consensus_state_id.clone()).await?;

		let interval = tokio::time::interval(self.consensus_update_frequency);

		let interval_stream = futures::stream::try_unfold(interval, move |mut interval| {
			let client = client.clone();
			let counterparty = counterparty.clone();

			async move {
				loop {
					// tick the interval
					interval.tick().await;
					let checkpoint =
						client.prover.fetch_finalized_checkpoint(Some("head")).await?.finalized;
					let last_consensus_update = counterparty
						.query_consensus_update_time(client.consensus_state_id.clone())
						.await?;
					let counterparty_timestamp = counterparty.query_timestamp().await?;
					let delay = counterparty_timestamp - last_consensus_update;
					// If onchain timestamp has not progressed sleep
					if delay < challenge_period {
						tokio::time::sleep(delay).await;
					}

					let update = consensus_notification(&client, counterparty.clone(), checkpoint)
						.await?
						.map(|beacon_message| ConsensusMessage {
							consensus_proof: beacon_message.encode(),
							consensus_state_id: client.consensus_state_id,
						});
					if let Some(update) = update {
						return Ok::<_, anyhow::Error>(Some((update, interval)))
					} else {
						// We continue the loop
						continue
					}
				}
			}
		});

		Ok(Box::pin(interval_stream))
	}
}
