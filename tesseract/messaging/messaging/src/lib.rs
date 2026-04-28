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
/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = concat!("messaging", "-inbound");

pub mod events;
pub mod fees;
mod get_requests;
pub mod outbound;
pub mod outbound_claim;
/// Unprofitable-message retry loop. Kept public for callers that want to wire
/// it up; **not** spawned by [`inbound`] in the consolidated relayer — the
/// design is that retrying unprofitable messages is the outbound task's
/// concern.
pub mod retries;

use anyhow::anyhow;
use get_requests::process_get_request_events;
use itertools::Itertools;
use polkadot_sdk::sc_service::TaskManager;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::Instrument;

use crate::events::{filter_events, translate_events_to_messages};
use futures::{FutureExt, StreamExt};
use ismp::{consensus::StateMachineHeight, events::Event, host::StateMachine, router::GetRequest};
use primitive_types::U256;

use tesseract_primitives::{
	config::RelayerConfig, observe_challenge_period, wait_for_state_machine_update,
	HandleGetResponse, HyperbridgeClaim, IsmpProvider, StateMachineUpdated, TxReceipt, TxResult,
};
use transaction_fees::TransactionPayment;

type FeeAccSender = Sender<Vec<TxReceipt>>;
type GetReqSender = Sender<(Vec<GetRequest>, StateMachineUpdated)>;

/// Spawn the chain_b → Hyperbridge inbound messaging pipeline and the
/// GET-request processor.
///
/// **Explicitly NOT spawned here** (caller's responsibility or explicitly
/// descoped from this relayer):
/// - Outbound (Hyperbridge → chain_b) POST delivery — handled by [`tesseract_relayer::outbound`] as
///   a single event-driven fan-out task.
/// - Fee accumulation — spawned once per chain in `tesseract-relayer/src/cli.rs`.
/// - Fee withdrawal — spawned globally in `tesseract-relayer/src/cli.rs`.
/// - Unprofitable-message retries — the retry loop at
///   [`crate::retries::retry_unprofitable_messages`] is kept in-tree but deliberately **not**
///   spawned. Retrying unprofitable inbound messages is the outbound relayer's concern, not this
///   one.
///
/// The inbound messaging task already queries chain_b's events on every HB-side
/// state-machine-update, so it also feeds the GET-request channel that
/// [`process_get_request_events`] drains — no separate forwarder needed.
///
/// `fee_acc_sender` is forwarded to the inbound loop so `TxReceipt`s reach the
/// caller's fee-accumulation task. Pass `None` to opt this chain out of fee
/// accumulation entirely.
pub async fn inbound<A>(
	hyperbridge: A,
	chain_b: Arc<dyn IsmpProvider>,
	config: RelayerConfig,
	coprocessor: StateMachine,
	tx_payment: Arc<TransactionPayment>,
	client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	task_manager: &TaskManager,
	fee_acc_sender: Option<FeeAccSender>,
) -> Result<(), anyhow::Error>
where
	A: IsmpProvider + Clone + HyperbridgeClaim + HandleGetResponse + 'static,
{
	// Shared GET-request channel. The inbound messaging task populates it as a
	// side effect of its normal event querying; the processor task drains it.
	let (get_request_sender, get_request_receiver) =
		tokio::sync::mpsc::channel::<(Vec<GetRequest>, StateMachineUpdated)>(64);

	// chain_b → Hyperbridge inbound messaging. Also forwards any GET requests
	// seen on chain_b into `get_request_sender` (via handle_update's built-in
	// filter) so we don't need a separate forwarder task.
	{
		let hyperbridge = Arc::new(hyperbridge.clone());
		let chain_b_inner = chain_b.clone();
		let client_map = client_map.clone();
		let tx_payment = tx_payment.clone();
		let config = config.clone();
		let sender = fee_acc_sender.clone();
		let name = format!("messaging-{}-{}", chain_b.name(), hyperbridge.name());
		let span = tracing::info_span!("inbound_messaging", chain = %chain_b.name(), hb = %hyperbridge.name());
		task_manager.spawn_essential_handle().spawn_blocking(
			Box::leak(Box::new(name)),
			"messaging",
			async move {
				let res = handle_notification(
					hyperbridge,
					chain_b_inner,
					tx_payment,
					config,
					coprocessor,
					client_map,
					sender,
					Some(get_request_sender),
				)
				.await;
				tracing::error!(target: LOG_TARGET, ?res, "task terminated");
			}
			.instrument(span)
			.boxed(),
		);
	}

	// Hyperbridge → chain_b outbound messaging for substrate chains
	let chain_b_state = chain_b.state_machine_id().state_id;
	if chain_b_state.is_substrate() {
		let hyperbridge = Arc::new(hyperbridge.clone());
		let chain_b_inner = chain_b.clone();
		let client_map = client_map.clone();
		let tx_payment = tx_payment.clone();
		let config = config.clone();
		let sender = fee_acc_sender.clone();
		let name = format!("messaging-{}-{}", chain_b.name(), hyperbridge.name());
		let span = tracing::info_span!("inbound_messaging", chain = %chain_b.name(), hb = %hyperbridge.name());
		task_manager.spawn_essential_handle().spawn_blocking(
			Box::leak(Box::new(name)),
			"messaging",
			async move {
				let res = handle_notification(
					chain_b_inner,
					hyperbridge,
					tx_payment,
					config,
					coprocessor,
					client_map,
					sender,
					None,
				)
				.await;
				tracing::error!(target: LOG_TARGET, ?res, "task terminated");
			}
			.instrument(span)
			.boxed(),
		);
	}

	// GET-request processor: drains the channel fed by the inbound messaging
	// task, builds source + storage proofs, and submits each as a
	// `GetRequestsWithProof` to Hyperbridge.
	{
		let hb_processor = hyperbridge.clone();
		let source = chain_b.clone();
		let client_map = client_map.clone();
		let name = format!("get-processor-{}-{}", chain_b.name(), source.name());
		let span =
			tracing::info_span!("get_processor", chain = %source.name(), hb = %hb_processor.name());
		task_manager.spawn_essential_handle().spawn_blocking(
			Box::leak(Box::new(name)),
			"messaging",
			async move {
				let res = process_get_request_events(
					get_request_receiver,
					source,
					hb_processor,
					client_map,
				)
				.await;
				tracing::error!(target: LOG_TARGET, ?res, "task terminated");
			}
			.instrument(span)
			.boxed(),
		);
	}

	Ok(())
}

