// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Error;
use std::collections::BTreeMap;
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
use ismp::messaging::{ConsensusMessage, CreateConsensusState};
use primitive_types::H160;
use sync_committee_primitives::constants::Config;

use crate::notification::consensus_notification;
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider, Reconnect};

// todo: Figure out the issue with the stream
#[cfg(feature = "finality-events")]
#[async_trait::async_trait]
impl<T: Config + Send + Sync> IsmpHost for SyncCommitteeHost<T> {
	async fn consensus_notification<C>(
		&self,
		counterparty: C,
	) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
	where
		C: IsmpHost + IsmpProvider + 'static,
	{
		use eventsource_client::Client;
		use sync_committee_primitives::consensus_types::Checkpoint;
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

	async fn get_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
		let mut ismp_contract_addresses = BTreeMap::new();
		let mut l2_oracle = BTreeMap::new();
		let mut rollup_core = H160::default();
		if let Some(host) = &self.arbitrum_client {
			ismp_contract_addresses.insert(host.evm.state_machine, host.evm.ismp_host);
			rollup_core = host.host.rollup_core;
		}

		if let Some(host) = &self.optimism_client {
			ismp_contract_addresses.insert(host.evm.state_machine, host.evm.ismp_host);
			l2_oracle.insert(host.evm.state_machine, host.host.l2_oracle);
		}

		if let Some(host) = &self.base_client {
			ismp_contract_addresses.insert(host.evm.state_machine, host.evm.ismp_host);
			l2_oracle.insert(host.evm.state_machine, host.host.l2_oracle);
		}
		ismp_contract_addresses.insert(self.state_machine, self.evm.ismp_host);
		let initial_consensus_state = self
			.get_consensus_state(ismp_contract_addresses, l2_oracle, rollup_core, None)
			.await?;
		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: consensus_client::BEACON_CONSENSUS_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_period: 5 * 60,
			state_machine_commitments: vec![],
		}))
	}
}

#[cfg(not(feature = "finality-events"))]
#[async_trait::async_trait]
impl<T: Config + Send + Sync + 'static> IsmpHost for SyncCommitteeHost<T> {
	async fn consensus_notification<C>(
		&self,
		counterparty: C,
	) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
	where
		C: IsmpHost + IsmpProvider + 'static,
	{
		let client = SyncCommitteeHost::clone(&self);

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

	async fn get_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
		let mut ismp_contract_addresses = BTreeMap::new();
		let mut l2_oracle = BTreeMap::new();
		let mut rollup_core = H160::default();
		if let Some(host) = &self.arbitrum_client {
			ismp_contract_addresses.insert(host.evm.state_machine, host.evm.ismp_host);
			rollup_core = host.host.rollup_core;
		}

		if let Some(host) = &self.optimism_client {
			ismp_contract_addresses.insert(host.evm.state_machine, host.evm.ismp_host);
			l2_oracle.insert(host.evm.state_machine, host.host.l2_oracle);
		}

		if let Some(host) = &self.base_client {
			ismp_contract_addresses.insert(host.evm.state_machine, host.evm.ismp_host);
			l2_oracle.insert(host.evm.state_machine, host.host.l2_oracle);
		}
		ismp_contract_addresses.insert(self.state_machine, self.evm.ismp_host);
		let initial_consensus_state = self
			.get_consensus_state(ismp_contract_addresses, l2_oracle, rollup_core, None)
			.await?;
		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: ismp_sync_committee::BEACON_CONSENSUS_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_period: 5 * 60,
			state_machine_commitments: vec![],
		}))
	}
}

#[async_trait::async_trait]
impl<T: Config + Send + Sync + 'static> Reconnect for SyncCommitteeHost<T> {
	async fn reconnect(&mut self) -> Result<(), anyhow::Error> {
		if let Some(arb_client) = self.arbitrum_client.as_mut() {
			arb_client.reconnect().await?;
		}
		if let Some(base_client) = self.base_client.as_mut() {
			base_client.reconnect().await?;
		}

		if let Some(op_client) = self.optimism_client.as_mut() {
			op_client.reconnect().await?;
		}
		Ok(())
	}
}
