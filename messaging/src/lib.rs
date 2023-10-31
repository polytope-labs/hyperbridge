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

use std::time::Duration;

use crate::event_parser::{filter_events, parse_ismp_events, Event};
use anyhow::anyhow;
use futures::StreamExt;
use ismp::{consensus::StateMachineHeight, host::StateMachine};
use tesseract_primitives::{
	config::RelayerConfig, reconnect_with_exponential_back_off, BoxStream, IsmpHost, IsmpProvider,
	StateMachineUpdated,
};

// Default wait period in seconds
// This creates a enough time to compensate for cases where the consensus task restarted, increasing
// the wait time
const DEFAULT_WAIT_TIME: u64 = 1200;

pub async fn relay<A, B>(
	chain_a: A,
	chain_b: B,
	config: Option<RelayerConfig>,
	// Optional values to be used to determine acceptable wait time between state machine
	// updates
	wait_time_a: Option<Duration>,
	wait_time_b: Option<Duration>,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let router_id = config.as_ref().map(|config| config.router).flatten();
	let task_a = tokio::spawn({
		let mut chain_a = chain_a.clone();
		let mut chain_b = chain_b.clone();
		let router_id = router_id.clone();
		let mut previous_height = chain_b.initial_height();
		async move {
			let mut state_machine_update_stream = chain_a
				.state_machine_update_notification(chain_b.state_machine_id())
				.await
				.expect("Please restart the relayer, initial websocket connection failed");
			handle_notification(
				&mut chain_a,
				&mut chain_b,
				&mut state_machine_update_stream,
				router_id,
				&mut previous_height,
				wait_time_a,
			)
			.await
		}
	});

	let task_b = tokio::spawn({
		let mut chain_a = chain_a.clone();
		let mut chain_b = chain_b.clone();
		let router_id = router_id.clone();
		let mut previous_height = chain_a.initial_height();
		async move {
			let mut state_machine_update_stream = chain_b
				.state_machine_update_notification(chain_a.state_machine_id())
				.await
				.expect("Please restart the relayer, initial websocket connection failed");
			handle_notification(
				&mut chain_b,
				&mut chain_a,
				&mut state_machine_update_stream,
				router_id,
				&mut previous_height,
				wait_time_b,
			)
			.await
		}
	});
	let _ = futures::future::join_all(vec![task_a, task_b]).await;
	Ok(())
}

async fn handle_notification<A, B>(
	chain_a: &mut A,
	chain_b: &mut B,
	state_machine_update_stream: &mut BoxStream<StateMachineUpdated>,
	router_id: Option<StateMachine>,
	previous_height: &mut u64,
	wait_time: Option<Duration>,
) where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	loop {
		// Default wait time before restarting stream is 15 minutes
		let time_inbetween_yields =
			tokio::time::sleep(wait_time.unwrap_or(Duration::from_secs(DEFAULT_WAIT_TIME)));
		// We use a select to ensure that if the state machine stream stops yielding, we forcefully
		// restart
		let item = tokio::select! {
			_ = time_inbetween_yields => {
				Some(Err(anyhow!("State Machine Stream has stalled, restarting")))
			}
			res = state_machine_update_stream.next() => {
				res
			}
		};
		let res = match item {
			None => Err(anyhow::anyhow!("Stream returned None")),
			Some(Ok(state_machine_update)) =>
				handle_update(chain_a, chain_b, state_machine_update, previous_height, router_id)
					.await,
			Some(Err(e)) => Err(e),
		};
		if let Err(e) = res {
			log::error!(
				target: "tesseract",
				"{} encountered an error in the state machine update notification stream: {e}", chain_a.name()
			);
			log::info!("RESTARTING {}-{} messaging task", chain_a.name(), chain_b.name());
			if let Err(_) =
				reconnect_with_exponential_back_off(chain_b, chain_a, None, None, 1000).await
			{
				panic!("Fatal Error, failed to reconnect")
			}

			if let Err(_) = reconnect_with_exponential_back_off(
				chain_a,
				chain_b,
				Some(state_machine_update_stream),
				None,
				1000,
			)
			.await
			{
				panic!("Fatal Error, failed to reconnect")
			}

			log::info!("RESTARTING {}-{} messaging task completed", chain_a.name(), chain_b.name());
		}
	}
}

async fn handle_update<A, B>(
	chain_a: &A,
	chain_b: &B,
	state_machine_update: StateMachineUpdated,
	previous_height: &mut u64,
	router_id: Option<StateMachine>,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider,
	B: IsmpHost + IsmpProvider,
{
	// Chain B's state machine has been updated to a new height on chain A
	// We query all the events that have been emitted on chain B that can be submitted to
	// chain A filter events list to contain only Request and Response events
	let events = chain_b
		.query_ismp_events(*previous_height, state_machine_update.clone())
		.await?
		.into_iter()
		.filter(|ev| filter_events(router_id, chain_a.state_machine_id().state_id, ev))
		.collect::<Vec<_>>();

	if events.is_empty() {
		*previous_height = state_machine_update.latest_height;
		return Ok(())
	}

	let log_events = events.clone().into_iter().map(Into::into).collect::<Vec<Event>>();
	log::info!(
	   target: "tesseract",
	   "Events from {} {:#?}", chain_b.name(),
	   log_events // event names
	);
	let state_machine_height = StateMachineHeight {
		id: state_machine_update.state_machine_id,
		height: state_machine_update.latest_height,
	};
	let (messages, get_responses) =
		parse_ismp_events(chain_b, chain_a, events, state_machine_height).await?;

	if !messages.is_empty() {
		// Sleep for 30 seconds before submitting messages
		tokio::time::sleep(Duration::from_secs(12)).await;
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
		// Sleep for 30 seconds before submitting messages
		tokio::time::sleep(Duration::from_secs(12)).await;
		log::info!(
			target: "tesseract",
			"🛰️Submitting GET response messages to {}",
			chain_b.name()
		);
		let _ = chain_b.submit(get_responses).await;
	}
	*previous_height = state_machine_update.latest_height;
	Ok(())
}
