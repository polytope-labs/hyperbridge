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
	collections::{BTreeMap, HashMap},
	sync::Arc,
};

use anyhow::anyhow;
use codec::Decode;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::Event,
	host::StateMachine,
	messaging::{ConsensusMessage, Message},
};
use tesseract_primitives::{
	config::RelayerConfig, ConsensusProofSource, IsmpProvider, PendingConsensusDeliveryClaim,
	ProofAccepted, RotationProof, StateMachineUpdated, TxReceipt, BEEFY_CONSENSUS_STATE_ID,
};
use tokio::sync::mpsc::Sender;
use tracing::Instrument;
use transaction_fees::TransactionPayment;

use crate::events::translate_events_to_messages;

/// Log/tracing target for the outbound pipeline.
const LOG_TARGET: &str = concat!("tesseract-messaging", "-outbound");

/// Cap on consensus proofs bundled into a single `submit` call. EVM destinations
/// enforce calldata and gas limits that a large rotation catch-up would blow
/// through — three BEEFY proofs is the empirical ceiling that still fits under
/// mainnet block gas on the hottest destinations.
const MAX_CONSENSUS_PROOFS_PER_BATCH: usize = 3;

pub async fn run(
	hyperbridge: Arc<dyn IsmpProvider>,
	destinations: BTreeMap<StateMachine, Arc<dyn IsmpProvider>>,
	proof_source: Arc<dyn ConsensusProofSource>,
	relayer_config: RelayerConfig,
	client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	fee_senders: HashMap<StateMachine, Sender<Vec<TxReceipt>>>,
	claim_sender: Option<Sender<Vec<PendingConsensusDeliveryClaim>>>,
	claim_tx_payment: Option<Arc<TransactionPayment>>,
) -> Result<(), anyhow::Error> {
	let hb_state_machine_id = hyperbridge.state_machine_id();
	let coprocessor = hb_state_machine_id.state_id;
	let source_name = hyperbridge.name();
	let mut stream = hyperbridge.proof_accepted_notification().await?;
	let mut cursor: u64 = hyperbridge.initial_height();

	tracing::info!(target: LOG_TARGET, source = %source_name, cursor, "subscribed to ProofAccepted");

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
			"ProofAccepted",
		);

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
			};
			let dest_ctx = DestinationContext {
				dest: dest.clone(),
				fee_sender,
				claim_sender: claim_sender.clone(),
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
}

