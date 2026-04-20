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

use std::{collections::BTreeMap, sync::Arc};

use futures::{stream::FuturesUnordered, StreamExt};
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::Event,
	host::StateMachine,
	messaging::{
		hash_request, hash_response, ConsensusMessage, Message, Proof, RequestMessage,
		ResponseMessage,
	},
	router::{Request, RequestResponse, Response},
};
use tesseract_primitives::{Hasher, IsmpProvider, ProofAccepted, Query, StateMachineUpdated};

use crate::provider::ConsensusProofSource;

/// BEEFY `ConsensusStateId` — matches the solidity `BEEFY_CONSENSUS_ID` and
/// `pallet_beefy_consensus_proofs::BEEFY_CONSENSUS_ID`.
const BEEFY_CONSENSUS_STATE_ID: [u8; 4] = *b"BEEF";

pub async fn run(
	hyperbridge: Arc<dyn IsmpProvider>,
	destinations: BTreeMap<StateMachine, Arc<dyn IsmpProvider>>,
	proof_source: Arc<dyn ConsensusProofSource>,
) -> Result<(), anyhow::Error> {
	let hb_state_machine_id = hyperbridge.state_machine_id();
	let hb_name = hyperbridge.name();
	let mut stream = hyperbridge.proof_accepted_notification().await?;
	let mut cursor: u64 = hyperbridge.initial_height();

	tracing::info!("[outbound] {hb_name}: subscribed to ProofAccepted, cursor={cursor}");

	while let Some(item) = stream.next().await {
		let accepted: ProofAccepted = match item {
			Ok(ev) => ev,
			Err(err) => {
				tracing::error!("[outbound] {hb_name}: proof_accepted stream: {err:?}");
				continue;
			},
		};

		let new_height = accepted.height;
		let is_mandatory = accepted.new_set_id.is_some();

		let synth = StateMachineUpdated {
			state_machine_id: hb_state_machine_id,
			latest_height: new_height,
		};

		let events = match hyperbridge.query_ismp_events(cursor, synth).await {
			Ok(events) => events,
			Err(err) => {
				tracing::error!(
					"[outbound] {hb_name}: query_ismp_events({cursor}..={new_height}): {err:?}"
				);
				continue;
			},
		};

		let proof_bytes = match proof_source.fetch(new_height, accepted.new_set_id).await {
			Ok(bytes) => bytes,
			Err(err) => {
				tracing::error!(
					"[outbound] {hb_name}: proof fetch (h={new_height} set={:?}): {err:?}",
					accepted.new_set_id,
				);
				continue;
			},
		};

		tracing::info!(
			"[outbound] {hb_name}: ProofAccepted h={new_height} set={:?} mandatory={is_mandatory} events={}",
			accepted.new_set_id,
			events.len(),
		);

		let mut tasks = FuturesUnordered::new();
		for dest in destinations.values() {
			tasks.push(submit_for_dest(
				hyperbridge.clone(),
				dest.clone(),
				events.clone(),
				proof_bytes.clone(),
				is_mandatory,
				new_height,
				hb_state_machine_id,
			));
		}
		while let Some(res) = tasks.next().await {
			if let Err(err) = res {
				tracing::error!("[outbound] submit_for_dest: {err:?}");
			}
		}

		cursor = new_height;
	}

	Err(anyhow::anyhow!("[outbound] {hb_name}: proof_accepted stream ended"))
}

async fn submit_for_dest(
	hyperbridge: Arc<dyn IsmpProvider>,
	dest: Arc<dyn IsmpProvider>,
	events: Vec<Event>,
	proof_bytes: Vec<u8>,
	is_mandatory: bool,
	new_height: u64,
	hb_state_machine_id: StateMachineId,
) -> Result<(), anyhow::Error> {
	let dest_state_machine = dest.state_machine_id().state_id;
	let dest_name = dest.name();
	let hb_name = hyperbridge.name();

	let (requests, responses) = partition_events_for_dest(&events, dest_state_machine);
	let has_messages = !requests.is_empty() || !responses.is_empty();

	// Messaging-only proofs with nothing for this chain — skip. Rotation
	// proofs (mandatory) must propagate even without user messages so future
	// messaging proofs remain verifiable on the destination.
	if !has_messages && !is_mandatory {
		return Ok(());
	}

	let mut batch: Vec<Message> = vec![Message::Consensus(ConsensusMessage {
		consensus_proof: proof_bytes,
		consensus_state_id: BEEFY_CONSENSUS_STATE_ID,
		signer: dest.address(),
	})];

	if has_messages {
		let height = StateMachineHeight { id: hb_state_machine_id, height: new_height };
		let hb_signer = hyperbridge.address();

		if !requests.is_empty() {
			let keys: Vec<_> = requests
				.iter()
				.map(|req| Query {
					source_chain: req.source,
					dest_chain: req.dest,
					nonce: req.nonce,
					commitment: hash_request::<Hasher>(&Request::Post(req.clone())),
				})
				.collect();
			match hyperbridge.query_requests_proof(new_height, keys, dest_state_machine).await {
				Ok(proof) => batch.push(Message::Request(RequestMessage {
					requests,
					proof: Proof { height, proof },
					signer: hb_signer.clone(),
				})),
				Err(err) => tracing::error!(
					"[outbound] {hb_name}->{dest_name}: request proof failed: {err:?}"
				),
			}
		}

		if !responses.is_empty() {
			let keys: Vec<_> = responses
				.iter()
				.map(|res| Query {
					source_chain: res.post.source,
					dest_chain: res.post.dest,
					nonce: res.post.nonce,
					commitment: hash_response::<Hasher>(&Response::Post(res.clone())),
				})
				.collect();
			match hyperbridge.query_responses_proof(new_height, keys, dest_state_machine).await {
				Ok(proof) => batch.push(Message::Response(ResponseMessage {
					datagram: RequestResponse::Response(
						responses.into_iter().map(Response::Post).collect(),
					),
					proof: Proof { height, proof },
					signer: hb_signer,
				})),
				Err(err) => tracing::error!(
					"[outbound] {hb_name}->{dest_name}: response proof failed: {err:?}"
				),
			}
		}
	}

	// If all the message-proof queries failed, we may be left with only the
	// consensus entry — only worth sending on mandatory (rotation) proofs.
	if batch.len() == 1 && !is_mandatory {
		return Ok(());
	}

	tracing::info!(
		"[outbound] {hb_name}->{dest_name}: submit {} msgs at h={new_height} mandatory={is_mandatory}",
		batch.len(),
	);
	dest.submit(batch, hb_state_machine_id.state_id).await?;
	Ok(())
}

fn partition_events_for_dest(
	events: &[Event],
	dest: StateMachine,
) -> (Vec<ismp::router::PostRequest>, Vec<ismp::router::PostResponse>) {
	let mut requests = Vec::new();
	let mut responses = Vec::new();
	for event in events {
		match event {
			Event::PostRequest(req) if req.dest == dest => requests.push(req.clone()),
			Event::PostResponse(res) if res.dest_chain() == dest => responses.push(res.clone()),
			_ => {},
		}
	}
	(requests, responses)
}
