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
	config::RelayerConfig, ConsensusProofSource, IsmpProvider, ProofAccepted, RotationProof,
	StateMachineUpdated, TxReceipt, BEEFY_CONSENSUS_STATE_ID,
};
use tokio::sync::mpsc::Sender;
use tracing::Instrument;

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
			tasks.push(
				submit_for_dest(
					hyperbridge.clone(),
					dest.clone(),
					events.clone(),
					proof_bytes.clone(),
					is_mandatory,
					new_height,
					hb_state_machine_id,
					relayer_config.clone(),
					coprocessor,
					client_map.clone(),
					fee_sender,
					proof_source.clone(),
				)
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

#[allow(clippy::too_many_arguments)]
async fn submit_for_dest(
	hyperbridge: Arc<dyn IsmpProvider>,
	dest: Arc<dyn IsmpProvider>,
	events: Vec<Event>,
	proof_bytes: Vec<u8>,
	is_mandatory: bool,
	new_height: u64,
	hb_state_machine_id: StateMachineId,
	relayer_config: RelayerConfig,
	coprocessor: StateMachine,
	client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	fee_sender: Option<Sender<Vec<TxReceipt>>>,
	proof_source: Arc<dyn ConsensusProofSource>,
) -> Result<(), anyhow::Error> {
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

	// Forward receipts for fee accumulation (best-effort — channel may be
	// closed if the relayer is shutting down).
	if let (Some(sender), false) = (fee_sender, result.receipts.is_empty()) {
		if let Err(err) = sender.send(result.receipts).await {
			tracing::warn!(target: LOG_TARGET, ?err, "fee-accumulation channel send failed");
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
			hb,
			dest,
			Vec::new(),
			vec![0xcc],
			false,
			100,
			hb_id(),
			RelayerConfig::default(),
			HB,
			client_map,
			None,
			proof_source(),
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
			hb,
			dest,
			Vec::new(),
			vec![0xcc],
			true,
			100,
			hb_id(),
			RelayerConfig::default(),
			HB,
			client_map,
			None,
			proof_source(),
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
			hb,
			dest,
			events,
			vec![0xcc],
			false,
			100,
			hb_id(),
			RelayerConfig::default(),
			HB,
			client_map,
			None,
			proof_source(),
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
			hb,
			dest,
			events,
			vec![0xcc],
			false,
			100,
			hb_id(),
			RelayerConfig::default(),
			HB,
			client_map,
			None,
			proof_source(),
		)
		.await
		.unwrap();

		assert!(
			submitted_log.lock().unwrap().is_empty(),
			"DEST_A should see nothing when events target DEST_B"
		);
	}
}
