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
    handlers::{validate_state_machine, MessageResult, RequestResponseResult},
    host::ISMPHost,
    messaging::ResponseMessage,
    router::RequestResponse,
    util::hash_request,
};

/// Validate the state machine, verify the response message and dispatch the message to the router
pub fn handle<H>(host: &H, msg: ResponseMessage) -> Result<MessageResult, Error>
where
    H: ISMPHost,
{
    let consensus_client = validate_state_machine(host, &msg.proof)?;
    // For a response to be valid a request commitment must be present in storage
    let commitment = host.request_commitment(&msg.response.request)?;

    if commitment != hash_request::<H>(&msg.response.request) {
        return Err(Error::RequestCommitmentNotFound {
            nonce: msg.response.request.nonce(),
            source: msg.response.request.source_chain(),
            dest: msg.response.request.dest_chain(),
        })
    }

    let state = host.state_machine_commitment(msg.proof.height)?;
    // Verify membership proof
    consensus_client.verify_membership(
        host,
        RequestResponse::Response(msg.response.clone()),
        state,
        &msg.proof,
    )?;

    let router = host.ismp_router();

    let result = RequestResponseResult {
        dest_chain: msg.response.request.source_chain(),
        source_chain: msg.response.request.dest_chain(),
        nonce: msg.response.request.nonce(),
    };

    router.write_response(msg.response)?;

    Ok(MessageResult::Response(result))
}
