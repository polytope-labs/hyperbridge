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
mod retries;

use anyhow::anyhow;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
	events::{filter_events, translate_events_to_messages, Event},
	retries::spawn_unprofitable_retries_task,
};
use futures::StreamExt;
use ismp::{consensus::StateMachineHeight, host::StateMachine};
use tesseract_client::AnyClient;

use tesseract_primitives::{
	config::{Chain, RelayerConfig},
	observe_challenge_period, wait_for_challenge_period, wait_for_state_machine_update,
	HyperbridgeClaim, IsmpHost, IsmpProvider, StateMachineUpdated, TxReceipt,
};
use transaction_fees::TransactionPayment;

type FeeAccSender = Sender<Vec<TxReceipt>>;
pub async fn relay<A, B>(
	chain_a: A,
	chain_b: B,
	config: RelayerConfig,
	coprocessor: Chain,
	tx_payment: Arc<TransactionPayment>,
	client_map: HashMap<StateMachine, AnyClient>,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + HyperbridgeClaim + 'static,
	B: IsmpHost + IsmpProvider + 'static,
{
	let (sender, receiver) = tokio::sync::mpsc::channel::<Vec<TxReceipt>>(64);
	let task_a = {
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		let client_map = client_map.clone();
		let tx_payment = tx_payment.clone();
		let config = config.clone();
		let sender = sender.clone();
		tokio::spawn(async move {
			let _ = handle_notification(
				chain_a,
				chain_b,
				tx_payment,
				config,
				coprocessor,
				client_map,
				sender,
			)
			.await?;
			Ok::<_, anyhow::Error>(())
		})
	};

	let task_b = {
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		let client_map = client_map.clone();
		let tx_payment = tx_payment.clone();
		let config = config.clone();
		let sender = sender.clone();
		tokio::spawn(async move {
			let _ = handle_notification(
				chain_b,
				chain_a,
				tx_payment,
				config,
				coprocessor,
				client_map,
				sender,
			)
			.await?;
			Ok::<_, anyhow::Error>(())
		})
	};

	// Fee accumulation background task
	{
		let hyperbridge = chain_a.clone();
		let dest = chain_b.clone();
		let client_map = client_map.clone();
		let tx_payment = tx_payment.clone();

		tokio::spawn(async move {
			fee_accumulation(receiver, dest, hyperbridge, client_map, tx_payment).await
		})
	};

	// Spawn retries for unprofitable messages
	{
		let hyperbridge = chain_a.clone();
		let dest = chain_b.clone();
		let client_map = client_map.clone();
		let tx_payment = tx_payment.clone();
		let config = config.clone();
		let sender = sender.clone();
		spawn_unprofitable_retries_task(
			dest,
			hyperbridge,
			client_map,
			tx_payment,
			config,
			coprocessor,
			sender,
		)?;
	}

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
	fee_acc_sender: FeeAccSender,
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
					fee_acc_sender.clone(),
				)
				.await
				{
					tracing::error!(
						"Error while handling {:?} on {:?}: {err:?}",
						state_machine_update.state_machine_id.state_id,
						chain_a.state_machine_id().state_id
					);
				}
			},
			Err(e) => {
				tracing::error!(target: "tesseract","Messaging task {}->{} encountered an error: {e:?}", chain_a.name(), chain_b.name());
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

async fn handle_update<A, B>(
	chain_a: &A,
	chain_b: &B,
	tx_payment: &Arc<TransactionPayment>,
	state_machine_update: StateMachineUpdated,
	previous_height: &mut u64,
	config: RelayerConfig,
	coprocessor: Chain,
	client_map: &HashMap<StateMachine, AnyClient>,
	fee_acc_sender: FeeAccSender,
) -> Result<(), anyhow::Error>
where
	A: IsmpHost + IsmpProvider + 'static,
	B: IsmpHost + IsmpProvider + 'static,
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
			tracing::error!(
				"Encountered an error querying events from {:?}: {err:?}",
				chain_b.state_machine_id().state_id
			);
			Default::default()
		},
	};

	let state_machine = state_machine_update.state_machine_id.state_id;
	if events.is_empty() {
		tracing::info!(
			"Skipping latest finalized height {} on {}, no new messages from {state_machine:?} in range {:?}",
			state_machine_update.latest_height,
			chain_a.name(),
			*previous_height..=state_machine_update.latest_height
		);
		*previous_height = state_machine_update.latest_height;
		return Ok(());
	}
	// Advance latest known height by relayer
	*previous_height = state_machine_update.latest_height;
	let log_events = events.clone().into_iter().map(Into::into).collect::<Vec<Event>>();
	tracing::info!(
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

	let (messages, unprofitable) = translate_events_to_messages(
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
		tracing::info!(
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
						// Store receipts in database before auto accumulation
						tracing::trace!(target: "tesseract", "Persisting {} deliveries from {}->{} to the db", receipts.len(), chain_b.name(), chain_a.name());
						if let Err(err) = tx_payment.store_messages(receipts.clone()).await {
							tracing::error!(
								"Failed to persist {} deliveries to database: {err:?}",
								receipts.len()
							)
						}
						// Send receipts to the fee accumulation task
						match fee_acc_sender.send(receipts).await {
							Err(_sent) => {
								tracing::error!(
									"Fee auto accumulation failed You can try again manually"
								)
							},
							_ => {},
						}
					}
				}
			},
			Err(err) => tracing::error!("Failed to submit transaction to {state_machine}: {err:?}"),
		}
	}

	// Store currently unprofitable in messages in db
	if !unprofitable.is_empty() {
		tracing::trace!(target: "tesseract", "Persisting {} unprofitable messages going to {} to the db", unprofitable.len(), chain_a.name());
		if let Err(err) = tx_payment
			.store_unprofitable_messages(unprofitable, chain_a.state_machine_id().state_id)
			.await
		{
			tracing::error!(
				"Encountered an error while storing unprofitable messages inside the database {err:?}"
			)
		}
	}

	Ok(())
}