async fn handle_notification(
	chain_a: Arc<dyn IsmpProvider>,
	chain_b: Arc<dyn IsmpProvider>,
	tx_payment: Arc<TransactionPayment>,
	config: RelayerConfig,
	coprocessor: StateMachine,
	client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	fee_acc_sender: Option<FeeAccSender>,
	get_request_sender: Option<GetReqSender>,
) -> Result<(), anyhow::Error> {
	let mut state_machine_update_stream = chain_a
		.state_machine_update_notification(chain_b.state_machine_id())
		.await
		.map_err(|err| {
			anyhow!(
				"StateMachineUpdated stream subscription failed (source={}, dest={}): {err:?}",
				chain_b.name(),
				chain_a.name(),
			)
		})?;

	let mut previous_height = chain_b.initial_height();

	while let Some(item) = state_machine_update_stream.next().await {
		match item {
			Ok(state_machine_update) => {
				if let Err(err) = handle_update(
					chain_a.clone(),
					chain_b.clone(),
					tx_payment.clone(),
					state_machine_update.clone(),
					&mut previous_height,
					config.clone(),
					coprocessor,
					&client_map,
					fee_acc_sender.clone(),
					get_request_sender.clone(),
				)
				.await
				{
					tracing::error!(
						target: LOG_TARGET,
						source = %chain_b.name(),
						dest = %chain_a.name(),
						state_machine = %state_machine_update.state_machine_id.state_id,
						?err,
						"Error while handling state machine update",
					);
				}
			},
			Err(e) => {
				tracing::error!(
					target: LOG_TARGET,
					source = %chain_b.name(),
					dest = %chain_a.name(),
					err = ?e,
					"Messaging task state-machine-update stream error",
				);
				continue;
			},
		}
	}

	Err(anyhow!(
		"{}->{} messaging task has failed, Please restart relayer",
		chain_b.name(),
		chain_a.name()
	))?
}

