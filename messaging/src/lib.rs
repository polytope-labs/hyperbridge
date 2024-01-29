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

use std::sync::Arc;

use crate::event_parser::{filter_events, parse_ismp_events, Event};
use futures::StreamExt;
use ismp::{consensus::StateMachineHeight, host::StateMachine};
use tesseract_primitives::{
	config::RelayerConfig, wait_for_challenge_period, BoxStream, IsmpHost, IsmpProvider,
	StateMachineUpdated,
};
use transaction_payment::TransactionPayment;

pub async fn relay<A, B>(
	chain_a: A,
	chain_b: B,
	config: Option<RelayerConfig>,
	tx_payment: Arc<TransactionPayment>,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let router_id = config.as_ref().map(|config| config.router).flatten();
	let task_a = tokio::spawn({
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		let tx_payment = tx_payment.clone();
		let router_id = router_id.clone();
		let mut previous_height = get_previous_height(&chain_a, &chain_b, &tx_payment).await;
		async move {
			let mut state_machine_update_stream = chain_a
				.state_machine_update_notification(chain_b.state_machine_id())
				.await
				.expect("Please restart the relayer, initial websocket connection failed");
			handle_notification(
				&chain_a,
				&chain_b,
				tx_payment,
				&mut state_machine_update_stream,
				router_id,
				&mut previous_height,
			)
			.await
		}
	});

	let task_b = tokio::spawn({
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		let tx_payment = tx_payment.clone();
		let router_id = router_id.clone();
		let mut previous_height = get_previous_height(&chain_b, &chain_a, &tx_payment).await;
		async move {
			let mut state_machine_update_stream = chain_b
				.state_machine_update_notification(chain_a.state_machine_id())
				.await
				.expect("Please restart the relayer, initial websocket connection failed");
			handle_notification(
				&chain_b,
				&chain_a,
				tx_payment,
				&mut state_machine_update_stream,
				router_id,
				&mut previous_height,
			)
			.await
		}
	});
	let _ = futures::future::join_all(vec![task_a, task_b]).await;
	Ok(())
}

async fn handle_notification<A, B>(
	chain_a: &A,
	chain_b: &B,
	tx_payment: Arc<TransactionPayment>,
	state_machine_update_stream: &mut BoxStream<StateMachineUpdated>,
	router_id: Option<StateMachine>,
	previous_height: &mut u64,
) where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	loop {
		match state_machine_update_stream.next().await {
			None => {
				panic!(
					"{}-{} messaging task has failed, Please restart relayer",
					chain_a.name(),
					chain_b.name()
				)
			},
			Some(Ok(state_machine_update)) => {
				let _ = handle_update(
					chain_a,
					chain_b,
					&tx_payment,
					state_machine_update,
					previous_height,
					router_id,
				)
				.await;
			},
			Some(Err(e)) => {
				log::error!(target: "tesseract","Messaging {}-{} {e:?}", chain_a.name(), chain_b.name());
			},
		};
	}
}

async fn handle_update<A, B>(
	chain_a: &A,
	chain_b: &B,
	tx_payment: &Arc<TransactionPayment>,
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
	let messages = parse_ismp_events(chain_b, chain_a, events, state_machine_height).await?;

	if !messages.is_empty() {
		log::info!(
			target: "tesseract",
			"🛰️Submitting ismp messages from {} to {}",
			chain_b.name(), chain_a.name()
		);
		let last_consensus_update = chain_a
			.query_consensus_update_time(chain_b.state_machine_id().consensus_state_id)
			.await?;
		let challenge_period = chain_a
			.query_challenge_period(chain_b.state_machine_id().consensus_state_id)
			.await?;
		// Wait for the challenge period for the consensus update to elapse before submitting
		// messages
		wait_for_challenge_period(chain_a, last_consensus_update, challenge_period).await?;
		if let Err(err) = chain_a.submit(messages.clone()).await {
			log::error!("Failed to submit transaction to {}: {err:?}", chain_a.name())
		} else {
			tx_payment.store_messages(messages).await?;
		}
	}

	*previous_height = state_machine_update.latest_height;
	tx_payment
		.store_latest_height(
			chain_b.state_machine_id().state_id,
			chain_a.state_machine_id().state_id,
			state_machine_update.latest_height,
		)
		.await?;
	Ok(())
}

async fn get_previous_height<A: IsmpProvider, B: IsmpProvider>(
	source: &A,
	sink: &B,
	tx_payment: &Arc<TransactionPayment>,
) -> u64 {
	let retrieve_latest_height = tx_payment
		.retreive_latest_height(
			sink.state_machine_id().state_id,
			source.state_machine_id().state_id,
		)
		.await;
	if retrieve_latest_height.is_err() {
		log::error!(
			"Error retrieving last relayed height from database; resuming from latest height"
		)
	}
	if let Some(previous_height) = retrieve_latest_height.ok().flatten() {
		// Try to query events at stored height to see if it exists,
		// if the query returns an error we resume relaying from the latest height
		let res = sink
			.query_ismp_events(
				previous_height,
				StateMachineUpdated {
					state_machine_id: sink.state_machine_id(),
					latest_height: previous_height + 1,
				},
			)
			.await;
		if res.is_ok() {
			previous_height
		} else {
			sink.initial_height()
		}
	} else {
		sink.initial_height()
	}
}
