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

use futures::{stream::FuturesUnordered, StreamExt};
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::Event,
	host::StateMachine,
	messaging::{ConsensusMessage, Message},
};
use tesseract_messaging::events::translate_events_to_messages;
use tesseract_primitives::{
	config::RelayerConfig, IsmpProvider, ProofAccepted, StateMachineUpdated, TxReceipt,
};
use tokio::sync::mpsc::Sender;
use tracing::Instrument;

use crate::provider::ConsensusProofSource;

/// BEEFY `ConsensusStateId` — matches the solidity `BEEFY_CONSENSUS_ID` and
/// `pallet_beefy_consensus_proofs::BEEFY_CONSENSUS_ID`.
const BEEFY_CONSENSUS_STATE_ID: [u8; 4] = *b"BEEF";

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
	let mut stream = hyperbridge.proof_accepted_notification().await?;
	let mut cursor: u64 = hyperbridge.initial_height();

	tracing::info!(target: "tesseract", cursor, "subscribed to ProofAccepted");

	while let Some(item) = stream.next().await {
		let accepted: ProofAccepted = match item {
			Ok(ev) => ev,
			Err(err) => {
				tracing::error!(target: "tesseract", ?err, "proof_accepted stream error");
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
				tracing::error!(target: "tesseract", cursor, to = new_height, ?err, "query_ismp_events failed",);
				continue;
			},
		};

		let proof_bytes = match proof_source.fetch(new_height).await {
			Ok(bytes) => bytes,
			Err(err) => {
				tracing::error!(
					target: "tesseract", height = new_height,
					set_id = ?new_set_id,
					?err,
					"proof fetch failed",
				);
				continue;
			},
		};

		tracing::info!(
			target: "tesseract", height = new_height,
			set_id = ?new_set_id,
			mandatory = is_mandatory,
			events = events.len(),
			"ProofAccepted",
		);

		let mut tasks = FuturesUnordered::new();
		for dest in destinations.values() {
			let fee_sender = fee_senders.get(&dest.state_machine_id().state_id).cloned();
			let dest_span = tracing::info_span!(
				"dest",
				chain = %dest.name(),
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
				)
				.instrument(dest_span),
			);
		}
		while let Some(res) = tasks.next().await {
			if let Err(err) = res {
				tracing::error!(target: "tesseract", ?err, "submit_for_dest failed");
			}
		}

		cursor = new_height;
	}

	Err(anyhow::anyhow!("proof_accepted stream ended"))
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
) -> Result<(), anyhow::Error> {
	let dest_state_machine = dest.state_machine_id().state_id;

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
		tracing::trace!(target: "tesseract", "skipping — no events for this chain, not mandatory");
		return Ok(());
	}

	let mut batch: Vec<Message> = vec![Message::Consensus(ConsensusMessage {
		consensus_proof: proof_bytes,
		consensus_state_id: BEEFY_CONSENSUS_STATE_ID,
		signer: dest.address(),
	})];

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
		)
		.await
		{
			Ok((deliverable, unprofitable)) => {
				if !unprofitable.is_empty() {
					tracing::debug!(target: "tesseract", dropped = unprofitable.len(), "unprofitable messages dropped");
				}
				batch.extend(deliverable);
			},
			Err(err) => tracing::error!(target: "tesseract", ?err, "translate_events_to_messages failed"),
		}
	}

	// If translate returned no deliverable messages we may be left with only
	// the consensus entry — only worth sending on mandatory (rotation) proofs.
	if batch.len() == 1 && !is_mandatory {
		tracing::trace!(target: "tesseract", "skipping — consensus-only batch, not mandatory");
		return Ok(());
	}

	tracing::info!(target: "tesseract", msgs = batch.len(), "submitting batch");
	// `submit` transparently picks the right transport — EVM destinations
	// whose handler supports IHandlerV2 dispatch the whole batch as a single
	// `batchCall(bytes[])` tx; everything else uses the legacy serial path.
	let result = dest.submit(batch, hb_state_machine_id.state_id).await?;

	// Forward receipts for fee accumulation (best-effort — channel may be
	// closed if the relayer is shutting down).
	if let (Some(sender), false) = (fee_sender, result.receipts.is_empty()) {
		if let Err(err) = sender.send(result.receipts).await {
			tracing::warn!(target: "tesseract", ?err, "fee-accumulation channel send failed");
		}
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
		)
		.await
		.unwrap();

		assert!(
			submitted_log.lock().unwrap().is_empty(),
			"DEST_A should see nothing when events target DEST_B"
		);
	}
}