async fn handle_update(
	chain_a: Arc<dyn IsmpProvider>,
	chain_b: Arc<dyn IsmpProvider>,
	tx_payment: Arc<TransactionPayment>,
	state_machine_update: StateMachineUpdated,
	previous_height: &mut u64,
	config: RelayerConfig,
	coprocessor: StateMachine,
	client_map: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	fee_acc_sender: Option<FeeAccSender>,
	get_request_sender: Option<GetReqSender>,
) -> Result<(), anyhow::Error> {
	// Chain B's state machine has been updated to a new height on chain A
	// We query all the events that have been emitted on chain B that can be submitted to
	// chain A filter events list to contain only Request and Response events
	let result = chain_b.query_ismp_events(*previous_height, state_machine_update.clone()).await;

	let events = match result {
		Ok(events) => {
			if let Some(sender) = get_request_sender {
				let get_requests = events
					.clone()
					.into_iter()
					.filter_map(|e| match e {
						Event::GetRequest(req) => Some(req),
						_ => None,
					})
					.collect::<Vec<_>>();
				if !get_requests.is_empty() {
					let _ = sender.send((get_requests, state_machine_update.clone())).await;
				}
			}

			events
				.into_iter()
				.filter(|ev| {
					filter_events(&config, coprocessor, chain_a.state_machine_id().state_id, ev)
				})
				.collect::<Vec<_>>()
		},
		Err(err) => {
			tracing::error!(
				target: LOG_TARGET,
				source = %chain_b.name(),
				dest = %chain_a.name(),
				?err,
				"Error querying events from source chain",
			);
			Default::default()
		},
	};

	let state_machine = state_machine_update.state_machine_id.state_id;
	if events.is_empty() {
		tracing::info!(
			target: LOG_TARGET, "Skipping latest finalized height {} on {}, no new messages from {state_machine} in range {:?}",
			state_machine_update.latest_height,
			chain_a.name(),
			*previous_height..=state_machine_update.latest_height
		);
		*previous_height = state_machine_update.latest_height;
		return Ok(());
	}
	// Advance latest known height by relayer
	*previous_height = state_machine_update.latest_height;
	let log_events = events
		.iter()
		.chunk_by(|event| match event {
			ismp::events::Event::PostRequest(req) => req.dest,
			ismp::events::Event::PostResponse(res) => res.dest_chain(),
			event => {
				unreachable!("Only application messages filtered; qed. Unexpected event: {event:?}")
			},
		})
		.into_iter()
		.fold(String::default(), |acc, (state_machine, items)| {
			format!("{acc}{}->{}: {} messages, ", chain_b.name(), state_machine, items.count())
		});
	tracing::info!(target: LOG_TARGET, "{log_events}");
	let state_machine_height = StateMachineHeight {
		id: state_machine_update.state_machine_id,
		height: state_machine_update.latest_height,
	};

	let (messages, unprofitable) = translate_events_to_messages(
		chain_b.clone(),
		chain_a.clone(),
		events,
		state_machine_height.clone(),
		config.clone(),
		coprocessor,
		&client_map,
		// Inbound pipeline doesn't batch a consensus message alongside — the
		// dest's light client is advanced by a separate consensus task.
		None,
	)
	.await?;

	if !messages.is_empty() {
		tracing::info!(
			target: LOG_TARGET,
			"🛰️ Transmitting ismp messages from {} to {}",
			chain_b.name(), chain_a.name()
		);

		let res = chain_a.submit(messages.clone(), coprocessor).await;
		match res {
			Ok(TxResult { receipts, unsuccessful, new_epochs: _ }) => {
				if let Some(sender) = fee_acc_sender {
					// We should not store messages when they are delivered to hyperbridge
					if chain_a.state_machine_id().state_id != coprocessor {
						// Filter out receipts for transactions that originated from the coprocessor
						let receipts = receipts
							.into_iter()
							.filter(|receipt| receipt.source() != coprocessor)
							.collect::<Vec<_>>();
						if !receipts.is_empty() {
							// Store receipts in database before auto accumulation
							tracing::trace!(
								target: LOG_TARGET,
								source = %chain_b.name(),
								dest = %chain_a.name(),
								count = receipts.len(),
								"Persisting deliveries to the db",
							);
							if let Err(err) = tx_payment.store_messages(receipts.clone()).await {
								tracing::error!(
									target: LOG_TARGET,
									source = %chain_b.name(),
									dest = %chain_a.name(),
									count = receipts.len(),
									?err,
									"Failed to persist deliveries to database",
								)
							}
							// Send receipts to the fee accumulation task
							match sender.send(receipts).await {
								Err(_sent) => {
									tracing::error!(
										target: LOG_TARGET,
										source = %chain_b.name(),
										dest = %chain_a.name(),
										"Fee auto accumulation failed; you can try again manually",
									)
								},
								_ => {},
							}
						}
					}
				}

				if !unsuccessful.is_empty() &&
					config.unprofitable_retry_frequency.is_some() &&
					chain_a.state_machine_id().state_id != coprocessor
				{
					tracing::error!(
						target: LOG_TARGET,
						source = %chain_b.name(),
						dest = %chain_a.name(),
						count = unsuccessful.len(),
						"Some transactions were cancelled and will be retried",
					);
					tracing::trace!(
						target: LOG_TARGET,
						source = %chain_b.name(),
						dest = %chain_a.name(),
						count = unsuccessful.len(),
						"Persisting cancelled transactions to the db",
					);
					if let Err(err) = tx_payment
						.store_unprofitable_messages(
							unsuccessful,
							chain_a.state_machine_id().state_id,
						)
						.await
					{
						tracing::error!(
							target: LOG_TARGET,
							source = %chain_b.name(),
							dest = %chain_a.name(),
							?err,
							"Failed to persist cancelled messages to the database",
						)
					}
				}
			},
			Err(err) => {
				tracing::error!(
					target: LOG_TARGET,
					source = %chain_b.name(),
					dest = %chain_a.name(),
					?err,
					"Failed to submit transaction",
				)
			},
		}
	}

	// Store currently unprofitable in messages in db
	if !unprofitable.is_empty() &&
		config.unprofitable_retry_frequency.is_some() &&
		chain_a.state_machine_id().state_id != coprocessor
	{
		tracing::trace!(
			target: LOG_TARGET,
			source = %chain_b.name(),
			dest = %chain_a.name(),
			count = unprofitable.len(),
			"Persisting unprofitable messages to the db",
		);
		if let Err(err) = tx_payment
			.store_unprofitable_messages(unprofitable, chain_a.state_machine_id().state_id)
			.await
		{
			tracing::error!(
				target: LOG_TARGET,
				source = %chain_b.name(),
				dest = %chain_a.name(),
				?err,
				"Error while storing unprofitable messages in the database",
			)
		}
	}

	Ok(())
}

