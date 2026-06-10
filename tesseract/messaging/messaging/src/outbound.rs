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

use std::{
	collections::{BTreeMap, BTreeSet, HashMap},
	sync::Arc,
};

use anyhow::anyhow;
use codec::{Decode, Encode};
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::Event,
	host::StateMachine,
	messaging::{hash_request, ConsensusMessage, Message},
	router::{PostRequest, Request},
};
use primitive_types::H256;
use tesseract_primitives::{
	config::RelayerConfig, ConsensusProofSource, Hasher, IsmpProvider, NewEpochEvent,
	PendingRequestDeliveryClaim, ProofAccepted, RotationProof, StateMachineUpdated, TxReceipt,
	BEEFY_CONSENSUS_STATE_ID,
};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use tokio::sync::mpsc::Sender;
use tracing::Instrument;
use transaction_fees::TransactionPayment;

use crate::events::{filter_events, translate_events_to_messages};

/// Log/tracing target for the outbound pipeline.
const LOG_TARGET: &str = concat!("messaging", "-outbound");

/// Cap on consensus proofs bundled into a single `submit` call. EVM destinations
/// enforce calldata and gas limits that a large rotation catch-up would blow
/// through — three BEEFY proofs is the empirical ceiling that still fits under
/// mainnet block gas on the hottest destinations.
const MAX_CONSENSUS_PROOFS_PER_BATCH: usize = 3;

pub async fn run(
	hyperbridge: Arc<dyn IsmpProvider>,
	hyperbridge_sub: SubstrateClient<KeccakSubstrateChain>,
	destinations: BTreeMap<StateMachine, Arc<dyn IsmpProvider>>,
	proof_source: Arc<dyn ConsensusProofSource>,
	relayer_config: RelayerConfig,
	client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	fee_senders: HashMap<StateMachine, Sender<Vec<TxReceipt>>>,
	claim_tx_payment: Option<Arc<TransactionPayment>>,
) -> Result<(), anyhow::Error> {
	let hb_state_machine_id = hyperbridge.state_machine_id();
	let coprocessor = hb_state_machine_id.state_id;
	let source_name = hyperbridge.name();
	let mut stream = hyperbridge.proof_accepted_notification().await?;
	let mut cursor: u64 = hyperbridge.initial_height();

	tracing::info!(target: LOG_TARGET, source = %source_name, cursor, "Subscribed to Beefy Proof Notifications");

	// Startup catch-up: before draining the live stream, walk every
	// destination once and try to land any mandatory rotation proofs
	// the destination is behind on. This covers the case where the
	// relayer was offline through one or more authority-set rotations —
	// without this pass, the destination's BEEFY light client stays at
	// the old set_id, every subsequent messaging proof's signature
	// verification fails, and the relayer can't deliver until a fresh
	// mandatory proof arrives on Hyperbridge.
	//
	// Subscription happens before this call so live `ProofAccepted`
	// events buffer in the stream while catch-up runs. Each
	// `catch_up_rotations` invocation is idempotent on the
	// `_epochs[set_id]` slot, so the per-proof catch-up inside
	// `submit_for_dest` will see "dest already current" and return an
	// empty vec the first time it fires after this. `current_set_id =
	// None` means "no in-flight rotation to filter out" — every
	// rotation the destination is behind on is eligible.
	{
		let mut startup_tasks = FuturesUnordered::new();
		for dest in destinations.values() {
			let hb = hyperbridge.clone();
			let dest = dest.clone();
			let proof_source = proof_source.clone();
			let claim_tx_payment = claim_tx_payment.clone();
			let dest_name = dest.name();
			let dest_state_machine = dest.state_machine_id().state_id;
			startup_tasks.push(
				async move {
					let result = catch_up_rotations(&hb, &dest, &proof_source, None).await;
					(dest_name, dest_state_machine, result, claim_tx_payment)
				}
				.boxed(),
			);
		}
		while let Some((dest_name, dest_state_machine, result, claim_tx_payment)) =
			startup_tasks.next().await
		{
			match result {
				Ok(new_epochs) =>
					if !new_epochs.is_empty() {
						tracing::info!(
							target: LOG_TARGET,
							source = %source_name,
							dest = %dest_name,
							rotations = new_epochs.len(),
							"startup rotation catch-up landed rotations",
						);
						forward_consensus_delivery_claims(
							&dest_name,
							dest_state_machine,
							new_epochs,
							&claim_tx_payment,
						)
						.await;
					},
				Err(err) => {
					tracing::warn!(
						target: LOG_TARGET,
						source = %source_name,
						dest = %dest_name,
						?err,
						"startup rotation catch-up failed; proceeding to stream loop",
					);
				},
			}
		}
	}

	while let Some(item) = stream.next().await {
		let accepted: ProofAccepted = match item {
			Ok(ev) => ev,
			Err(err) => {
				tracing::error!(target: LOG_TARGET, source = %source_name, ?err, "proof_accepted stream error");
				continue;
			},
		};

		let new_height = accepted.height;
		let is_mandatory = accepted.new_set_id.is_some();
		let new_set_id = accepted.new_set_id;

		let synth = StateMachineUpdated {
			state_machine_id: hb_state_machine_id,
			latest_height: new_height,
		};

		let events = match hyperbridge.query_ismp_events(cursor, synth).await {
			Ok(events) => events,
			Err(err) => {
				tracing::error!(
					target: LOG_TARGET,
					source = %source_name,
					cursor,
					to = new_height,
					?err,
					"query_ismp_events failed",
				);
				continue;
			},
		};

		let proof_bytes = match proof_source.fetch(new_height).await {
			Ok(bytes) => bytes,
			Err(err) => {
				tracing::error!(
					target: LOG_TARGET,
					source = %source_name,
					height = new_height,
					set_id = ?new_set_id,
					?err,
					"proof fetch failed",
				);
				continue;
			},
		};

		tracing::info!(
			target: LOG_TARGET,
			source = %source_name,
			height = new_height,
			set_id = ?new_set_id,
			mandatory = is_mandatory,
			events = events.len(),
			"Received New Beefy Proof",
		);

		// On-chain allowlist of `(destination, source_module_id)` pairs that earn
		// a relayer reward. Fetched once per BEEFY notification so governance
		// updates take effect within one cycle (~6s on hyperbridge).
		let incentivized = match hyperbridge_sub.incentivized_outbound_request_modules().await {
			Ok(set) => Some(Arc::new(set)),
			Err(err) => {
				tracing::warn!(
					target: LOG_TARGET,
					?err,
					"incentivized_outbound_request_modules fetch failed; not filtering this cycle",
				);
				None
			},
		};

		let mut tasks = FuturesUnordered::new();
		for dest in destinations.values() {
			let fee_sender = fee_senders.get(&dest.state_machine_id().state_id).cloned();
			let dest_name = dest.name();
			let dest_span = tracing::info_span!(
				"dest",
				source = %source_name,
				dest = %dest_name,
				height = new_height,
				mandatory = is_mandatory,
			);
			let event_ctx = OutboundEventContext {
				hyperbridge: hyperbridge.clone(),
				hb_state_machine_id,
				coprocessor,
				relayer_config: relayer_config.clone(),
				client_map: client_map.clone(),
				proof_source: proof_source.clone(),
				events: events.clone(),
				proof_bytes: proof_bytes.clone(),
				is_mandatory,
				new_height,
				new_set_id,
				incentivized: incentivized.clone(),
			};
			let dest_ctx = DestinationContext {
				dest: dest.clone(),
				fee_sender,
				claim_tx_payment: claim_tx_payment.clone(),
			};
			tasks.push(
				submit_for_dest(event_ctx, dest_ctx)
					.instrument(dest_span)
					.map(move |r| (dest_name, r)),
			);
		}
		while let Some((dest_name, res)) = tasks.next().await {
			if let Err(err) = res {
				tracing::error!(
					target: LOG_TARGET,
					source = %source_name,
					dest = %dest_name,
					?err,
					"submit_for_dest failed",
				);
			}
		}

		cursor = new_height;
	}

	Err(anyhow::anyhow!("proof_accepted stream ended (source={source_name})"))
}

