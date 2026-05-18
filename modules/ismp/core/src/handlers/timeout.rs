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
	messaging::{dedup_requests, hash_request, TimeoutMessage},
	router::{GetResponse, Request},
};
use alloc::vec::Vec;
use sp_weights::Weight;

/// This function handles timeouts
pub fn handle<H>(host: &H, msg: TimeoutMessage) -> Result<MessageResult, anyhow::Error>
where
	H: IsmpHost,
{
	if msg.requests().is_empty() {
		Err(Error::EmptyBatch)?
	}

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

			let wrapped: Vec<Request> = requests.iter().cloned().map(Request::Post).collect();
			dedup_requests::<H>(&wrapped)?;

			for post in &requests {
				let dest_chain = post.dest;

				// in order to allow proxies, the host must configure the given state machine
				// as it's proxy and must not have a state machine client for the destination chain
				let allow_proxy = host.is_allowed_proxy(&timeout_proof.height.id.state_id) &&
					check_state_machine_client(dest_chain);

				// check if the timeout is allowed to be proxied
				if dest_chain != timeout_proof.height.id.state_id && !allow_proxy {
					Err(Error::RequestProxyProhibited { meta: post.into() })?
				}

				// Ensure a commitment exists for all requests in the batch
				let commitment = hash_request::<H>(&Request::Post(post.clone()));
				if host.request_commitment(commitment).is_err() {
					Err(Error::UnknownRequest { meta: post.into() })?
				}

				if !post.timed_out(state.timestamp()) {
					Err(Error::RequestTimeoutNotElapsed {
						meta: post.into(),
						timeout_timestamp: post.timeout(),
						state_machine_time: state.timestamp(),
					})?
				}
			}

			let commitments = requests
				.iter()
				.map(|post| hash_request::<H>(&Request::Post(post.clone())))
				.collect();
			state_machine.verify_non_membership(host, commitments, state, &timeout_proof)?;

			let router = host.ismp_router();
			requests
				.into_iter()
				.map(|post| {
					let cb = router.module_for_id(post.from.clone())?;
					let request = Request::Post(post.clone());
					// Delete commitment to prevent rentrancy attack
					let meta = host.delete_request_commitment(&request)?;
					let mut signer = None;
					// If it was a routed request delete the receipt
					if host.host_state_machine() != post.source {
						signer = host.delete_request_receipt(&request).ok();
					}
					let res = cb.on_timeout(request.clone()).map(|weight| {
						total_module_weight.saturating_accrue(weight);
						let commitment = hash_request::<H>(&request);
						Event::PostRequestTimeoutHandled(TimeoutHandled {
							commitment,
							source: post.source,
							dest: post.dest,
						})
					});
					if res.is_ok() {
						host.on_request_timeout(&request, meta)?;
					} else {
						// Module callback failed; restore commitment so the request
						// can be retried.
						host.store_request_commitment(&request, meta)?;
						if host.host_state_machine() != post.source && signer.is_some() {
							host.store_request_receipt(&request, &signer.expect("Infaliible"))?;
						}
					}
					Ok::<_, anyhow::Error>(res)
				})
				.collect::<Result<Vec<_>, _>>()?
		},
		TimeoutMessage::Get { requests } => {
			let wrapped: Vec<Request> = requests.iter().cloned().map(Request::Get).collect();
			dedup_requests::<H>(&wrapped)?;

			for get in &requests {
				let commitment = hash_request::<H>(&Request::Get(get.clone()));
				// if we have a commitment, it came from us
				if host.request_commitment(commitment).is_err() {
					Err(Error::UnknownRequest { meta: get.into() })?
				}

				// Reject the timeout if a response has already been received for this request
				let response = GetResponse { get: get.clone(), values: Default::default() };
				if host.response_receipt(&response).is_some() {
					Err(Error::GetResponseAlreadyReceived { meta: get.into() })?
				}

				// Ensure the get timeout has elapsed on the host
				if !get.timed_out(host.timestamp()) {
					Err(Error::RequestTimeoutNotElapsed {
						meta: get.into(),
						timeout_timestamp: get.timeout(),
						state_machine_time: host.timestamp(),
					})?
				}
			}

			let router = host.ismp_router();
			requests
				.into_iter()
				.map(|get| {
					let cb = router.module_for_id(get.from.clone())?;
					let request = Request::Get(get.clone());
					// Delete commitment to prevent reentrancy
					let meta = host.delete_request_commitment(&request)?;
					let res = cb.on_timeout(request.clone()).map(|weight| {
						total_module_weight.saturating_accrue(weight);
						let commitment = hash_request::<H>(&request);
						Event::GetRequestTimeoutHandled(TimeoutHandled {
							commitment,
							source: get.source,
							dest: get.dest,
						})
					});
					if res.is_ok() {
						host.on_request_timeout(&request, meta)?;
					} else {
						// Module callback failed; restore commitment so the request
						// can be retried.
						host.store_request_commitment(&request, meta)?;
					}
					Ok::<_, anyhow::Error>(res)
				})
				.collect::<Result<Vec<_>, _>>()?
		},
	};

	Ok(MessageResult::Timeout { events: results, weight: total_module_weight })
}
