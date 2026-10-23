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

/// How often both outbound claim tasks wake to process pending rows.
pub(crate) const CLAIM_INTERVAL_SECS: u64 = 600;

pub mod events;
pub mod fees;
mod get_requests;
pub mod outbound;
pub mod outbound_claim;
pub mod outbound_request_claim;
/// Retry loop for deliveries that never landed. Spawned by
/// [`outbound::initialize`], one per destination, not by [`inbound`]: retrying
/// is the outbound task's concern.
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
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::Event,
	host::StateMachine,
	messaging::{Message, Proof, RequestMessage},
	router::GetRequest,
};
use primitive_types::U256;

use tesseract_primitives::{
	config::RelayerConfig, observe_challenge_period, wait_for_state_machine_update,
	HandleGetResponse, HyperbridgeClaim, IsmpProvider, StateMachineUpdated, TxReceipt, TxResult,
};
use transaction_fees::{unix_now, ParkedMessage, RetryState, TransactionPayment};

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
/// - Retries of undelivered messages — [`crate::retries::retry_undelivered_messages`] is spawned
///   per destination by [`outbound::initialize`], which is also what parks the messages it drains.
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
		tokio::sync::mpsc::channel::<(Vec<GetRequest>, StateMachineUpdated)>(512);

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
						retry_from = previous_height,
						to = state_machine_update.latest_height,
						?err,
						"Error while handling state machine update, the range will be read again on the next update",
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
	//
	// `previous_height` only moves past a range once its events were read. A range whose events
	// could not be read, or that carries a GetResponse whose proof failed, is queried again on
	// the next update; a range whose own proofs could not be built has its post requests parked
	// (see [`park_request_events`]) so the next update proves them at its own, newer height
	// instead of re-reading an ever-growing range.
	let fresh = chain_b
		.query_ismp_events(*previous_height, state_machine_update.clone())
		.await
		.map_err(|err| anyhow!("Error querying events from source chain: {err:?}"))?;

	// GET requests need no proof and are forwarded once per range, so they are taken from the
	// events read in this range only, never from the parked rows.
	let get_requests = fresh
		.iter()
		.filter_map(|event| match event {
			Event::GetRequest(req) => Some(req.clone()),
			_ => None,
		})
		.collect::<Vec<_>>();
	let fresh_events = fresh
		.into_iter()
		.filter(|ev| filter_events(&config, chain_a.state_machine_id().state_id, coprocessor, ev))
		.collect::<Vec<_>>();

	// Requests parked by an earlier update whose proofs failed. They were admissible when read,
	// so `filter_events` is deliberately not re-applied: a later config change must not silently
	// drop them.
	let sink_state = chain_a.state_machine_id().state_id;
	// Every inbound pipeline that delivers to hyperbridge parks its rows under the same sink key,
	// so the requests' own source is what tells this pipeline's rows from another source chain's.
	let source_state = chain_b.state_machine_id().state_id;
	let mut parked_events = Vec::new();
	let mut parked_ids = Vec::new();
	for ParkedMessage { message, id, .. } in tx_payment.unprofitable_messages(&sink_state).await? {
		let Message::Request(msg) = message else {
			tracing::debug!(
				target: LOG_TARGET,
				?message,
				"ignoring non-request row parked for the inbound pipeline",
			);
			continue;
		};
		let row_sources = msg.requests.iter().map(|post| post.source).collect::<Vec<_>>();
		let own_requests = msg
			.requests
			.into_iter()
			.filter(|post| post.source == source_state)
			.collect::<Vec<_>>();
		if own_requests.is_empty() {
			tracing::debug!(
				target: LOG_TARGET,
				source = %source_state,
				row_sources = ?row_sources,
				"ignoring parked row for another source chain",
			);
			continue;
		}
		parked_events.extend(own_requests.into_iter().map(Event::PostRequest));
		if let Some(id) = id {
			parked_ids.push(id);
		}
	}

	let mut events = fresh_events.clone();
	events.extend(parked_events);

	let state_machine = state_machine_update.state_machine_id.state_id;
	let (messages, unprofitable) = if events.is_empty() {
		tracing::info!(
			target: LOG_TARGET, "Skipping latest finalized height {} on {}, no new messages from {state_machine} in range {:?}",
			state_machine_update.latest_height,
			chain_a.name(),
			*previous_height..=state_machine_update.latest_height
		);
		Default::default()
	} else {
		let log_events = events
			.iter()
			.chunk_by(|event| match event {
				ismp::events::Event::PostRequest(req) => req.dest,
				// A GetResponse is delivered back to the chain that made the request.
				ismp::events::Event::GetResponse(res) => res.get.source,
				event => {
					unreachable!(
						"Only application messages filtered; qed. Unexpected event: {event:?}"
					)
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

		match translate_events_to_messages(
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
		.await
		{
			Ok(result) => result,
			Err(err) => {
				// A GetResponse has no row to re-prove from, so its range has to be read again
				// to not lose it.
				if fresh_events.iter().any(|event| matches!(event, Event::GetResponse(_))) {
					return Err(err)
				}
				// Only this range's events are parked; the already parked rows were read above
				// and stay in the db for the next update. Never advance past events that were
				// not persisted, so a failed store leaves the range to be read again.
				let parked = park_request_events(
					&tx_payment,
					&*chain_a,
					state_machine_update.state_machine_id,
					state_machine_update.latest_height,
					&fresh_events,
				)
				.await?;
				*previous_height = state_machine_update.latest_height;
				forward_get_requests(&get_request_sender, get_requests, &state_machine_update);
				tracing::error!(
					target: LOG_TARGET,
					source = %chain_b.name(),
					dest = %chain_a.name(),
					source_height = state_machine_update.latest_height,
					parked,
					?err,
					"Could not build proofs for this range, parked its requests to be proved at a newer height",
				);
				return Ok(())
			},
		}
	};

	// Advance latest known height by relayer
	*previous_height = state_machine_update.latest_height;

	// The parked requests were joined to this range's proof query, so their rows are done.
	// Requests that have since timed out were dropped by the translation above; deleting
	// their rows is correct either way.
	if !parked_ids.is_empty() {
		if let Err(err) = tx_payment.delete_unprofitable_messages(parked_ids).await {
			tracing::warn!(
				target: LOG_TARGET,
				?err,
				"Failed to delete parked requests from the database; they dedup on the next read",
			);
		}
	}

	forward_get_requests(&get_request_sender, get_requests, &state_machine_update);

	if !messages.is_empty() {
		tracing::info!(
			target: LOG_TARGET,
			"🛰️ Transmitting ismp messages from {} to {}",
			chain_b.name(), chain_a.name()
		);

		let res = chain_a.submit(messages.clone(), coprocessor).await;
		match res {
			Ok(TxResult { receipts, unsuccessful, new_epochs: _ }) => {
				// Deliveries to hyperbridge itself are not accumulated, hyperbridge is
				// where those fees are claimed from.
				if chain_a.state_machine_id().state_id != coprocessor {
					record_deliveries(&tx_payment, &fee_acc_sender, receipts, coprocessor).await;
				}

				// Nothing retries deliveries on this pipeline: the retry loop only
				// serves the EVM destinations the outbound fan-out parks for. A
				// cancelled batch here is picked up again by whichever relayer next
				// sees the events, so it is only logged.
				if !unsuccessful.is_empty() {
					tracing::error!(
						target: LOG_TARGET,
						source = %chain_b.name(),
						dest = %chain_a.name(),
						count = unsuccessful.len(),
						"Some transactions were cancelled",
					);
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

	if !unprofitable.is_empty() {
		tracing::debug!(
			target: LOG_TARGET,
			source = %chain_b.name(),
			dest = %chain_a.name(),
			dropped = unprofitable.len(),
			"unprofitable messages dropped",
		);
	}

	Ok(())
}

/// Store the post requests of a range whose proofs could not be built, so the next update proves
/// them at a newer height rather than re-reading the range.
///
/// The rows are keyed by the sink's state machine: that key groups the rows of every inbound
/// pipeline delivering to the same sink, so every pipeline for hyperbridge reads back the same
/// rows. Each request's own `source` is what tells one source chain's rows from another's, and a
/// pipeline only joins the requests of its own source to its proof query. The only other user of
/// this table is the retry loop
/// ([`crate::retries::retry_undelivered_messages`]), which only ever drains EVM destinations, so
/// an inbound row (the sink is hyperbridge) cannot cross-talk with it.
async fn park_request_events(
	tx_payment: &TransactionPayment,
	sink: &dyn IsmpProvider,
	source_state_machine_id: StateMachineId,
	height: u64,
	events: &[Event],
) -> anyhow::Result<usize> {
	let requests = events
		.iter()
		.filter_map(|event| match event {
			Event::PostRequest(post) => Some(ParkedMessage {
				message: Message::Request(RequestMessage {
					requests: vec![post.clone()],
					// The proof is only a carrier for the height the request was read at; the row
					// is never submitted from the db. The next update re-proves the request at its
					// own, then-current height.
					proof: Proof {
						height: StateMachineHeight { id: source_state_machine_id, height },
						proof: Vec::new(),
					},
					signer: sink.address(),
				}),
				// Inbound rows are drained by this handler, not the per-destination retry loop,
				// so the state is inert; the timestamp keeps the row consistent with the schema.
				state: RetryState { created_at: unix_now(), ..Default::default() },
				id: None,
			}),
			_ => None,
		})
		.collect::<Vec<_>>();
	if requests.is_empty() {
		return Ok(0)
	}

	let parked = requests.len();
	tx_payment
		.store_unprofitable_messages(requests, sink.state_machine_id().state_id)
		.await?;
	Ok(parked)
}

/// Forward a range's GET requests to the GET-request processor.
fn forward_get_requests(
	get_request_sender: &Option<GetReqSender>,
	get_requests: Vec<GetRequest>,
	state_machine_update: &StateMachineUpdated,
) {
	let Some(sender) = get_request_sender else { return };
	if get_requests.is_empty() {
		return
	}
	// `try_send` so this inbound forwarder never blocks on a backed-up get-request
	// consumer. On `Full` the range's GET requests are dropped, as the range is not
	// queried again; warn so it surfaces.
	if let Err(err) = sender.try_send((get_requests, state_machine_update.clone())) {
		tracing::warn!(target: "messaging-inbound", ?err, "get-request channel send failed",);
	}
}

/// Persist deliveries and hand them to the fee accumulation task.
///
/// Receipts for requests that originated on hyperbridge are dropped: those are
/// paid for by the outbound delivery reward, which the claim tasks collect, so
/// accumulating them here would bill hyperbridge twice for the same delivery.
/// Sending is a `try_send` so a backed up accumulator never stalls a delivery
/// task; the rows are already in the db by then, which is what the manual
/// `accumulate-fees` subcommand reads.
pub(crate) async fn record_deliveries(
	tx_payment: &TransactionPayment,
	fee_acc_sender: &Option<FeeAccSender>,
	receipts: Vec<TxReceipt>,
	coprocessor: StateMachine,
) {
	let Some(sender) = fee_acc_sender else { return };
	let receipts = receipts
		.into_iter()
		.filter(|receipt| receipt.source() != coprocessor)
		.collect::<Vec<_>>();
	if receipts.is_empty() {
		return;
	}

	tracing::trace!(target: LOG_TARGET, count = receipts.len(), "Persisting deliveries to the db");
	if let Err(err) = tx_payment.store_messages(receipts.clone()).await {
		tracing::error!(
			target: LOG_TARGET,
			count = receipts.len(),
			?err,
			"Failed to persist deliveries to database",
		);
	}

	if let Err(err) = sender.try_send(receipts) {
		tracing::error!(
			target: LOG_TARGET,
			?err,
			"Fee auto accumulation channel send failed; you can try again manually",
		);
	}
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

				let fee = source_chain.query_request_fee_metadata(receipt.query.commitment).await;

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
		receipts.iter().for_each(|receipt| {
			let entry = groups.entry(receipt.query.source_chain).or_insert(vec![]);
			entry.push(*receipt);
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
							.create_proof_from_receipts(source_height.into(), dest_height, source_chain.clone(), dest.clone(), receipts.clone(), &*hyperbridge)
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

#[cfg(test)]
mod tests {
	use super::*;
	use ismp::{messaging::Message, router::PostRequest};
	use tesseract_primitives::mocks::MockHost;

	const SOURCE: StateMachine = StateMachine::Evm(97);
	const DEST: StateMachine = StateMachine::Evm(80002);
	const COPROCESSOR: StateMachine = StateMachine::Kusama(4009);

	/// Build a unique on-disk path for a test database.
	fn temp_db_path() -> String {
		let pid = std::process::id();
		let nanos = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|d| d.as_nanos())
			.unwrap_or(0);
		format!("/tmp/tesseract-messaging-test-{pid}-{nanos}.db")
	}

	fn post() -> PostRequest {
		PostRequest {
			source: SOURCE,
			dest: DEST,
			nonce: 1,
			from: vec![1],
			to: vec![2],
			timeout_timestamp: 0,
			body: vec![],
		}
	}

	fn get() -> GetRequest {
		GetRequest {
			source: SOURCE,
			dest: DEST,
			nonce: 2,
			from: vec![1],
			keys: vec![vec![3]],
			height: 10,
			context: vec![],
			timeout_timestamp: 0,
		}
	}

	struct Handler {
		chain_a: Arc<dyn IsmpProvider>,
		chain_b: Arc<dyn IsmpProvider>,
		tx_payment: Arc<TransactionPayment>,
		get_request_sender: GetReqSender,
	}

	impl Handler {
		async fn update(
			&self,
			latest_height: u64,
			previous_height: &mut u64,
		) -> anyhow::Result<()> {
			let update = StateMachineUpdated {
				state_machine_id: self.chain_b.state_machine_id(),
				latest_height,
			};
			handle_update(
				self.chain_a.clone(),
				self.chain_b.clone(),
				self.tx_payment.clone(),
				update,
				previous_height,
				RelayerConfig::default(),
				COPROCESSOR,
				&HashMap::new(),
				None,
				Some(self.get_request_sender.clone()),
			)
			.await
		}
	}

	/// A range whose proofs cannot be built is not read again: `previous_height` advances, its GET
	/// requests are forwarded and its post requests are parked under the sink's state machine. The
	/// next update joins them to its own proof query, so they are proved at the newer height.
	#[tokio::test]
	async fn a_range_whose_proofs_fail_is_parked_and_proved_later() {
		let (post, get) = (post(), get());
		let chain_b = MockHost::new((), 0, SOURCE)
			.with_events(vec![Event::PostRequest(post.clone()), Event::GetRequest(get.clone())])
			.with_request_proof_fail();
		let request_proof_ok = chain_b.request_proof_ok.clone();
		let chain_b_events = chain_b.events.clone();
		let chain_a = MockHost::new((), 0, COPROCESSOR);
		let submitted = chain_a.submitted.clone();

		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let (get_request_sender, mut get_requests) = tokio::sync::mpsc::channel(8);
		let chain_a: Arc<dyn IsmpProvider> = Arc::new(chain_a);
		let chain_b: Arc<dyn IsmpProvider> = Arc::new(chain_b);

		let mut previous_height = 100;
		let handle =
			Handler { chain_a, chain_b, tx_payment: tx_payment.clone(), get_request_sender };

		handle.update(200, &mut previous_height).await.unwrap();
		assert_eq!(previous_height, 200);
		assert!(submitted.lock().unwrap().is_empty());
		let (forwarded, update) = get_requests.try_recv().unwrap();
		assert_eq!(forwarded, vec![get]);
		assert_eq!(update.latest_height, 200);
		assert!(get_requests.try_recv().is_err());

		let parked = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(parked.len(), 1);
		match &parked[0].message {
			Message::Request(msg) => assert_eq!(msg.requests, vec![post.clone()]),
			message => panic!("expected one parked request, got {message:?}"),
		}

		*request_proof_ok.lock().unwrap() = true;
		*chain_b_events.lock().unwrap() = Vec::new();
		handle.update(300, &mut previous_height).await.unwrap();

		assert_eq!(previous_height, 300);
		{
			let submitted = submitted.lock().unwrap();
			assert_eq!(submitted.len(), 1);
			match submitted[0].as_slice() {
				[Message::Request(msg)] => {
					assert_eq!(msg.requests, vec![post]);
					assert_eq!(msg.proof.height.height, 300);
				},
				messages => panic!("expected one request message, got {messages:?}"),
			}
		}
		assert!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().is_empty());

		let _ = std::fs::remove_file(&path);
		let _ = std::fs::remove_file(format!("{path}-journal"));
	}

	/// Rows parked under a shared sink key belong to whichever pipeline's source chain emitted
	/// them. A row for another source is neither proved by this pipeline nor deleted by it: doing
	/// so would submit a proof taken from the wrong chain and lose the row.
	#[tokio::test]
	async fn parked_rows_are_only_proved_by_their_own_source() {
		let chain_a = MockHost::new((), 0, COPROCESSOR);
		let submitted = chain_a.submitted.clone();
		// No events of its own; the only request in play is the foreign row stored below.
		let chain_b = MockHost::new((), 0, SOURCE);

		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let foreign = PostRequest {
			source: StateMachine::Evm(8453),
			dest: DEST,
			nonce: 9,
			from: vec![1],
			to: vec![2],
			timeout_timestamp: 0,
			body: vec![],
		};
		tx_payment
			.store_unprofitable_messages(
				vec![ParkedMessage {
					message: Message::Request(RequestMessage {
						requests: vec![foreign.clone()],
						proof: Proof {
							height: StateMachineHeight {
								id: StateMachineId {
									state_id: StateMachine::Evm(8453),
									consensus_state_id: *b"BEEF",
								},
								height: 150,
							},
							proof: Vec::new(),
						},
						signer: Vec::new(),
					}),
					state: RetryState { created_at: unix_now(), ..Default::default() },
					id: None,
				}],
				COPROCESSOR,
			)
			.await
			.unwrap();

		let (get_request_sender, _get_requests) = tokio::sync::mpsc::channel(8);
		let chain_a: Arc<dyn IsmpProvider> = Arc::new(chain_a);
		let chain_b: Arc<dyn IsmpProvider> = Arc::new(chain_b);
		let mut previous_height = 100;
		let handle =
			Handler { chain_a, chain_b, tx_payment: tx_payment.clone(), get_request_sender };

		handle.update(200, &mut previous_height).await.unwrap();
		assert_eq!(previous_height, 200);
		assert!(submitted.lock().unwrap().is_empty());
		let parked = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(parked.len(), 1);
		match &parked[0].message {
			Message::Request(msg) => assert_eq!(msg.requests, vec![foreign.clone()]),
			message => panic!("expected the foreign request to stay parked, got {message:?}"),
		}

		let _ = std::fs::remove_file(&path);
		let _ = std::fs::remove_file(format!("{path}-journal"));
	}

	/// A range whose events cannot be read is not skipped: `previous_height` stays put, so the
	/// next update reads the same range again.
	#[tokio::test]
	async fn a_range_is_handled_again_after_its_event_query_fails() {
		let chain_b = MockHost::new((), 0, SOURCE)
			.with_events(vec![Event::PostRequest(post()), Event::GetRequest(get())])
			.with_query_events_fail();
		let query_events_ok = chain_b.query_events_ok.clone();
		let chain_a = MockHost::new((), 0, COPROCESSOR);
		let submitted = chain_a.submitted.clone();

		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let (get_request_sender, mut get_requests) = tokio::sync::mpsc::channel(8);
		let chain_a: Arc<dyn IsmpProvider> = Arc::new(chain_a);
		let chain_b: Arc<dyn IsmpProvider> = Arc::new(chain_b);

		let mut previous_height = 100;
		let handle =
			Handler { chain_a, chain_b, tx_payment: tx_payment.clone(), get_request_sender };

		handle.update(200, &mut previous_height).await.unwrap_err();
		assert_eq!(previous_height, 100);
		assert!(submitted.lock().unwrap().is_empty());
		assert!(get_requests.try_recv().is_err());
		assert!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().is_empty());

		*query_events_ok.lock().unwrap() = true;
		handle.update(300, &mut previous_height).await.unwrap();
		assert_eq!(previous_height, 300);

		let _ = std::fs::remove_file(&path);
		let _ = std::fs::remove_file(format!("{path}-journal"));
	}
}
