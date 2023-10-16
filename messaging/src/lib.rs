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

//! ISMP Message relay

mod event_parser;

use crate::event_parser::{filter_events, parse_ismp_events, Event};
use futures::StreamExt;
use ismp::consensus::StateMachineHeight;
use tesseract_primitives::{
	config::RelayerConfig, reconnect_with_exponential_back_off, IsmpHost, IsmpProvider,
};

pub async fn relay<A, B>(
	mut chain_a: A,
	mut chain_b: B,
	config: Option<RelayerConfig>,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider,
	B: IsmpHost + IsmpProvider,
{
	let mut state_machine_update_stream_a =
		chain_a.state_machine_update_notification(chain_b.state_machine_id()).await;
	let mut state_machine_update_stream_b =
		chain_b.state_machine_update_notification(chain_a.state_machine_id()).await;
	let router_id = config.as_ref().map(|config| config.router).flatten();

	loop {
		tokio::select! {
			result = state_machine_update_stream_a.next() =>  {
				match result {
					None => {
						log::info!("RESTARTING {}-{} messaging task", chain_a.name(), chain_b.name());
						reconnect_with_exponential_back_off(&mut chain_a, &chain_b, 1000).await?;
						reconnect_with_exponential_back_off(&mut chain_b, &chain_a, 1000).await?;
						state_machine_update_stream_a = chain_a.state_machine_update_notification(chain_b.state_machine_id()).await;
						state_machine_update_stream_b = chain_b.state_machine_update_notification(chain_a.state_machine_id()).await;
						log::info!("RESTARTING completed");
						continue
					},
					Some(Ok(state_machine_update)) => {
						log::info!("{} updated: {state_machine_update:?}", chain_a.name());
						// Chain B's state machine has been updated to a new height on chain A
						// We query all the events that have been emitted on chain B that can be submitted to chain A
						// filter events list to contain only Request and Response events
						let events = chain_b.query_ismp_events(state_machine_update.clone()).await?.into_iter()
							.filter(|ev| filter_events(router_id, chain_a.state_machine_id().state_id, ev)).collect::<Vec<_>>();

						if events.is_empty() {
							continue
						}

						let log_events = events.clone().into_iter().map(Into::into).collect::<Vec<Event>>();
						log::info!(
							target: "tesseract",
							"Events from {} {:#?}", chain_b.name(),
							log_events // event names
						 );
						let state_machine_height = StateMachineHeight {
							id: state_machine_update.state_machine_id,
							height: state_machine_update.latest_height
						};
						let (messages, get_responses) = parse_ismp_events(&chain_b, &chain_a, events, state_machine_height).await?;

						if !messages.is_empty() {
							log::info!(
								target: "tesseract",
								"🛰️Submitting ismp messages from {} to {}",
								chain_b.name(), chain_a.name()
							);
							if let Err(err) = chain_a.submit(messages).await {
								log::error!("Failed to submit transaction to {}: {err:?}", chain_a.name())
							}
						}

						if !get_responses.is_empty() {
							log::info!(
								target: "tesseract",
								"🛰️Submitting GET response messages to {}",
								chain_b.name()
							);
							let _ = chain_b.submit(get_responses).await;
						}
					},
					Some(Err(e)) => {
						log::error!(
							target: "tesseract",
							"{} encountered an error in the state machine update notification stream: {e}", chain_a.name()
						)
					}
				}
			}

			result = state_machine_update_stream_b.next() =>  {
				 match result {
					None => {
						log::info!("RESTARTING {}-{} messaging task", chain_a.name(), chain_b.name());
						reconnect_with_exponential_back_off(&mut chain_b, &chain_a, 1000).await?;
						reconnect_with_exponential_back_off(&mut chain_a, &chain_b, 1000).await?;
						state_machine_update_stream_b = chain_b.state_machine_update_notification(chain_a.state_machine_id()).await;
						state_machine_update_stream_a = chain_a.state_machine_update_notification(chain_b.state_machine_id()).await;
						log::info!("RESTARTING completed");
						continue;
					},
					Some(Ok(state_machine_update)) => {
						log::info!("{} updated: {state_machine_update:?}", chain_b.name());
						// Chain A's state machine has been updated to a new height on chain B
						// We query all the events that have been emitted on chain A that can be submitted to chain B

						// filter events list to contain only Request and Response events
						let events = chain_a.query_ismp_events(state_machine_update.clone()).await?.into_iter()
							.filter(|ev| filter_events(router_id, chain_b.state_machine_id().state_id, ev)).collect::<Vec<_>>();

						if events.is_empty() {
							continue
						}

						let log_events = events.clone().into_iter().map(Into::into).collect::<Vec<Event>>();
						log::info!(
							target: "tesseract",
							"Events from {} {:#?}", chain_a.name(),
							log_events
						 );
						let state_machine_height = StateMachineHeight {
							id: state_machine_update.state_machine_id,
							height: state_machine_update.latest_height
						};
						let (messages, get_responses) = parse_ismp_events(&chain_a, &chain_b, events, state_machine_height).await?;
						if !messages.is_empty() {
							log::info!(
								target: "tesseract",
								"🛰️Submitting ismp messages from {} to {}",
								chain_a.name(), chain_b.name()
							);
							if let Err(err) = chain_b.submit(messages).await {
								log::error!("Failed to submit transaction to {}: {err:?}", chain_b.name())
							}
						}

						if !get_responses.is_empty() {
							log::info!(
								target: "tesseract",
								"🛰️Submitting GET response messages to {}",
								chain_a.name()
							);
							let _ = chain_a.submit(get_responses).await;
						}
					},
					Some(Err(e)) => {
						log::error!(
							target: "tesseract",
							"{} encountered an error in the state machine update notification stream: {e}", chain_b.name()
						)
					}
				}
			}
		}
	}
}
