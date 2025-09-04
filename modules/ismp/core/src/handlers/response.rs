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

//! The ISMP response handler

use crate::{
	error::Error,
	events::{Event, RequestResponseHandled},
	handlers::{validate_state_machine, MessageResult},
	host::{IsmpHost, StateMachine},
	messaging::{hash_request, hash_response, ResponseMessage},
	router::{GetResponse, Request, RequestResponse, Response, StorageValue},
};
use alloc::{vec, vec::Vec};
use sp_weights::Weight;

/// Validate the state machine, verify the response message and dispatch the message to the modules
pub fn handle<H>(host: &H, msg: ResponseMessage) -> Result<MessageResult, anyhow::Error>
where
	H: IsmpHost,
{
	let proof = msg.proof();
	let state_machine = validate_state_machine(host, proof.height)?;
	let state = host.state_machine_commitment(proof.height)?;

	let consensus_clients = host.consensus_clients();
	let check_state_machine_client = |state_machine: StateMachine| {
		consensus_clients
			.iter()
			.find_map(|client| client.state_machine(state_machine).ok())
			.is_none()
	};

	let mut total_weights = Weight::zero();
	let result = match &msg.datagram {
		RequestResponse::Response(responses) => {
			for response in responses.iter() {
				let request = response.request();
				let commitment = hash_request::<H>(&request);

				if host.request_commitment(commitment).is_err() {
					Err(Error::UnsolicitedResponse { meta: response.into() })?
				}

				if host.response_receipt(&response).is_some() {
					Err(Error::DuplicateResponse { meta: response.into() })?
				}

				if response.timed_out(host.timestamp()) {
					Err(Error::ResponseTimeout { response: response.into() })?
				}

				let source_chain = response.source_chain();

				// in order to allow proxies, the host must configure the given state machine
				// as it's proxy and must not have a state machine client for the source chain
				let allow_proxy = host.is_allowed_proxy(&msg.proof.height.id.state_id) &&
					check_state_machine_client(source_chain);

				// check if the response is allowed to be proxied
				if response.source_chain() != msg.proof.height.id.state_id && !allow_proxy {
					Err(Error::ResponseProxyProhibited { meta: response.into() })?
				}
			}

			// Verify membership proof
			state_machine.verify_membership(
				host,
				RequestResponse::Response(responses.clone()),
				state,
				&proof,
			)?;

			let router = host.ismp_router();
			responses
				.clone()
				.into_iter()
				.map(|response| {
					let cb = router.module_for_id(response.destination_module())?;
					// Store response receipt to prevent reentrancy attack
					let signer = host.store_response_receipt(&response, &msg.signer)?;
					let res = cb.on_response(response.clone()).map(|weight| {
						total_weights.saturating_accrue(weight);
						let commitment = hash_response::<H>(&response);
						Event::PostResponseHandled(RequestResponseHandled {
							commitment,
							relayer: signer,
						})
					});
					// Delete receipt if module callback failed so it can be timed out
					if res.is_err() {
						host.delete_response_receipt(&response)?;
					}
					Ok::<_, anyhow::Error>(res)
				})
				.collect::<Result<Vec<_>, _>>()?
		},
		RequestResponse::Request(requests) => {
			let mut get_requests = vec![];
			for req in requests.iter() {
				let Request::Get(get) = req else {
					Err(Error::InvalidResponseType { meta: req.into() })?
				};

				if req.timed_out(host.timestamp()) {
					Err(Error::RequestTimeout { meta: req.into() })?
				}

				if req.dest_chain() != proof.height.id.state_id {
					Err(Error::RequestProofMetadataNotValid { meta: req.into() })?
				}

				let commitment = hash_request::<H>(&Request::Get(get.clone()));
				if host.request_commitment(commitment).is_err() {
					Err(Error::UnknownRequest { meta: req.into() })?
				}

				let res =
					Response::Get(GetResponse { get: get.clone(), values: Default::default() });

				if host.response_receipt(&res).is_some() {
					Err(Error::DuplicateResponse { meta: res.into() })?
				}

				get_requests.push(get.clone());
			}

			// Ensure the proof height is equal to each retrieval height specified in the Get
			// requests
			if !get_requests.iter().all(|get| get.height == proof.height.height) {
				Err(Error::InsufficientProofHeight)?
			}

			// Since each get request can contain multiple storage keys
			// we should handle them individually
			get_requests
				.into_iter()
				.map(|request| {
					let wrapped_req = Request::Get(request.clone());
					let keys = request.keys.clone();
					let values = state_machine
						.verify_state_proof(host, keys, state, &proof)?
						.into_iter()
						.map(|(key, value)| StorageValue { key, value })
						.collect();

					let router = host.ismp_router();
					let cb = router.module_for_id(request.from.clone())?;
					let response = Response::Get(GetResponse {
						get: request.clone(),
						values: Default::default(),
					});
					let signer = host.store_response_receipt(&response, &msg.signer)?;
					let res = cb
						.on_response(Response::Get(GetResponse { get: request.clone(), values }))
						.map(|weight| {
							total_weights.saturating_accrue(weight);
							let commitment = hash_request::<H>(&wrapped_req);
							Event::GetRequestHandled(RequestResponseHandled {
								commitment,
								relayer: signer,
							})
						});
					// Delete receipt if module callback failed so it can be timed out
					if res.is_err() {
						host.delete_response_receipt(&response)?;
					}
					Ok::<_, anyhow::Error>(res)
				})
				.collect::<Result<Vec<_>, _>>()?
		},
	};

	Ok(MessageResult::Response { events: result, weight: total_weights })
}
