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
use anyhow::anyhow;
use codec::Encode;
use futures::StreamExt;
use ismp::messaging::ConsensusMessage;
use primitives::consensus_types::Checkpoint;
use reqwest_eventsource::EventSource;

use crate::notification::{consensus_notification, EventResponse};
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider};

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
        let node_url =
            format!("{}/eth/v1/events?topics=finalized_checkpoint", client.beacon_node_rpc);
        let (sender, receiver) =
            tokio::sync::mpsc::unbounded_channel::<Result<ConsensusMessage, anyhow::Error>>();
        let mut es = EventSource::get(node_url);
        tokio::spawn({
            let client = client.clone();
            let counterparty = counterparty.clone();
            async move {
                while let Some(event) = es.next().await {
                    match event {
                        Ok(reqwest_eventsource::Event::Message(msg)) => {
                            if let Ok(message) = serde_json::from_str::<EventResponse>(&msg.data) {
                                let checkpoint = Checkpoint {
                                    epoch: message
                                        .epoch
                                        .parse()
                                        .expect("Epoch is always available"),
                                    root: message.block,
                                };

                                let last_consensus_update = if let Ok(last_consensus_update) =
                                    counterparty
                                        .query_consensus_update_time(
                                            client.consensus_state_id.clone(),
                                        )
                                        .await
                                {
                                    last_consensus_update
                                } else {
                                    sender.send(Err(anyhow!("Failed to fetch consensus update time from counterparty, skipping update")))
                                        .expect("Receiver has been dropped");
                                    continue
                                };
                                let counterparty_timestamp = if let Ok(counterparty_timestamp) =
                                    counterparty.query_timestamp().await
                                {
                                    counterparty_timestamp
                                } else {
                                    sender.send(Err(anyhow!("Failed to fetch consensus update time from counterparty, skipping update")))
                                        .expect("Receiver has been dropped");
                                    continue
                                };
                                if counterparty_timestamp - last_consensus_update < challenge_period
                                {
                                    continue
                                }

                                consensus_notification(&client, counterparty.clone(), checkpoint)
                                    .await
                                    .ok()
                                    .flatten()
                                    .into_iter()
                                    .for_each(|beacon_message| {
                                        let message = ConsensusMessage {
                                            consensus_proof: beacon_message.encode(),
                                            consensus_state_id: client.consensus_state_id,
                                        };
                                        sender
                                            .send(Ok(message))
                                            .expect("Receiver has been dropped");
                                    });
                            }
                        }
                        Err(err) => {
                            println!("Encountered Error and closed stream {err:?}");
                            break
                        }
                        _ => continue,
                    }
                }
            }
        });

        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod test {
    use futures::StreamExt;
    use reqwest_eventsource::{Event, EventSource};

    #[tokio::test]
    #[ignore]
    async fn test_event_subscription() {
        let node_url = "http://localhost:3500/eth/v1/events?topics=finalized_checkpoint";
        let es = EventSource::get(node_url);
        for event in es.take(2).collect::<Vec<_>>().await {
            match event {
                Ok(Event::Open) => println!("Connection Open!"),
                Ok(Event::Message(message)) => println!("Message: {:#?}", message),
                Err(err) => {
                    println!("Error: {}", err);
                    break
                }
            }
        }
    }
}
