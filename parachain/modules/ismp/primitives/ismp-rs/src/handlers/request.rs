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

/// Validate the state machine, verify the request message and dispatch the message to the router
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

    let check_source = |source: StateMachine| -> bool {
        msg.proof.height.id.state_id == source || host.is_allowed_proxy(&source)
    };

    let router = host.ismp_router();
    // If a receipt exists for any request then it's a duplicate and it is not dispatched
    let result = msg
        .requests
        .into_iter()
        .filter(|req| {
            let req = Request::Post(req.clone());
            host.request_receipt(&req).is_none() &&
                !req.timed_out(state.timestamp()) &&
                check_source(req.source_chain())
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
                host.store_request_receipt(&Request::Post(request))?;
            }
            Ok(res)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MessageResult::Request(result))
}
