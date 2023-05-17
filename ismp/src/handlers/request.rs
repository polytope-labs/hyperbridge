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
    host::ISMPHost,
    messaging::RequestMessage,
    router::RequestResponse,
};
use alloc::vec::Vec;

/// Validate the state machine, verify the request message and dispatch the message to the router
pub fn handle<H>(host: &H, msg: RequestMessage) -> Result<MessageResult, Error>
where
    H: ISMPHost,
{
    let consensus_client = validate_state_machine(host, msg.proof.height)?;
    // Verify membership proof
    let state = host.state_machine_commitment(msg.proof.height)?;

    consensus_client.verify_membership(
        host,
        RequestResponse::Request(msg.requests.clone()),
        state,
        &msg.proof,
    )?;

    let router = host.ismp_router();
    // If a receipt exists for any request then it's a duplicate and it is not dispatched
    let result = msg
        .requests
        .into_iter()
        .filter(|req| host.get_request_receipt(req).is_none())
        .map(|request| {
            let res = router.dispatch(request.clone());
            host.store_request_receipt(&request)?;
            Ok(res)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MessageResult::Request(result))
}
