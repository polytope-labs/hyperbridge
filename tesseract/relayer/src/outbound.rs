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

		let proof_bytes = match proof_source.fetch(new_height).await {
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

	#[test]
	fn partition_filters_by_destination() {
		let events = vec![
			Event::PostRequest(post_req(HB, DEST_A, 1)),
			Event::PostRequest(post_req(HB, DEST_B, 2)),
			Event::PostResponse(post_res(HB, DEST_A, 3)),
			Event::PostResponse(post_res(DEST_A, HB, 4)),
		];

		let (reqs_a, res_a) = partition_events_for_dest(&events, DEST_A);
		assert_eq!(reqs_a.len(), 1);
		assert_eq!(reqs_a[0].nonce, 1);
		assert_eq!(res_a.len(), 1);
		assert_eq!(res_a[0].post.nonce, 4);

		let (reqs_b, res_b) = partition_events_for_dest(&events, DEST_B);
		assert_eq!(reqs_b.len(), 1);
		assert_eq!(reqs_b[0].nonce, 2);
		assert!(res_b.is_empty());
	}

	#[tokio::test]
	async fn skips_when_no_messages_and_not_mandatory() {
		let hb = Arc::new(mock(HB));
		let dest = Arc::new(mock(DEST_A));

		submit_for_dest(hb.clone(), dest.clone(), Vec::new(), vec![0xcc], false, 100, hb_id())
			.await
			.unwrap();

		assert!(dest.submissions().is_empty(), "should not submit without messages or rotation");
	}

	#[tokio::test]
	async fn submits_consensus_only_when_mandatory_no_messages() {
		let hb = Arc::new(mock(HB));
		let dest = Arc::new(mock(DEST_A));

		submit_for_dest(hb.clone(), dest.clone(), Vec::new(), vec![0xcc], true, 100, hb_id())
			.await
			.unwrap();

		let submissions = dest.submissions();
		assert_eq!(submissions.len(), 1);
		assert_eq!(submissions[0].len(), 1);
		assert!(matches!(submissions[0][0], Message::Consensus(_)));
	}

	#[tokio::test]
	async fn submits_full_batch_when_messages_present() {
		let hb = Arc::new(mock(HB));
		let dest = Arc::new(mock(DEST_A));

		let events = vec![
			Event::PostRequest(post_req(HB, DEST_A, 1)),
			Event::PostResponse(post_res(DEST_A, HB, 2)),
		];

		submit_for_dest(hb.clone(), dest.clone(), events, vec![0xcc], false, 100, hb_id())
			.await
			.unwrap();

		let submissions = dest.submissions();
		assert_eq!(submissions.len(), 1);
		assert_eq!(submissions[0].len(), 3, "consensus + request + response");
		assert!(matches!(submissions[0][0], Message::Consensus(_)));
		assert!(matches!(submissions[0][1], Message::Request(_)));
		assert!(matches!(submissions[0][2], Message::Response(_)));
	}

	#[tokio::test]
	async fn events_for_other_destinations_are_ignored() {
		let hb = Arc::new(mock(HB));
		let dest = Arc::new(mock(DEST_A));

		// Only DEST_B-targeted events; messaging-only proof for DEST_A.
		let events = vec![
			Event::PostRequest(post_req(HB, DEST_B, 1)),
			Event::PostResponse(post_res(DEST_B, HB, 2)),
		];

		submit_for_dest(hb.clone(), dest.clone(), events, vec![0xcc], false, 100, hb_id())
			.await
			.unwrap();

		assert!(
			dest.submissions().is_empty(),
			"DEST_A should see nothing when events target DEST_B"
		);
	}

	#[tokio::test]
	async fn skips_when_request_proof_fails_non_mandatory() {
		let hb = Arc::new(mock(HB).with_request_proof_fail());
		let dest = Arc::new(mock(DEST_A));

		// Requests only — if request proof fails and not mandatory, batch has only
		// consensus and should not be submitted.
		let events = vec![Event::PostRequest(post_req(HB, DEST_A, 1))];

		submit_for_dest(hb.clone(), dest.clone(), events, vec![0xcc], false, 100, hb_id())
			.await
			.unwrap();

		assert!(dest.submissions().is_empty());
	}
}