/// Per-`ProofAccepted` context shared across every destination's
/// [`submit_for_dest`] invocation in the same outbound fan-out cycle.
/// Carved out so each destination only needs the dest-specific bits at
/// the call site instead of a 12+ argument list.
struct OutboundEventContext {
	hyperbridge: Arc<dyn IsmpProvider>,
	hb_state_machine_id: StateMachineId,
	coprocessor: StateMachine,
	relayer_config: RelayerConfig,
	client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	proof_source: Arc<dyn ConsensusProofSource>,
	events: Vec<Event>,
	proof_bytes: Vec<u8>,
	is_mandatory: bool,
	new_height: u64,
	new_set_id: Option<u64>,
	/// Snapshot of allowlisted `module_id`s for this fan-out cycle. `None`
	/// means the fetch failed and the filter should not run.
	incentivized: Option<Arc<BTreeSet<Vec<u8>>>>,
}

/// Per-destination args. Splits cleanly from [`OutboundEventContext`] so
/// the cycle context can be cloned cheaply once and passed to every
/// destination, with only the dest-specific bits varying per call.
struct DestinationContext {
	dest: Arc<dyn IsmpProvider>,
	fee_sender: Option<Sender<Vec<TxReceipt>>>,
	claim_tx_payment: Option<Arc<TransactionPayment>>,
}

