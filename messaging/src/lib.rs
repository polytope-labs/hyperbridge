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

mod events;

use anyhow::anyhow;
use std::{collections::HashMap, sync::Arc};

use crate::events::{filter_events, translate_events_to_messages, Event};
use futures::StreamExt;
use ismp::{consensus::StateMachineHeight, host::StateMachine};
use tesseract_client::AnyClient;
use tracing::instrument;

use tesseract_primitives::{
	config::{Chain, RelayerConfig},
	wait_for_challenge_period, IsmpHost, IsmpProvider, StateMachineUpdated,
};
use transaction_fees::TransactionPayment;

pub async fn relay<A, B>(
	chain_a: A,
	chain_b: B,
	config: RelayerConfig,
	coprocessor: Chain,
	tx_payment: Arc<TransactionPayment>,
	client_map: HashMap<StateMachine, AnyClient>,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let task_a = {
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		let client_map = client_map.clone();
		let tx_payment = tx_payment.clone();
		let config = config.clone();
		tokio::spawn(async move {
			let _ =
				handle_notification(chain_a, chain_b, tx_payment, config, coprocessor, client_map)
					.await?;
			Ok::<_, anyhow::Error>(())
		})
	};

	let task_b = {
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		let tx_payment = tx_payment.clone();
		tokio::spawn(async move {
			let _ =
				handle_notification(chain_b, chain_a, tx_payment, config, coprocessor, client_map)
					.await?;
			Ok::<_, anyhow::Error>(())
		})
	};

	// if one task completes, abort the other
	tokio::select! {
		result_a = task_a => {
			result_a??
		}
		result_b = task_b => {
			result_b??
		}
	};

	Ok(())
}

async fn handle_notification<A, B>(
	chain_a: A,
	chain_b: B,
	tx_payment: Arc<TransactionPayment>,
	config: RelayerConfig,
	coprocessor: Chain,
	client_map: HashMap<StateMachine, AnyClient>,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let mut state_machine_update_stream = chain_a
		.state_machine_update_notification(chain_b.state_machine_id())
		.await
		.map_err(|err| anyhow!("StateMachineUpdated stream subscription failed: {err:?}"))?
		// skipping the first event, because it yields the most recent event
		// but we've already initialized our heights to that event.
		// don't remove
		.skip(1);
	let mut previous_height = chain_b.initial_height();

	while let Some(item) = state_machine_update_stream.next().await {
		match item {
			Ok(state_machine_update) => {
				if let Err(err) = handle_update(
					&chain_a,
					&chain_b,
					&tx_payment,
					state_machine_update.clone(),
					&mut previous_height,
					config.clone(),
					coprocessor,
					&client_map,
				)
				.await
				{
					log::error!(
						"Error while handling {:?} on {:?}: {err:?}",
						state_machine_update.state_machine_id.state_id,
						chain_a.state_machine_id().state_id
					);
				}
			},
			Err(e) => {
				log::error!(target: "tesseract","Messaging task {}->{} encountered an error: {e:?}", chain_a.name(), chain_b.name());
				continue;
			},
		}
	}

	Err(anyhow!(
		"{}-{} messaging task has failed, Please restart relayer",
		chain_a.name(),
		chain_b.name()
	))?
}

#[instrument(skip_all, fields(chain_a = chain_a.state_machine_id().state_id.to_string(), chain_b = chain_b.state_machine_id().state_id.to_string()))]
async fn handle_update<A, B>(
	chain_a: &A,
	chain_b: &B,
	tx_payment: &Arc<TransactionPayment>,
	state_machine_update: StateMachineUpdated,
	previous_height: &mut u64,
	config: RelayerConfig,
	coprocessor: Chain,
	client_map: &HashMap<StateMachine, AnyClient>,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider,
	B: IsmpHost + IsmpProvider,
{
	// Chain B's state machine has been updated to a new height on chain A
	// We query all the events that have been emitted on chain B that can be submitted to
	// chain A filter events list to contain only Request and Response events
	let result = chain_b.query_ismp_events(*previous_height, state_machine_update.clone()).await;

	let events = match result {
		Ok(events) => events
			.into_iter()
			.filter(|ev| {
				filter_events(coprocessor.state_machine(), chain_a.state_machine_id().state_id, ev)
			})
			.collect::<Vec<_>>(),
		Err(err) => {
			log::error!(
				"Encountered an error querying events from {:?}: {err:?}",
				chain_b.state_machine_id().state_id
			);
			Default::default()
		},
	};

	let state_machine = state_machine_update.state_machine_id.state_id;
	if events.is_empty() {
		log::info!(
			"Skipping latest finalized height {} on {}, no new messages from {state_machine:?} in range {:?}",
			state_machine_update.latest_height,
			chain_a.name(),
			*previous_height..=state_machine_update.latest_height
		);
		*previous_height = state_machine_update.latest_height;
		return Ok(())
	}
	// Advance latest known height by relayer
	*previous_height = state_machine_update.latest_height;
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

	let last_consensus_update =
		chain_a.query_state_machine_update_time(state_machine_height).await?;
	let challenge_period = chain_a
		.query_challenge_period(chain_b.state_machine_id().consensus_state_id)
		.await?;
	// Wait for the challenge period for the consensus update to elapse before submitting
	// messages. This is so that calls to debug_traceCall can succeed
	wait_for_challenge_period(chain_a, last_consensus_update, challenge_period).await?;

	let messages = translate_events_to_messages(
		chain_b,
		chain_a,
		events,
		state_machine_height.clone(),
		config.clone(),
		coprocessor,
		&client_map,
	)
	.await?;

	if !messages.is_empty() {
		log::info!(
			target: "tesseract",
			"🛰️ Transmitting ismp messages from {} to {}",
			chain_b.name(), chain_a.name()
		);

		let res = chain_a.submit(messages.clone()).await;
		match res {
			Ok(receipts) => {
				// We should not store messages when they are delivered to hyperbridge
				if chain_a.state_machine_id().state_id != coprocessor.state_machine() {
					if !receipts.is_empty() {
						log::info!(target: "tesseract", "Storing {} deliveries from {} to {} inside the database",receipts.len(), chain_b.name(), chain_a.name());
						if let Err(err) = tx_payment.store_messages(receipts).await {
							log::error!(
								"Failed to store some delivered messages to database: {err:?}"
							)
						}
					}
				}
			},
			Err(err) => log::error!("Failed to submit transaction to {state_machine}: {err:?}"),
		}
	}

	Ok(())
}
