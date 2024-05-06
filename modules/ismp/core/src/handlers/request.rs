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

//! The ISMP request handler

use crate::{
	error::Error,
	events::{Event, RequestResponseHandled},
	handlers::{validate_state_machine, MessageResult},
	host::{IsmpHost, StateMachine},
	messaging::{hash_request, RequestMessage},
	router::{Request, RequestResponse},
};
use alloc::vec::Vec;

/// Validate the state machine, verify the request message and dispatch the message to the modules
pub fn handle<H>(host: &H, msg: RequestMessage) -> Result<MessageResult, Error>
where
	H: IsmpHost,
{
	let signer = msg.signer.clone();
	let state_machine = validate_state_machine(host, msg.proof.height)?;
	let consensus_clients = host.consensus_clients();
	let check_state_machine_client = |state_machine: StateMachine| {
		consensus_clients
			.iter()
			.find_map(|client| client.state_machine(state_machine).ok())
			.is_none()
	};

	let router = host.ismp_router();
	for req in msg.requests.iter() {
		let req = Request::Post(req.clone());
		// If a receipt exists for any request then it's a duplicate and it is not dispatched
		if host.request_receipt(&req).is_some() {
			Err(Error::DuplicateRequest { meta: req.clone().into() })?
		}

		// can't dispatch timed out requests
		if req.timed_out(host.timestamp()) {
			Err(Error::RequestTimeout { meta: req.clone().into() })?
		}

		// either the host is a router and can accept requests on behalf of any chain
		// or the request must be intended for this chain
		if req.dest_chain() != host.host_state_machine() && !host.is_router() {
			Err(Error::InvalidRequestDestination { meta: req.clone().into() })?
		}

		// check if the source chain does not match the proof metadata in which case
		// the proof metadata must be the configured proxy
		// and we must not have a configured state machine client for the destination
		if req.source_chain() != msg.proof.height.id.state_id &&
			!(host.is_allowed_proxy(&msg.proof.height.id.state_id) &&
				check_state_machine_client(req.source_chain()))
		{
			Err(Error::RequestProxyProhibited { meta: req.clone().into() })?
		}
	}

	// Verify membership proof

	let state = host.state_machine_commitment(msg.proof.height)?;
	state_machine.verify_membership(
		host,
		RequestResponse::Request(msg.requests.clone().into_iter().map(Request::Post).collect()),
		state,
		&msg.proof,
	)?;

	let result = msg
		.requests
		.into_iter()
		.map(|request| {
			let wrapped_req = Request::Post(request.clone());
			let lambda = || {
				let cb = router.module_for_id(request.to.clone())?;
				// Store request receipt to prevent reentrancy attack
				host.store_request_receipt(&wrapped_req, &msg.signer)?;
				let res = cb.on_accept(request.clone()).map(|_| {
					let commitment = hash_request::<H>(&wrapped_req);
					Event::PostRequestHandled(RequestResponseHandled {
						commitment,
						relayer: signer.clone(),
					})
				});
				// Delete receipt if module callback failed so it can be timed out
				if res.is_err() {
					host.delete_request_receipt(&wrapped_req)?;
				}
				Ok(res)
			};

			let res = lambda().and_then(|res| res);
			res
		})
		.collect::<Vec<_>>();

	Ok(MessageResult::Request(result))
}