async fn submit_for_dest(
	event_ctx: OutboundEventContext,
	dest_ctx: DestinationContext,
) -> Result<(), anyhow::Error> {
	let OutboundEventContext {
		hyperbridge,
		hb_state_machine_id,
		coprocessor,
		relayer_config,
		client_map,
		proof_source,
		events,
		proof_bytes,
		is_mandatory,
		new_height,
		new_set_id,
		incentivized,
	} = event_ctx;
	let DestinationContext { dest, fee_sender, claim_tx_payment } = dest_ctx;
	let dest_state_machine = dest.state_machine_id().state_id;
	let dest_name = dest.name();
	// Bring the destination's BEEFY light client up to HB's current
	// authority-set id before submitting the current update. A messaging
	// proof whose set_id is ahead of the destination's locally-known
	// authorities gets rejected by the BEEFY verifier, so any missing
	// rotations have to land first. Best-effort: if we can't read the
	// destination's consensus state we assume it's current and fall through.
	//
	// `new_set_id` is threaded through so catch-up skips the rotation that
	// the main batch is about to submit via `consensus_msg` (which is the
	// proof for `new_set_id` when this notification is mandatory). Any
	// `NewEpoch` set_ids the catch-up chunks land are merged into the
	// claim list below so each one earns its delivery reward.
	let catchup_new_epochs =
		match catch_up_rotations(&hyperbridge, &dest, &proof_source, new_set_id).await {
			Ok(epochs) => epochs,
			Err(err) => {
				tracing::warn!(
					target: LOG_TARGET,
					dest = %dest_name,
					?err,
					"rotation catch-up failed; proceeding with current update",
				);
				Vec::new()
			},
		};

	// Only events relevant to this destination matter; skip the RPC-heavy
	// translate_events_to_messages entirely when there's nothing to do.
	// `dest_state_machine` is the counterparty whose batch we're building,
	// so the filter keeps every event routed to this destination.
	let mut events = events
		.into_iter()
		.filter(|ev| filter_events(&relayer_config, dest_state_machine, coprocessor, ev))
		.collect::<Vec<_>>();

	retain_incentivized_requests(&mut events, coprocessor, incentivized.as_deref());
	let has_events_for_dest = events
		.iter()
		.any(|ev| matches!(ev, Event::PostRequest(req) if req.dest == dest_state_machine));

	if !has_events_for_dest && !is_mandatory {
		// Messaging-only proof with nothing for this chain — skip. Rotation
		// proofs (mandatory) must propagate even without user messages so future
		// messaging proofs remain verifiable on the destination.
		tracing::trace!(target: LOG_TARGET, dest = %dest_name, "skipping — no events for this chain, not mandatory");
		// Even though we're not submitting anything else, the catch-up
		// loop above may already have landed rotation proofs on this dest
		// — persist those new_epochs so the claim task can collect their
		// `OutboundConsensusDeliveryReward`.
		forward_consensus_delivery_claims(
			&dest_name,
			dest_state_machine,
			catchup_new_epochs,
			&claim_tx_payment,
		)
		.await;
		return Ok(());
	}

	let consensus_msg = Message::Consensus(ConsensusMessage {
		consensus_proof: proof_bytes,
		consensus_state_id: BEEFY_CONSENSUS_STATE_ID,
		signer: dest.address(),
	});
	let mut batch: Vec<Message> = vec![consensus_msg.clone()];

	if has_events_for_dest {
		let state_machine_height =
			StateMachineHeight { id: hb_state_machine_id, height: new_height };

		match translate_events_to_messages(
			hyperbridge.clone(),
			dest.clone(),
			events,
			state_machine_height,
			relayer_config,
			coprocessor,
			&client_map,
			// Pass the consensus update as the gas-estimation prelude so EVM
			// sinks simulate each message inside `batchCall([consensus, msg])`
			// — matching the real on-chain dispatch order.
			Some(consensus_msg),
		)
		.await
		{
			Ok((deliverable, unprofitable)) => {
				if !unprofitable.is_empty() {
					tracing::debug!(target: LOG_TARGET, dropped = unprofitable.len(), "unprofitable messages dropped");
				}
				batch.extend(deliverable);
			},
			Err(err) => {
				tracing::error!(target: LOG_TARGET, ?err, dest = %dest_name, "translate_events_to_messages failed")
			},
		}
	}

	// If translate returned no deliverable messages we may be left with only
	// the consensus entry — only worth sending on mandatory (rotation) proofs.
	if batch.len() == 1 && !is_mandatory {
		tracing::trace!(target: LOG_TARGET,dest = %dest_name, "skipping — consensus-only batch, not mandatory");
		// As above: catch-up rotations already landed on the dest carry
		// claim-eligible new_epochs; persist them before bailing out.
		forward_consensus_delivery_claims(
			&dest_name,
			dest_state_machine,
			catchup_new_epochs,
			&claim_tx_payment,
		)
		.await;
		return Ok(());
	}

	if batch.len() == 1 && is_mandatory {
		tracing::info!(target: "tesseract", msgs = batch.len(), "🛰️ Transmitting Mandatory Consensus Message to {dest_name}");
	} else {
		tracing::info!(target: "tesseract", msgs = batch.len(), "🛰️ Transmitting ismp messages to {dest_name}");
	}
	// Extract the post requests from the batch before submit consumes it,
	// so the request-claim forwarder can index them by commitment.
	let batch_requests: Vec<PostRequest> = batch
		.iter()
		.flat_map(|msg| match msg {
			Message::Request(req_msg) => req_msg.requests.clone(),
			_ => Vec::new(),
		})
		.collect();

	// `submit` transparently picks the right transport — EVM destinations
	// whose handler supports IHandlerV2 dispatch the whole batch as a single
	// `batchCall(bytes[])` tx; everything else uses the legacy serial path.
	let result = dest.submit(batch, hb_state_machine_id.state_id).await?;

	// Forward a claim for every hyperbridge-originated request just delivered.
	forward_request_delivery_claims(
		&dest_name,
		dest_state_machine,
		coprocessor,
		&batch_requests,
		&result.receipts,
		&claim_tx_payment,
	)
	.await;

	// Forward receipts for fee accumulation. `try_send` so the outbound
	// task never blocks on the fee-accumulator consumer; on `Full`,
	// persist the receipts to the local DB so the fee-accumulator
	// restart / `accumulate-fees` subcommand can replay them. On
	// `Closed` (shutdown), persist for the same reason. Receipts are
	// otherwise only seen in-memory on this path — unlike the inbound
	// flows in `lib.rs` / `retries.rs`, the outbound side doesn't
	// pre-persist before sending.
	if let (Some(sender), false) = (fee_sender, result.receipts.is_empty()) {
		match sender.try_send(result.receipts) {
			Ok(()) => {},
			Err(tokio::sync::mpsc::error::TrySendError::Full(receipts)) => {
				tracing::warn!(
					target: LOG_TARGET,
					count = receipts.len(),
					"fee-accumulation channel full; persisting receipts to DB instead of dropping",
				);
				if let Some(tx_payment) = &claim_tx_payment {
					if let Err(err) = tx_payment.store_messages(receipts).await {
						tracing::error!(
							target: LOG_TARGET,
							?err,
							"failed to persist outbound receipts to DB after channel-full",
						);
					}
				} else {
					tracing::warn!(
						target: LOG_TARGET,
						"no tx_payment handle available; receipts dropped (fees disabled)",
					);
				}
			},
			Err(tokio::sync::mpsc::error::TrySendError::Closed(receipts)) => {
				tracing::warn!(
					target: LOG_TARGET,
					count = receipts.len(),
					"fee-accumulation channel closed; persisting receipts to DB",
				);
				if let Some(tx_payment) = &claim_tx_payment {
					if let Err(err) = tx_payment.store_messages(receipts).await {
						tracing::error!(
							target: LOG_TARGET,
							?err,
							"failed to persist outbound receipts to DB after channel-closed",
						);
					}
				}
			},
		}
	}

	// Forward a claim for the outbound consensus delivery reward, one per
	// `NewEpoch(set_id, self_address)` log earned by this submission.
	// Combine `catchup_new_epochs` (rotations the catch-up landed) with
	// `result.new_epochs` (rotations the main batch landed) — both groups
	// contribute set_ids whose `_epochs[set_id]` slot now points at this
	// relayer's address, so both earn rewards. Order is preserved
	// (catch-up first, main batch second) which matches on-chain order.
	let mut combined_new_epochs = catchup_new_epochs;
	combined_new_epochs.extend(result.new_epochs.iter().copied());
	forward_consensus_delivery_claims(
		&dest_name,
		dest_state_machine,
		combined_new_epochs,
		&claim_tx_payment,
	)
	.await;

	Ok(())
}