pub async fn fee_accumulation<A: IsmpProvider + Clone + Clone + HyperbridgeClaim + 'static>(
	mut receiver: Receiver<Vec<TxReceipt>>,
	dest: Arc<dyn IsmpProvider>,
	hyperbridge: A,
	client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	tx_payment: Arc<TransactionPayment>,
) -> Result<(), anyhow::Error> {
	let client_map = Arc::new(client_map);
	while let Some(receipts) = receiver.recv().await {
		let stream = futures::stream::iter(receipts.into_iter()).filter_map(|receipt| {
			let client_map = Arc::clone(&client_map);
			let tx_payment = Arc::clone(&tx_payment);
			async move {
				let source_chain = match client_map.get(&receipt.source()) {
					Some(client) => client.clone(),
					None => {
						return None;
					},
				};

				let fee = match receipt {
					TxReceipt::Request { query, .. } =>
						source_chain.query_request_fee_metadata(query.commitment).await,
					TxReceipt::Response { query, .. } =>
						source_chain.query_response_fee_metadata(query.commitment).await,
				};

				match fee {
					Ok(fee_amount) if fee_amount > U256::zero() => Some(receipt),
					Ok(_) => None,
					Err(err) => {
						tracing::warn!(
							target: LOG_TARGET,
							source = %source_chain.name(),
							?receipt,
							?err,
							"Failed to query fee for receipt; storing for retry",
						);
						if let Err(db_err) = tx_payment.store_messages(vec![receipt]).await {
							tracing::error!(
								target: LOG_TARGET,
								source = %source_chain.name(),
								err = ?db_err,
								"Failed to store receipt in DB after fee query error",
							);
						}
						None
					},
				}
			}
		});
		let receipts = stream.collect::<Vec<_>>().await;

		if receipts.is_empty() {
			continue;
		}

		let hyperbridge = Arc::new(hyperbridge.clone());

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
			target: LOG_TARGET,
			dest = %dest.name(),
			hb = %hyperbridge.name(),
			count = receipts.len(),
			"Fee accumulation started",
		);
		let dest_height = match wait_for_state_machine_update(
			dest.state_machine_id(),
			hyperbridge.clone(),
			dest.clone(),
			delivery_height,
		)
		.await
		{
			Ok(height) => height,
			Err(err) => {
				tracing::error!(
					target: LOG_TARGET,
					dest = %dest.name(),
					hb = %hyperbridge.name(),
					?err,
					"Waiting for state machine update failed; auto fee accumulation aborted, receipts stored in the db — you can retry manually",
				);
				if let Err(err) = tx_payment.store_messages(receipts).await {
					tracing::error!(
						target: LOG_TARGET,
						dest = %dest.name(),
						?err,
						"Failed to store delivered messages to database",
					)
				}
				continue;
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
							let source_chain = source_chain.ok_or_else(|| anyhow!("Client for source={source} (dest={}) not found in config, fees cannot be accumulated", dest.name()))?;
							let source_height = hyperbridge.query_latest_height(source_chain.state_machine_id()).await?;
							// Create claim proof for deliveries from source to dest
							tracing::info!(
								target: LOG_TARGET,
								source = %source_chain.name(),
								dest = %dest.name(),
								"Creating withdrawal proofs from db for deliveries",
							);
							let proofs = tx_payment
							.create_proof_from_receipts(source_height.into(), dest_height, source_chain.clone(), dest.clone(), receipts.clone())
							.await?;
							observe_challenge_period(source_chain.clone(), hyperbridge.clone(), source_height.into()).await?;
							observe_challenge_period(dest.clone(), hyperbridge.clone(), dest_height).await?;
							let mut commitments =  vec![];
							for proof in proofs {
								commitments.extend_from_slice(&proof.commitments);
								hyperbridge.accumulate_fees(proof).await?;
							}
							tracing::info!(
								target: LOG_TARGET,
								source = %source_chain.name(),
								dest = %dest.name(),
								"Fee accumulation was successful",
							);
							// If delete fails, not an issue, they'll be deleted whenever manual accumulation is triggered
							let _ = tx_payment.delete_claimed_entries(commitments).await;
							Ok::<_, anyhow::Error>(())
						};

						match lambda().await {
							Ok(()) => {},
							Err(err) => {
								tracing::error!(
									target: LOG_TARGET,
									%source,
									dest = %dest.name(),
									?err,
									"Error accumulating some fees; receipts stored in the db — you can retry manually",
								);
							}
						}
					}
				}).await;
	}
	Ok::<_, anyhow::Error>(())
}
