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

//! The ISMP request timeout handler

use crate::{
	error::Error,
	events::{Event, TimeoutHandled},
	handlers::{validate_state_machine, MessageResult},
	host::{IsmpHost, StateMachine},
	messaging::{hash_post_response, hash_request, TimeoutMessage},
	router::Response,
};
use alloc::vec::Vec;
use sp_weights::Weight;

/// This function handles timeouts
pub fn handle<H>(host: &H, msg: TimeoutMessage) -> Result<MessageResult, anyhow::Error>
where
	H: IsmpHost,
{
	let consensus_clients = host.consensus_clients();

	let check_state_machine_client = |state_machine: StateMachine| {
		consensus_clients
			.iter()
			.find_map(|client| client.state_machine(state_machine).ok())
			.is_none()
	};
	let mut total_module_weight = Weight::zero();

	let results = match msg {
		TimeoutMessage::Post { requests, timeout_proof } => {
			let state_machine = validate_state_machine(host, timeout_proof.height)?;
			let state = host.state_machine_commitment(timeout_proof.height)?;

			for request in &requests {
				let dest_chain = request.dest_chain();

				// in order to allow proxies, the host must configure the given state machine
				// as it's proxy and must not have a state machine client for the destination chain
				let allow_proxy = host.is_allowed_proxy(&timeout_proof.height.id.state_id) &&
					check_state_machine_client(dest_chain);

				// check if the timeout is allowed to be proxied
				if dest_chain != timeout_proof.height.id.state_id && !allow_proxy {
					Err(Error::RequestProxyProhibited { meta: request.into() })?
				}

				// Ensure a commitment exists for all requests in the batch
				let commitment = hash_request::<H>(request);
				if host.request_commitment(commitment).is_err() {
					Err(Error::UnknownRequest { meta: request.into() })?
				}

				if !request.timed_out(state.timestamp()) {
					Err(Error::RequestTimeoutNotElapsed {
						meta: request.into(),
						timeout_timestamp: request.timeout(),
						state_machine_time: state.timestamp(),
					})?
				}
			}

			let keys = state_machine.receipts_state_trie_key(requests.clone().into());
			let values = state_machine.verify_state_proof(host, keys, state, &timeout_proof)?;
			if values.into_iter().any(|(_key, val)| val.is_some()) {
				Err(Error::Custom("Some Requests in the batch have been delivered".into()))?
			}

			let router = host.ismp_router();
			requests
				.into_iter()
				.map(|request| {
					let cb = router.module_for_id(request.source_module())?;
					// Delete commitment to prevent rentrancy attack
					let meta = host.delete_request_commitment(&request)?;
					let mut signer = None;
					// If it was a routed request delete the receipt
					if host.host_state_machine() != request.source_chain() {
						signer = host.delete_request_receipt(&request).ok();
					}
					let res = cb.on_timeout(request.clone().into()).map(|weight| {
						total_module_weight.saturating_accrue(weight);
						let commitment = hash_request::<H>(&request);
						Event::PostRequestTimeoutHandled(TimeoutHandled {
							commitment,
							source: request.source_chain(),
							dest: request.dest_chain(),
						})
					});
					// If module callback failed restore commitment so it can be retried
					if res.is_err() {
						host.store_request_commitment(&request, meta)?;
						// If the request was routed we store it's receipt
						if host.host_state_machine() != request.source_chain() && signer.is_some() {
							host.store_request_receipt(&request, &signer.expect("Infaliible"))?;
						}
					}
					Ok::<_, anyhow::Error>(res)
				})
				.collect::<Result<Vec<_>, _>>()?
		},
		TimeoutMessage::PostResponse { responses, timeout_proof } => {
			let state_machine = validate_state_machine(host, timeout_proof.height)?;
			let state = host.state_machine_commitment(timeout_proof.height)?;
			for response in &responses {
				let dest_chain = response.dest_chain();

				// in order to allow proxies, the host must configure the given state machine
				// as it's proxy and must not have a state machine client for the destination chain
				let allow_proxy = host.is_allowed_proxy(&timeout_proof.height.id.state_id) &&
					check_state_machine_client(dest_chain);

				// check if the response is allowed to be proxied
				if dest_chain != timeout_proof.height.id.state_id && !allow_proxy {
					Err(Error::ResponseProxyProhibited {
						meta: Response::Post(response.clone()).into(),
					})?
				}

				// Ensure a commitment exists for all responses in the batch
				let commitment = hash_post_response::<H>(response);
				if host.response_commitment(commitment).is_err() {
					Err(Error::UnknownResponse { meta: Response::Post(response.clone()).into() })?
				}

				if response.timeout() > state.timestamp() {
					Err(Error::RequestTimeoutNotElapsed {
						meta: response.into(),
						timeout_timestamp: response.timeout(),
						state_machine_time: state.timestamp(),
					})?
				}
			}

			let items = responses.iter().map(|r| Into::into(r.clone())).collect::<Vec<Response>>();
			let keys = state_machine.receipts_state_trie_key(items.into());
			let values = state_machine.verify_state_proof(host, keys, state, &timeout_proof)?;
			if values.into_iter().any(|(_key, val)| val.is_some()) {
				Err(Error::Custom("Some responses in the batch have been delivered".into()))?
			}

			let router = host.ismp_router();
			responses
				.into_iter()
				.map(|response| {
					let cb = router.module_for_id(response.source_module())?;
					// Delete commitment to prevent rentrancy
					let meta = host.delete_response_commitment(&response)?;
					// If the response was routed we delete it's receipt
					let mut signer = None;
					if host.host_state_machine() != response.source_chain() {
						signer =
							host.delete_response_receipt(&Response::Post(response.clone())).ok();
					}
					let res = cb.on_timeout(response.clone().into()).map(|weight| {
						total_module_weight.saturating_accrue(weight);
						let commitment = hash_post_response::<H>(&response);
						Event::PostResponseTimeoutHandled(TimeoutHandled {
							commitment,
							source: response.source_chain(),
							dest: response.dest_chain(),
						})
					});
					// If module callback failed restore commitment so it can be retried
					if res.is_err() {
						host.store_response_commitment(&response, meta)?;
						if host.host_state_machine() != response.source_chain() && signer.is_some()
						{
							host.store_response_receipt(
								&Response::Post(response),
								&signer.expect("Infallible"),
							)?;
						}
					}
					Ok::<_, anyhow::Error>(res)
				})
				.collect::<Result<Vec<_>, _>>()?
		},
		TimeoutMessage::Get { requests } => {
			for request in &requests {
				let commitment = hash_request::<H>(request);
				// if we have a commitment, it came from us
				if host.request_commitment(commitment).is_err() {
					Err(Error::UnknownRequest { meta: request.into() })?
				}

				// Ensure the get timeout has elapsed on the host
				if !request.timed_out(host.timestamp()) {
					Err(Error::RequestTimeoutNotElapsed {
						meta: request.into(),
						timeout_timestamp: request.timeout(),
						state_machine_time: host.timestamp(),
					})?
				}
			}

			let router = host.ismp_router();
			requests
				.into_iter()
				.map(|request| {
					let cb = router.module_for_id(request.source_module())?;
					// Delete commitment to prevent reentrancy
					let meta = host.delete_request_commitment(&request)?;
					let res = cb.on_timeout(request.clone().into()).map(|weight| {
						total_module_weight.saturating_accrue(weight);
						let commitment = hash_request::<H>(&request);
						Event::GetRequestTimeoutHandled(TimeoutHandled {
							commitment,
							source: request.source_chain(),
							dest: request.dest_chain(),
						})
					});
					// If module callback failed, restore commitment so it can be retried
					if res.is_err() {
						host.store_request_commitment(&request, meta)?;
					}
					Ok::<_, anyhow::Error>(res)
				})
				.collect::<Result<Vec<_>, _>>()?
		},
	};

	Ok(MessageResult::Timeout { events: results, weight: total_module_weight })
}