/// Drop hyperbridge-originated requests whose `source_module` is not on
/// the on-chain reward allowlist. User-originated requests and non-request
/// events pass through. `None` means the snapshot fetch failed this cycle;
/// deliver everything as a no-op fallback.
fn retain_incentivized_requests(
	events: &mut Vec<Event>,
	coprocessor: StateMachine,
	incentivized: Option<&BTreeSet<Vec<u8>>>,
) {
	let Some(incentivized) = incentivized else { return };
	events.retain(|ev| match ev {
		Event::PostRequest(post) => post.source != coprocessor || incentivized.contains(&post.from),
		_ => true,
	});
}

async fn forward_consensus_delivery_claims(
	dest_name: &str,
	dest_state_machine: StateMachine,
	new_epochs: Vec<NewEpochEvent>,
	claim_tx_payment: &Option<Arc<TransactionPayment>>,
) {
	if new_epochs.is_empty() {
		return;
	}

	let Some(tx_payment) = claim_tx_payment else { return };

	let dest_str = dest_state_machine.to_string();
	let claim_rows: Vec<(u64, u64)> =
		new_epochs.iter().map(|ev| (ev.set_id, ev.block_number)).collect();

	if let Err(err) = tx_payment.insert_pending_rotation_claims(&dest_str, &claim_rows).await {
		tracing::warn!(
			target: LOG_TARGET,
			?err,
			dest = %dest_name,
			?claim_rows,
			"failed to persist outbound-consensus claims to DB; will retry on next delivery",
		);
	}
}

/// Sibling of [`forward_consensus_delivery_claims`] for the request reward.
/// Indexes the batch's `PostRequest`s by commitment, then for every
/// [`TxReceipt`] whose `source_chain` is the hyperbridge coprocessor,
/// recovers the original request from the batch and persists a
/// [`PendingRequestDeliveryClaim`] row to the local DB. The periodic claim
/// task picks these up on its next tick.
async fn forward_request_delivery_claims(
	dest_name: &str,
	dest_state_machine: StateMachine,
	coprocessor: StateMachine,
	batch_requests: &[PostRequest],
	receipts: &[TxReceipt],
	claim_tx_payment: &Option<Arc<TransactionPayment>>,
) {
	let Some(tx_payment) = claim_tx_payment else {
		return;
	};

	let claims = collect_hyperbridge_request_claims(coprocessor, batch_requests, receipts);
	if claims.is_empty() {
		return;
	}

	let rows: Vec<(String, Vec<u8>, u64)> = claims
		.iter()
		.map(|c| (hex::encode(c.commitment().0), c.request.encode(), c.delivery_height))
		.collect();
	if let Err(err) = tx_payment
		.insert_pending_request_claims(&dest_state_machine.to_string(), &rows)
		.await
	{
		tracing::warn!(
			target: LOG_TARGET,
			?err,
			dest = %dest_name,
			count = rows.len(),
			"failed to persist outbound-request claims; will retry on next delivery",
		);
	}
}

/// Filters receipts to those originating from the hyperbridge coprocessor and
/// pairs each one with its source request. Returns an empty vec when nothing
/// in the batch is hyperbridge-originated.
fn collect_hyperbridge_request_claims(
	coprocessor: StateMachine,
	batch_requests: &[PostRequest],
	receipts: &[TxReceipt],
) -> Vec<PendingRequestDeliveryClaim> {
	// Index every PostRequest in the batch by its commitment so receipts
	// can be paired back to their source request. BTreeMap keeps iteration
	// deterministic when building the claim list.
	let by_commitment: BTreeMap<H256, PostRequest> = batch_requests
		.iter()
		.map(|req| (hash_request::<Hasher>(&Request::Post(req.clone())), req.clone()))
		.collect();

	receipts
		.iter()
		.filter_map(|TxReceipt { query, height }| {
			(query.source_chain == coprocessor)
				.then(|| by_commitment.get(&query.commitment).cloned())
				.flatten()
				.map(|request| PendingRequestDeliveryClaim { request, delivery_height: *height })
		})
		.collect()
}

/// Read the BEEFY `current_authorities.id` out of a SCALE-encoded
/// [`beefy_verifier_primitives::ConsensusState`]. Returns `None` when the
/// bytes can't be decoded — treated by callers as "assume it's in sync".
fn decode_current_set_id_scale(encoded: &[u8]) -> Option<u64> {
	beefy_verifier_primitives::ConsensusState::decode(&mut &encoded[..])
		.ok()
		.map(|s| s.current_authorities.id)
}