async fn fee_accumulation<
	A: IsmpProvider + Clone + 'static,
	B: IsmpProvider + Clone + HyperbridgeClaim + 'static,
>(
	mut receiver: Receiver<Vec<TxReceipt>>,
	dest: A,
	hyperbridge: B,
	client_map: HashMap<StateMachine, AnyClient>,
	tx_payment: Arc<TransactionPayment>,
) -> Result<(), anyhow::Error> {
	while let Some(receipts) = receiver.recv().await {
		if receipts.is_empty() {
			continue;
		}
		// Spawn a new task so fee accumulation can be done concurrently
		tokio::spawn({
			let dest = dest.clone();
			let hyperbridge = hyperbridge.clone();
			let client_map = client_map.clone();
			let tx_payment = tx_payment.clone();
			async move {
				// Group receipts by source chain;
				// Query latest state machine height of source on hyperbridge
				// Get height at which messages were delivered to destination
				// Wait for state machine update on hyperbridge
				// Generate proofs
				// Observe challenge period
				// Submit proof
				let mut groups = HashMap::new();
				let delivery_height = receipts
					.iter()
					.max_by(|a, b| a.height().cmp(&b.height()))
					.map(|tx| tx.height())
					.expect("Infallible");
				receipts.iter().for_each(|receipt| match receipt {
					TxReceipt::Request { query, .. } => {
						let entry = groups.entry(query.source_chain).or_insert(vec![]);
						entry.push(*receipt);
					},
					TxReceipt::Response { query, .. } => {
						let entry = groups.entry(query.source_chain).or_insert(vec![]);
						entry.push(*receipt);
					},
				});

				// Wait for destination chain's state machine update on hyperbridge
				tracing::info!(
					"Fee accumulation for {} messages submitted to {:?} has started",
					receipts.len(),
					dest.state_machine_id().state_id
				);
				let dest_height = match wait_for_state_machine_update(
					dest.state_machine_id(),
					&hyperbridge,
					delivery_height,
				)
				.await
				{
					Ok(height) => height,
					Err(err) => {
						tracing::error!("An error occurred while waiting for a state machine update, auto fee accumulation failed, Receipts have been stored in the db you can try again manually \n{err:?}");
						if let Err(err) = tx_payment.store_messages(receipts).await {
							tracing::error!(
								"Failed to store some delivered messages to database: {err:?}"
							)
						}
						return;
					},
				};

				let stream = futures::stream::iter(groups);
				stream.for_each_concurrent(None, |(source, receipts)| {
					let hyperbridge = hyperbridge.clone();
					let source_chain = client_map.get(&source).cloned();
					let dest = dest.clone();
					let tx_payment = tx_payment.clone();
					async move {
						let lambda = || async {
							let source_chain = source_chain.ok_or_else(|| anyhow!("Client for {source} not found in config, fees cannot be accumulated"))?;
							let source_height = hyperbridge.query_latest_height(source_chain.state_machine_id()).await?;
							// Create claim proof for deliveries from source to dest
							tracing::info!("Creating withdrawal proofs from db for deliveries from {:?}->{:?}", source, dest.state_machine_id().state_id);
							let proofs = tx_payment
							.create_proof_from_receipts(source_height.into(), dest_height, &source_chain, &dest, receipts.clone())
							.await?;

							observe_challenge_period(&dest, &hyperbridge, dest_height).await?;
							let mut commitments =  vec![];
							for proof in proofs {
								commitments.extend_from_slice(&proof.commitments);
								hyperbridge.accumulate_fees(proof).await?;
							}
							tracing::info!("Fee accumulation was sucessful");
							// If delete fails, not an issue, they'll be deleted whenever manual accumulation is triggered
							let _ = tx_payment.delete_claimed_entries(commitments).await;
							Ok::<_, anyhow::Error>(())
						};

						match lambda().await {
							Ok(()) => {},
							Err(err) => {
								tracing::error!("Error accummulating some fees, receipts have been stored in the db, you can try again manually \n{err:?}");
							}
						}
					}
				}).await;
			}
		});
	}
	Ok::<_, anyhow::Error>(())
}