/// Per-destination args. Splits cleanly from [`OutboundEventContext`] so
/// the cycle context can be cloned cheaply once and passed to every
/// destination, with only the dest-specific bits varying per call.
struct DestinationContext {
	dest: Arc<dyn IsmpProvider>,
	fee_sender: Option<Sender<Vec<TxReceipt>>>,
	claim_sender: Option<Sender<Vec<PendingConsensusDeliveryClaim>>>,
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
	} = event_ctx;
	let DestinationContext { dest, fee_sender, claim_sender, claim_tx_payment } = dest_ctx;
	let dest_state_machine = dest.state_machine_id().state_id;
	let dest_name = dest.name();
	// Bring the destination's BEEFY light client up to HB's current
	// authority-set id before submitting the current update. A messaging
	// proof whose set_id is ahead of the destination's locally-known
	// authorities gets rejected by the BEEFY verifier, so any missing
	// rotations have to land first. Best-effort: if we can't read the
	// destination's consensus state we assume it's current and fall through.
	if let Err(err) = catch_up_rotations(&hyperbridge, &dest, &proof_source).await {
		tracing::warn!(
			target: LOG_TARGET,
			dest = %dest_name,
			?err,
			"rotation catch-up failed; proceeding with current update",
		);
	}

	// Only events relevant to this destination matter; skip the RPC-heavy
	// translate_events_to_messages entirely when there's nothing to do.
	let has_events_for_dest = events.iter().any(|ev| {
		matches!(ev,
		Event::PostRequest(req) if req.dest == dest_state_machine) ||
			matches!(ev,
		Event::PostResponse(res) if res.dest_chain() == dest_state_machine)
	});

	if !has_events_for_dest && !is_mandatory {
		// Messaging-only proof with nothing for this chain — skip. Rotation
		// proofs (mandatory) must propagate even without user messages so future
		// messaging proofs remain verifiable on the destination.
		tracing::trace!(target: LOG_TARGET, dest = %dest_name, "skipping — no events for this chain, not mandatory");
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
			Err(err) =>
				tracing::error!(target: LOG_TARGET, ?err, dest = %dest_name, "translate_events_to_messages failed"),
		}
	}

	// If translate returned no deliverable messages we may be left with only
	// the consensus entry — only worth sending on mandatory (rotation) proofs.
	if batch.len() == 1 && !is_mandatory {
		tracing::trace!(target: LOG_TARGET,dest = %dest_name, "skipping — consensus-only batch, not mandatory");
		return Ok(());
	}

	tracing::info!(target: LOG_TARGET, msgs = batch.len(), "🛰️ Transmitting ismp messages to {dest_name}");
	// `submit` transparently picks the right transport — EVM destinations
	// whose handler supports IHandlerV2 dispatch the whole batch as a single
	// `batchCall(bytes[])` tx; everything else uses the legacy serial path.
	let result = dest.submit(batch, hb_state_machine_id.state_id).await?;

	// Forward receipts for fee accumulation (best-effort, channel may be
	// closed if the relayer is shutting down).
	if let (Some(sender), false) = (fee_sender, result.receipts.is_empty()) {
		if let Err(err) = sender.send(result.receipts).await {
			tracing::warn!(target: LOG_TARGET, ?err, "fee-accumulation channel send failed");
		}
	}

	// Forward a claim for the outbound consensus delivery reward, one per
	// `NewEpoch(set_id, self_address)` log in the receipt. A single tx can
	// batch multiple consensus messages (catch-up) and emit several logs;
	// each set_id earns its own reward. Loser relayers see an empty
	// `new_epochs` and skip the claim entirely (no wasted on-chain
	// extrinsic). We persist before sending so a crash between here and
	// the channel push survives: the claim task reads the DB on every
	// trigger and replays anything pending.
	if let Some(sender) = claim_sender {
		if !result.new_epochs.is_empty() {
			// `delivery_height` is the destination's own current finalized
			// height — used by the claim task to wait_for_state_machine_update
			// on HB. Any post-tx height works since `_epochs[set_id]` is set
			// once and persists; we use finalized so HB has a real chance to
			// have verified it.
			let delivery_height = match dest.query_finalized_height().await {
				Ok(h) => h,
				Err(err) => {
					tracing::warn!(
						target: LOG_TARGET,
						?err,
						dest = %dest_name,
						"query_finalized_height failed; skipping outbound consensus claim",
					);
					return Ok(());
				},
			};

			if let Some(tx_payment) = &claim_tx_payment {
				if let Err(err) = tx_payment
					.insert_pending_rotation_claims(
						&dest_state_machine.to_string(),
						&result.new_epochs,
						delivery_height,
					)
					.await
				{
					tracing::warn!(
						target: LOG_TARGET,
						?err,
						dest = %dest_name,
						delivery_height,
						set_ids = ?result.new_epochs,
						"failed to persist outbound-consensus claims; the channel send may \
						 still succeed but the claims will not survive a restart",
					);
				}
			}

			// Send the whole batch as one trigger: the claim task merges
			// it with whatever's persisted in the DB, dedupes, then
			// filters against `OutboundConsensusRotationsClaimed` on
			// Hyperbridge before submitting. A single send per receipt
			// cycle keeps the channel bounded even when a catch-up
			// emits several rotations in one tx.
			let claims: Vec<PendingConsensusDeliveryClaim> = result
				.new_epochs
				.iter()
				.map(|&set_id| PendingConsensusDeliveryClaim {
					destination: dest_state_machine,
					delivery_height,
					set_id,
				})
				.collect();
			let set_ids = result.new_epochs.clone();
			if let Err(err) = sender.send(claims).await {
				tracing::warn!(
					target: LOG_TARGET,
					?err,
					dest = %dest_name,
					delivery_height,
					?set_ids,
					"outbound-consensus claim channel send failed",
				);
			}
		}
	}

	Ok(())
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
/// This is a best-effort catch-up: any failure (stale offchain proofs, submit
/// reverts on one chunk) is logged and surfaced to the caller, which then
/// decides whether to still attempt the current update.
async fn catch_up_rotations(
	hyperbridge: &Arc<dyn IsmpProvider>,
	dest: &Arc<dyn IsmpProvider>,
	proof_source: &Arc<dyn ConsensusProofSource>,
) -> Result<(), anyhow::Error> {
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
		return Ok(());
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
		return Ok(());
	};

	if dest_set_id >= hb_set_id {
		return Ok(());
	}

	let rotations: Vec<RotationProof> = proof_source
		.rotation_proofs_from(dest_set_id)
		.await
		.map_err(|err| anyhow::anyhow!("rotation_proofs_from({dest_set_id}): {err:?}"))?;
	if rotations.is_empty() {
		return Err(anyhow!("dest is lagging but no rotation proofs are cached on HB"));
	}

	tracing::info!(
		target: LOG_TARGET,
		dest = %dest_name,
		dest_set_id,
		hb_set_id,
		rotations = rotations.len(),
		"catching destination up across authority-set epochs",
	);

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
			target: LOG_TARGET,
			?set_ids,
			msgs = batch.len(),
			"🛰️ Transmitting Mandatory Consensus Message to {dest_name}",
		);
		dest.submit(batch, hyperbridge.state_machine_id().state_id).await?;
	}

	Ok(())
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
	use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};

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
			let (fee_sender, fee_receiver) = tokio::sync::mpsc::channel::<Vec<TxReceipt>>(64);
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

	// Outbound consensus delivery reward claim task. Reads pending rows
	// from the DB on every trigger and processes them, so no startup
	// replay is needed in the caller.
	let (claim_sender, claim_receiver) =
		tokio::sync::mpsc::channel::<Vec<PendingConsensusDeliveryClaim>>(64);
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
			let res = crate::outbound_claim::run(
				claim_hb,
				claim_destinations,
				claim_receiver,
				Some(claim_tx_payment),
			)
			.await;
			tracing::error!(target: LOG_TARGET, ?res, "task terminated");
		}
		.instrument(claim_span)
		.boxed(),
	);

	// Outbound fan-out itself.
	let outbound_name = format!("outbound-{}", hyperbridge_provider.name());
	let destinations_len = destinations.len();
	let outbound_tx_payment = tx_payment;
	task_manager.spawn_essential_handle().spawn_blocking(
		Box::leak(Box::new(outbound_name)),
		"outbound",
		async move {
			tracing::trace!(target: LOG_TARGET, "task started");
			let res = run(
				hyperbridge_provider,
				destinations,
				proof_source,
				relayer_config,
				provider_clients,
				fee_senders,
				Some(claim_sender),
				Some(outbound_tx_payment),
			)
			.await;
			tracing::error!(target: LOG_TARGET, ?res, "task terminated");
		}
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
	use ismp::router::{PostRequest, PostResponse};
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

	/// Build a PostResponse whose `dest_chain()` is `response_to`. A response
	/// heads back to the *source* of the original request, so we put
	/// `response_to` in the inner post's `source` field.
	fn post_res(
		response_to: StateMachine,
		request_was_for: StateMachine,
		nonce: u64,
	) -> PostResponse {
		PostResponse {
			post: post_req(response_to, request_was_for, nonce),
			response: vec![9],
			timeout_timestamp: 0,
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
			},
			DestinationContext {
				dest,
				fee_sender: None,
				claim_sender: None,
				claim_tx_payment: None,
			},
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
			},
			DestinationContext {
				dest,
				fee_sender: None,
				claim_sender: None,
				claim_tx_payment: None,
			},
		)
		.await
		.unwrap();

		let submissions = submitted_log.lock().unwrap().clone();
		assert_eq!(submissions.len(), 1);
		assert_eq!(submissions[0].len(), 1);
		assert!(matches!(submissions[0][0], Message::Consensus(_)));
	}

	#[tokio::test]
	async fn submits_full_batch_when_messages_present() {
		let hb: Arc<dyn IsmpProvider> = Arc::new(mock(HB));
		let dest_mock = mock(DEST_A);
		let submitted_log = dest_mock.submitted.clone();
		let dest: Arc<dyn IsmpProvider> = Arc::new(dest_mock);
		let client_map = client_map_with(hb.clone(), dest.clone());

		let events = vec![
			Event::PostRequest(post_req(HB, DEST_A, 1)),
			Event::PostResponse(post_res(DEST_A, HB, 2)),
		];

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
			},
			DestinationContext {
				dest,
				fee_sender: None,
				claim_sender: None,
				claim_tx_payment: None,
			},
		)
		.await
		.unwrap();

		let submissions = submitted_log.lock().unwrap().clone();
		assert_eq!(submissions.len(), 1);
		assert_eq!(submissions[0].len(), 3, "consensus + request + response");
		assert!(matches!(submissions[0][0], Message::Consensus(_)));
		assert!(matches!(submissions[0][1], Message::Request(_)));
		assert!(matches!(submissions[0][2], Message::Response(_)));
	}

	#[tokio::test]
	async fn events_for_other_destinations_are_ignored() {
		let hb: Arc<dyn IsmpProvider> = Arc::new(mock(HB));
		let dest_mock = mock(DEST_A);
		let submitted_log = dest_mock.submitted.clone();
		let dest: Arc<dyn IsmpProvider> = Arc::new(dest_mock);
		let client_map = client_map_with(hb.clone(), dest.clone());

		// Only DEST_B-targeted events; messaging-only proof for DEST_A.
		let events = vec![
			Event::PostRequest(post_req(HB, DEST_B, 1)),
			Event::PostResponse(post_res(DEST_B, HB, 2)),
		];

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
			},
			DestinationContext {
				dest,
				fee_sender: None,
				claim_sender: None,
				claim_tx_payment: None,
			},
		)
		.await
		.unwrap();

		assert!(
			submitted_log.lock().unwrap().is_empty(),
			"DEST_A should see nothing when events target DEST_B"
		);
	}
}