/// If the destination's BEEFY `current_authorities.id` is behind HB's, fetch
/// every intervening rotation proof and submit them before the current update
/// lands. Rotations are submitted in chunks of
/// [`MAX_CONSENSUS_PROOFS_PER_BATCH`] to stay inside EVM calldata/gas limits.
///
/// `current_set_id` is the rotation that the *current* `ProofAccepted`
/// notification is about to land via the main batch's consensus message
/// (`Some(set_id)` when the current notification is a mandatory update,
/// `None` when it's messaging-only). Catch-up filters that set_id out of
/// the rotation list so it isn't submitted twice — the duplicate would be
/// a wasted batch slot at best and a contract revert at worst.
///
/// Returns every `NewEpoch(set_id, self)` log observed across the catch-up
/// chunks' receipts (i.e. the rotations this relayer was the first to
/// land). The caller forwards them to the outbound-claim task alongside
/// the main batch's `new_epochs` so each one earns its
/// `OutboundConsensusDeliveryReward`.
///
/// This is a best-effort catch-up: any failure (stale offchain proofs, submit
/// reverts on one chunk) is logged and surfaced to the caller, which then
/// decides whether to still attempt the current update.
async fn catch_up_rotations(
	hyperbridge: &Arc<dyn IsmpProvider>,
	dest: &Arc<dyn IsmpProvider>,
	proof_source: &Arc<dyn ConsensusProofSource>,
	current_set_id: Option<u64>,
) -> Result<Vec<NewEpochEvent>, anyhow::Error> {
	let dest_name = dest.name();
	let dest_consensus = dest
		.query_consensus_state(None, BEEFY_CONSENSUS_STATE_ID)
		.await
		.map_err(|err| anyhow::anyhow!("query dest consensus_state: {err:?}"))?;
	let Some(dest_set_id) = decode_current_set_id_scale(&dest_consensus) else {
		tracing::debug!(
			target: LOG_TARGET,
			"dest consensus state undecodable; skipping rotation catch-up",
		);
		return Ok(Vec::new());
	};

	let hb_consensus = hyperbridge
		.query_consensus_state(None, BEEFY_CONSENSUS_STATE_ID)
		.await
		.map_err(|err| anyhow::anyhow!("query hb consensus_state: {err:?}"))?;
	let Some(hb_set_id) = decode_current_set_id_scale(&hb_consensus) else {
		tracing::debug!(
			target: LOG_TARGET,
			"hb consensus state undecodable; skipping rotation catch-up",
		);
		return Ok(Vec::new());
	};

	if dest_set_id >= hb_set_id {
		return Ok(Vec::new());
	}

	let rotations: Vec<RotationProof> = proof_source
		.rotation_proofs_from(dest_set_id)
		.await
		.map_err(|err| anyhow::anyhow!("rotation_proofs_from({dest_set_id}): {err:?}"))?;
	if rotations.is_empty() {
		return Err(anyhow!("dest is lagging but no rotation proofs are cached on HB"));
	}

	// Drop the rotation that matches the current notification's set_id —
	// the main batch will submit that one via its `consensus_msg`. Without
	// this filter the same proof would land twice on the destination: once
	// here, once via the main batch.
	let rotations: Vec<RotationProof> =
		rotations.into_iter().filter(|r| Some(r.set_id) != current_set_id).collect();
	if rotations.is_empty() {
		return Ok(Vec::new());
	}

	tracing::info!(
		target: LOG_TARGET,
		dest = %dest_name,
		dest_set_id,
		hb_set_id,
		current_set_id = ?current_set_id,
		rotations = rotations.len(),
		"catching destination up across authority-set epochs",
	);

	let mut new_epochs: Vec<NewEpochEvent> = Vec::new();
	for chunk in rotations.chunks(MAX_CONSENSUS_PROOFS_PER_BATCH) {
		let batch: Vec<Message> = chunk
			.iter()
			.map(|r| {
				Message::Consensus(ConsensusMessage {
					consensus_proof: r.proof.clone(),
					consensus_state_id: BEEFY_CONSENSUS_STATE_ID,
					signer: dest.address(),
				})
			})
			.collect();
		let set_ids: Vec<u64> = chunk.iter().map(|r| r.set_id).collect();
		tracing::info!(
			target: "tesseract",
			?set_ids,
			msgs = batch.len(),
			"🛰️ Transmitting Mandatory Consensus Message to {dest_name}",
		);
		let result = dest.submit(batch, hyperbridge.state_machine_id().state_id).await?;
		new_epochs.extend(result.new_epochs);
	}

	Ok(new_epochs)
}

/// Static-friendly inputs for [`initialize`]. Lives next to the spawn
/// responsibility itself so cli.rs only has to assemble values; it
/// doesn't have to know how outbound stitches its tasks together.
pub struct OutboundInitParams {
	/// Plain HB substrate config — used to build per-task substrate
	/// clients (each task owns its own connection to avoid sharing one
	/// `OnlineClient` across multiple tokio tasks).
	pub hyperbridge_config: tesseract_substrate::SubstrateConfig,
	/// HB as an `IsmpProvider` for the outbound `run` loop subscription.
	pub hyperbridge_provider: Arc<dyn IsmpProvider>,
	/// Outbound-enabled destinations (chains with a configured signer).
	pub destinations: BTreeMap<StateMachine, Arc<dyn IsmpProvider>>,
	/// Full provider map (including HB) for downstream `client_map`
	/// lookups inside the spawned tasks.
	pub provider_clients: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	/// HB-side `ConsensusProofSource` used by both outbound (for
	/// destination delivery) and the fee-withdrawal task.
	pub proof_source: Arc<dyn ConsensusProofSource>,
	/// Messaging-side relayer config (profitability, deliver_failed,
	/// etc.).
	pub relayer_config: RelayerConfig,
	/// Local SQLite tracker.
	pub tx_payment: Arc<TransactionPayment>,
	/// When `true`, no fee-accumulation tasks are spawned and the
	/// outbound loop runs without forwarding receipts.
	pub fees_disabled: bool,
}

