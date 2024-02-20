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
    handlers::{validate_state_machine, MessageResult},
    host::{IsmpHost, StateMachine},
    messaging::RequestMessage,
    module::{DispatchError, DispatchSuccess},
    router::{Request, RequestResponse},
};
use alloc::{format, vec::Vec};

/// Validate the state machine, verify the request message and dispatch the message to the modules
pub fn handle<H>(host: &H, msg: RequestMessage) -> Result<MessageResult, Error>
where
    H: IsmpHost,
{
    let state_machine = validate_state_machine(host, msg.proof.height)?;

    // Verify membership proof
    let state = host.state_machine_commitment(msg.proof.height)?;
    state_machine.verify_membership(
        host,
        RequestResponse::Request(msg.requests.clone().into_iter().map(Request::Post).collect()),
        state,
        &msg.proof,
    )?;

    let consensus_clients = host.consensus_clients();
    let check_for_consensus_client = |state_machine: StateMachine| {
        consensus_clients
            .iter()
            .find_map(|client| client.state_machine(state_machine).ok())
            .is_none()
    };

    let router = host.ismp_router();
    let result = msg
        .requests
        .into_iter()
        .filter(|req| {
            let req = Request::Post(req.clone());
            // If a receipt exists for any request then it's a duplicate and it is not dispatched
            host.request_receipt(&req).is_none() &&
                // can't dispatch timed out requests
                !req.timed_out(host.timestamp()) &&
                // either the host is a router and can accept requests on behalf of any chain
                // or the request must be intended for this chain
                (req.dest_chain() == host.host_state_machine() ||
                host.is_router()) &&
                // either the proof metadata matches the source chain, or it's coming from a proxy
                // in which case, we must NOT have a configured state machine for the source
                (req.source_chain() == msg.proof.height.id.state_id ||
                host.is_allowed_proxy(&msg.proof.height.id.state_id) &&
                    check_for_consensus_client(req.source_chain()))
        })
        .map(|request| {
            let cb = router.module_for_id(request.to.clone())?;
            let res = cb
                .on_accept(request.clone())
                .map(|_| DispatchSuccess {
                    dest_chain: request.dest,
                    source_chain: request.source,
                    nonce: request.nonce,
                })
                .map_err(|e| DispatchError {
                    msg: format!("{e:?}"),
                    nonce: request.nonce,
                    source_chain: request.source,
                    dest_chain: request.dest,
                });
            if res.is_ok() {
                host.store_request_receipt(&Request::Post(request), &msg.signer)?;
            }
            Ok(res)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MessageResult::Request(result))
}
