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

use std::sync::Arc;

use futures::StreamExt;
use ismp::{
	consensus::StateMachineHeight,
	events::Event,
	host::StateMachine,
	messaging::{hash_request, hash_response, Message, Proof, RequestMessage, ResponseMessage},
	router::{Request, RequestResponse, Response},
};
use tesseract_primitives::{Hasher, IsmpProvider, Query};

use crate::provider::ConsensusProofProvider;

pub async fn run(
	hyperbridge: Arc<dyn IsmpProvider>,
	evm_chain: Arc<dyn IsmpProvider>,
	proof_provider: Arc<dyn ConsensusProofProvider>,
	coprocessor: StateMachine,
) -> Result<(), anyhow::Error> {
	let hb_state_machine_id = hyperbridge.state_machine_id();
	let evm_state_machine = evm_chain.state_machine_id().state_id;

	let mut update_stream =
		evm_chain.state_machine_update_notification(hb_state_machine_id).await?;

	let mut previous_height = evm_chain.initial_height();
	let hb_name = hyperbridge.name();
	let evm_name = evm_chain.name();

	while let Some(item) = update_stream.next().await {
		let update = match item {
			Ok(update) => update,
			Err(err) => {
				tracing::error!("Outbound {hb_name}->{evm_name}: update stream error: {err:?}");
				continue;
			},
		};

		let events = match hyperbridge.query_ismp_events(previous_height, update.clone()).await {
			Ok(events) => events,
			Err(err) => {
				tracing::error!("Outbound {hb_name}->{evm_name}: failed to query events: {err:?}");
				continue;
			},
		};

		previous_height = update.latest_height;

		let mut requests = Vec::new();
		let mut responses = Vec::new();

		for event in events {
			match event {
				Event::PostRequest(req) if req.dest == evm_state_machine => {
					requests.push(req);
				},
				Event::PostResponse(res) if res.dest_chain() == evm_state_machine => {
					responses.push(res);
				},
				_ => {},
			}
		}

		if requests.is_empty() && responses.is_empty() {
			continue;
		}

		tracing::info!(
			"Outbound {hb_name}->{evm_name}: {} requests, {} responses at height {}",
			requests.len(),
			responses.len(),
			update.latest_height,
		);

		let consensus_msg = match proof_provider.get_proof(update.latest_height).await {
			Ok(Some(msg)) => msg,
			Ok(None) => {
				tracing::info!(
					"Outbound {hb_name}->{evm_name}: no consensus proof for height {}, skipping",
					update.latest_height,
				);
				continue;
			},
			Err(err) => {
				tracing::error!(
					"Outbound {hb_name}->{evm_name}: consensus proof query failed: {err:?}",
				);
				continue;
			},
		};

		let height = StateMachineHeight { id: hb_state_machine_id, height: update.latest_height };
		let signer = hyperbridge.address();

		let mut batch = vec![Message::Consensus(consensus_msg)];

		if !requests.is_empty() {
			let keys: Vec<_> = requests
				.iter()
				.map(|req| {
					let commitment = hash_request::<Hasher>(&Request::Post(req.clone()));
					Query {
						source_chain: req.source,
						dest_chain: req.dest,
						nonce: req.nonce,
						commitment,
					}
				})
				.collect();

			match hyperbridge
				.query_requests_proof(update.latest_height, keys, evm_state_machine)
				.await
			{
				Ok(proof) => {
					batch.push(Message::Request(RequestMessage {
						requests,
						proof: Proof { height, proof },
						signer: signer.clone(),
					}));
				},
				Err(err) => {
					tracing::error!(
						"Outbound {hb_name}->{evm_name}: request proof failed: {err:?}",
					);
				},
			}
		}

		if !responses.is_empty() {
			let keys: Vec<_> = responses
				.iter()
				.map(|res| {
					let commitment = hash_response::<Hasher>(&Response::Post(res.clone()));
					Query {
						source_chain: res.post.source,
						dest_chain: res.post.dest,
						nonce: res.post.nonce,
						commitment,
					}
				})
				.collect();

			match hyperbridge
				.query_responses_proof(update.latest_height, keys, evm_state_machine)
				.await
			{
				Ok(proof) => {
					batch.push(Message::Response(ResponseMessage {
						datagram: RequestResponse::Response(
							responses.into_iter().map(Response::Post).collect(),
						),
						proof: Proof { height, proof },
						signer: signer.clone(),
					}));
				},
				Err(err) => {
					tracing::error!(
						"Outbound {hb_name}->{evm_name}: response proof failed: {err:?}",
					);
				},
			}
		}

		// Only consensus message in the batch means all proof queries failed
		if batch.len() <= 1 {
			continue;
		}

		match evm_chain.submit(batch, coprocessor).await {
			Ok(_) => {
				tracing::info!(
					"Outbound {hb_name}->{evm_name}: batch submitted at height {}",
					update.latest_height,
				);
			},
			Err(err) => {
				tracing::error!("Outbound {hb_name}->{evm_name}: batch submission failed: {err:?}",);
			},
		}
	}

	Err(anyhow::anyhow!("Outbound {hb_name}->{evm_name} update stream ended"))
}