/// Spawn the full outbound pipeline:
///
/// 1. Per-destination [`crate::fee_accumulation`] task (skipped when `fees_disabled`).
/// 2. [`outbound_claim::run`](crate::outbound_claim::run) for the consensus delivery reward.
/// 3. [`run`] itself — the `ProofAccepted` subscriber that fans out to every destination.
///
/// All three are essential tasks: if any of them ends, the surrounding
/// [`TaskManager`] terminates the relayer process. cli.rs just assembles
/// the [`OutboundInitParams`] and calls this once.
pub async fn initialize(
	params: OutboundInitParams,
	task_manager: &polkadot_sdk::sc_service::TaskManager,
) -> Result<(), anyhow::Error> {
	let OutboundInitParams {
		hyperbridge_config,
		hyperbridge_provider,
		destinations,
		provider_clients,
		proof_source,
		relayer_config,
		tx_payment,
		fees_disabled,
	} = params;

	if destinations.is_empty() {
		tracing::info!(target: LOG_TARGET, "no outbound-enabled destinations; skipping outbound pipeline");
		return Ok(());
	}

	// Per-destination fee accumulation. Each task owns its own substrate
	// client so the connection isn't shared across tokio tasks.
	let mut fee_senders: HashMap<StateMachine, Sender<Vec<TxReceipt>>> = HashMap::new();
	if !fees_disabled {
		for (sm, provider) in &destinations {
			let (fee_sender, fee_receiver) = tokio::sync::mpsc::channel::<Vec<TxReceipt>>(512);
			fee_senders.insert(*sm, fee_sender);

			let hb_for_fees =
				SubstrateClient::<KeccakSubstrateChain>::new(hyperbridge_config.clone()).await?;
			let dest = provider.clone();
			let client_map = provider_clients.clone();
			let tx_payment_for_fees = tx_payment.clone();
			let name = format!("fee-acc-{}-{}", provider.name(), hyperbridge_provider.name());
			let span = tracing::info_span!(
				"fee_accumulation",
				chain = %provider.name(),
				hb = %hyperbridge_provider.name(),
			);
			task_manager.spawn_essential_handle().spawn_blocking(
				Box::leak(Box::new(name)),
				"fees",
				async move {
					tracing::trace!(target: LOG_TARGET, "task started");
					let res = crate::fee_accumulation(
						fee_receiver,
						dest,
						hb_for_fees,
						client_map,
						tx_payment_for_fees,
					)
					.await;
					tracing::error!(target: LOG_TARGET, ?res, "task terminated");
				}
				.instrument(span)
				.boxed(),
			);
		}
	}

	// Periodic task that claims outbound consensus delivery rewards from the DB.
	let claim_hb = SubstrateClient::<KeccakSubstrateChain>::new(hyperbridge_config.clone()).await?;
	let claim_destinations: HashMap<StateMachine, Arc<dyn IsmpProvider>> =
		destinations.iter().map(|(sm, p)| (*sm, p.clone())).collect();
	let claim_tx_payment = tx_payment.clone();
	let claim_name = format!("outbound-claim-{}", hyperbridge_provider.name());
	let claim_span = tracing::info_span!("outbound_claim", hb = %hyperbridge_provider.name());
	task_manager.spawn_essential_handle().spawn_blocking(
		Box::leak(Box::new(claim_name)),
		"outbound",
		async move {
			tracing::trace!(target: LOG_TARGET, "task started");
			let res =
				crate::outbound_claim::run(claim_hb, claim_destinations, Some(claim_tx_payment))
					.await;
			tracing::error!(target: LOG_TARGET, ?res, "task terminated");
		}
		.instrument(claim_span)
		.boxed(),
	);

	// Outbound request delivery reward claim task. Sibling of the consensus
	// claim task; periodic 600s timer, DB-backed, keyed by request commitment.
	let request_claim_hb =
		SubstrateClient::<KeccakSubstrateChain>::new(hyperbridge_config.clone()).await?;
	let request_claim_destinations: HashMap<StateMachine, Arc<dyn IsmpProvider>> =
		destinations.iter().map(|(sm, p)| (*sm, p.clone())).collect();
	let request_claim_tx_payment = tx_payment.clone();
	let request_claim_name = format!("outbound-request-claim-{}", hyperbridge_provider.name());
	let request_claim_span =
		tracing::info_span!("outbound_request_claim", hb = %hyperbridge_provider.name());
	task_manager.spawn_essential_handle().spawn_blocking(
		Box::leak(Box::new(request_claim_name)),
		"outbound",
		async move {
			tracing::trace!(target: LOG_TARGET, "task started");
			let res = crate::outbound_request_claim::run(
				request_claim_hb,
				request_claim_destinations,
				Some(request_claim_tx_payment),
			)
			.await;
			tracing::error!(target: LOG_TARGET, ?res, "task terminated");
		}
		.instrument(request_claim_span)
		.boxed(),
	);

	// Outbound fan-out itself.
	let outbound_name = format!("outbound-{}", hyperbridge_provider.name());
	let destinations_len = destinations.len();
	let outbound_tx_payment = tx_payment;
	let outbound_hb_sub =
		SubstrateClient::<KeccakSubstrateChain>::new(hyperbridge_config.clone()).await?;
	let outbound_span = tracing::info_span!(
		"outbound",
		hb = %hyperbridge_provider.name(),
		destinations = destinations_len,
	);
	task_manager.spawn_essential_handle().spawn_blocking(
		Box::leak(Box::new(outbound_name)),
		"outbound",
		async move {
			tracing::trace!(target: LOG_TARGET, "task started");
			let res = run(
				hyperbridge_provider,
				outbound_hb_sub,
				destinations,
				proof_source,
				relayer_config,
				provider_clients,
				fee_senders,
				Some(outbound_tx_payment),
			)
			.await;
			tracing::error!(target: LOG_TARGET, ?res, "task terminated");
		}
		.instrument(outbound_span)
		.boxed(),
	);

	tracing::trace!(
		target: LOG_TARGET,
		destinations = destinations_len,
		"initialized outbound pipeline (fee accumulation + claim + fan-out)",
	);
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use ismp::router::PostRequest;
	use std::sync::Arc;
	use tesseract_primitives::mocks::MockHost;

	const HB: StateMachine = StateMachine::Kusama(4009);
	const DEST_A: StateMachine = StateMachine::Evm(1);
	const DEST_B: StateMachine = StateMachine::Evm(42161);

	fn post_req(source: StateMachine, dest: StateMachine, nonce: u64) -> PostRequest {
		PostRequest {
			source,
			dest,
			nonce,
			from: vec![1],
			to: vec![2],
			timeout_timestamp: 0,
			body: vec![],
		}
	}

	fn mock(state_machine: StateMachine) -> MockHost<()> {
		MockHost::new((), 0, state_machine).with_address(vec![0xab])
	}

	/// Test double for `ConsensusProofSource`: `fetch` returns a sentinel blob,
	/// `rotation_proofs_from` hands back the empty vec (tests don't exercise
	/// the catch-up path — `MockHost::query_consensus_state` returns bytes
	/// that fail to decode as a BEEFY `ConsensusState`, so catch-up is
	/// short-circuited before `rotation_proofs_from` would fire).
	struct NoopProofSource;
	#[async_trait::async_trait]
	impl ConsensusProofSource for NoopProofSource {
		async fn fetch(&self, _height: u64) -> Result<Vec<u8>, anyhow::Error> {
			Ok(vec![0xcc])
		}
	}

	fn proof_source() -> Arc<dyn ConsensusProofSource> {
		Arc::new(NoopProofSource)
	}

	fn hb_id() -> StateMachineId {
		StateMachineId { state_id: HB, consensus_state_id: *b"BEEF" }
	}

	fn client_map_with(
		hb: Arc<dyn IsmpProvider>,
		dest: Arc<dyn IsmpProvider>,
	) -> HashMap<StateMachine, Arc<dyn IsmpProvider>> {
		let mut m: HashMap<StateMachine, Arc<dyn IsmpProvider>> = HashMap::new();
		m.insert(HB, hb);
		m.insert(dest.state_machine_id().state_id, dest);
		m
	}

	#[tokio::test]
	async fn skips_when_no_messages_and_not_mandatory() {
		let hb: Arc<dyn IsmpProvider> = Arc::new(mock(HB));
		let dest_mock = mock(DEST_A);
		let submitted_log = dest_mock.submitted.clone();
		let dest: Arc<dyn IsmpProvider> = Arc::new(dest_mock);
		let client_map = client_map_with(hb.clone(), dest.clone());

		submit_for_dest(
			OutboundEventContext {
				hyperbridge: hb,
				hb_state_machine_id: hb_id(),
				coprocessor: HB,
				relayer_config: RelayerConfig::default(),
				client_map,
				proof_source: proof_source(),
				events: Vec::new(),
				proof_bytes: vec![0xcc],
				is_mandatory: false,
				new_height: 100,
				new_set_id: None,
				incentivized: None,
			},
			DestinationContext { dest, fee_sender: None, claim_tx_payment: None },
		)
		.await
		.unwrap();

		assert!(
			submitted_log.lock().unwrap().is_empty(),
			"should not submit without messages or rotation"
		);
	}

	#[tokio::test]
	async fn submits_consensus_only_when_mandatory_no_messages() {
		let hb: Arc<dyn IsmpProvider> = Arc::new(mock(HB));
		let dest_mock = mock(DEST_A);
		let submitted_log = dest_mock.submitted.clone();
		let dest: Arc<dyn IsmpProvider> = Arc::new(dest_mock);
		let client_map = client_map_with(hb.clone(), dest.clone());

		submit_for_dest(
			OutboundEventContext {
				hyperbridge: hb,
				hb_state_machine_id: hb_id(),
				coprocessor: HB,
				relayer_config: RelayerConfig::default(),
				client_map,
				proof_source: proof_source(),
				events: Vec::new(),
				proof_bytes: vec![0xcc],
				is_mandatory: true,
				new_height: 100,
				new_set_id: Some(1),
				incentivized: None,
			},
			DestinationContext { dest, fee_sender: None, claim_tx_payment: None },
		)
		.await
		.unwrap();

		let submissions = submitted_log.lock().unwrap().clone();
		assert_eq!(submissions.len(), 1);
		assert_eq!(submissions[0].len(), 1);
		assert!(matches!(submissions[0][0], Message::Consensus(_)));
	}

	#[tokio::test]
	async fn events_for_other_destinations_are_ignored() {
		let hb: Arc<dyn IsmpProvider> = Arc::new(mock(HB));
		let dest_mock = mock(DEST_A);
		let submitted_log = dest_mock.submitted.clone();
		let dest: Arc<dyn IsmpProvider> = Arc::new(dest_mock);
		let client_map = client_map_with(hb.clone(), dest.clone());

		// Only DEST_B-targeted events; messaging-only proof for DEST_A.
		let events = vec![Event::PostRequest(post_req(HB, DEST_B, 1))];

		submit_for_dest(
			OutboundEventContext {
				hyperbridge: hb,
				hb_state_machine_id: hb_id(),
				coprocessor: HB,
				relayer_config: RelayerConfig::default(),
				client_map,
				proof_source: proof_source(),
				events,
				proof_bytes: vec![0xcc],
				is_mandatory: false,
				new_height: 100,
				new_set_id: None,
				incentivized: None,
			},
			DestinationContext { dest, fee_sender: None, claim_tx_payment: None },
		)
		.await
		.unwrap();

		assert!(
			submitted_log.lock().unwrap().is_empty(),
			"DEST_A should see nothing when events target DEST_B"
		);
	}

	use tesseract_primitives::Query;

	/// Build a PostRequest with `source` so we can produce a matching receipt.
	fn build_post(source: StateMachine, nonce_marker: u8) -> PostRequest {
		PostRequest {
			source,
			dest: DEST_A,
			nonce: nonce_marker as u64,
			from: vec![0x77; 8],
			to: vec![0x88; 20],
			timeout_timestamp: 0,
			body: vec![nonce_marker],
		}
	}

	fn request_receipt_for(req: &PostRequest, height: u64) -> TxReceipt {
		let commitment = hash_request::<Hasher>(&Request::Post(req.clone()));
		TxReceipt {
			query: Query {
				source_chain: req.source,
				dest_chain: req.dest,
				nonce: req.nonce,
				commitment,
			},
			height,
		}
	}

	#[test]
	fn collect_claims_only_hyperbridge_originated() {
		let hb_req_a = build_post(HB, 0x11);
		let user_req = build_post(DEST_B, 0x22);
		let hb_req_b = build_post(HB, 0x33);
		let batch_requests = vec![hb_req_a.clone(), user_req.clone(), hb_req_b.clone()];
		let receipts = vec![
			request_receipt_for(&hb_req_a, 100),
			request_receipt_for(&user_req, 101),
			request_receipt_for(&hb_req_b, 102),
		];

		let claims = collect_hyperbridge_request_claims(HB, &batch_requests, &receipts);

		assert_eq!(claims.len(), 2, "only hyperbridge-originated requests forwarded");
		assert!(claims.iter().all(|c| c.request.source == HB));
		let heights: std::collections::BTreeSet<u64> =
			claims.iter().map(|c| c.delivery_height).collect();
		assert_eq!(heights, [100u64, 102u64].into_iter().collect());
	}

	#[test]
	fn collect_claims_empty_receipts_is_empty() {
		let hb_req = build_post(HB, 0x11);
		let claims = collect_hyperbridge_request_claims(HB, &[hb_req], &[]);
		assert!(claims.is_empty());
	}

	#[test]
	fn collect_claims_no_hyperbridge_requests_is_empty() {
		let user_req = build_post(DEST_B, 0x11);
		let receipts = vec![request_receipt_for(&user_req, 100)];
		let claims = collect_hyperbridge_request_claims(HB, &[user_req], &receipts);
		assert!(claims.is_empty());
	}

	#[test]
	fn retain_incentivized_drops_unallowlisted_hyperbridge_requests() {
		let allowed_module = vec![0xAA, 0xBB];
		let other_module = vec![0xCC, 0xDD];
		let allowlist: BTreeSet<Vec<u8>> = [allowed_module.clone()].into_iter().collect();

		let mut allowed = post_req(HB, DEST_A, 1);
		allowed.from = allowed_module;
		let mut unallowlisted = post_req(HB, DEST_A, 2);
		unallowlisted.from = other_module.clone();
		let mut user_originated = post_req(DEST_B, DEST_A, 3);
		user_originated.from = other_module;

		let mut events = vec![
			Event::PostRequest(allowed.clone()),
			Event::PostRequest(unallowlisted.clone()),
			Event::PostRequest(user_originated.clone()),
		];

		retain_incentivized_requests(&mut events, HB, Some(&allowlist));

		let nonces: Vec<u64> = events
			.iter()
			.filter_map(|ev| match ev {
				Event::PostRequest(p) => Some(p.nonce),
				_ => None,
			})
			.collect();
		assert_eq!(nonces, vec![1, 3]);
	}

	#[test]
	fn retain_incentivized_none_is_noop() {
		let mut events = vec![
			Event::PostRequest(post_req(HB, DEST_A, 1)),
			Event::PostRequest(post_req(HB, DEST_A, 2)),
		];
		retain_incentivized_requests(&mut events, HB, None);
		assert_eq!(events.len(), 2);
	}

	#[test]
	fn retain_incentivized_passes_non_request_events() {
		let allowlist: BTreeSet<Vec<u8>> = BTreeSet::new();
		// A non-`PostRequest` event (e.g. a state-machine update) must pass
		// through untouched regardless of the allowlist.
		let mut events = vec![Event::StateMachineUpdated(StateMachineUpdated {
			state_machine_id: hb_id(),
			latest_height: 1,
		})];
		retain_incentivized_requests(&mut events, HB, Some(&allowlist));
		assert_eq!(events.len(), 1);
	}

	#[test]
	fn collect_claims_drops_receipts_with_no_matching_batch_request() {
		// A receipt whose commitment is HB-sourced but whose request wasn't in
		// the batch produces no claim (shouldn't happen in practice).
		let orphan = build_post(HB, 0x11);
		let receipts = vec![request_receipt_for(&orphan, 100)];
		// orphan is NOT in batch_requests.
		let claims = collect_hyperbridge_request_claims(HB, &[], &receipts);
		assert!(claims.is_empty());
	}
}
