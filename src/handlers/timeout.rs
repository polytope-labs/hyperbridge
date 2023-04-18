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
    handlers::{validate_state_machine, MessageResult, RequestResponseResult},
    host::ISMPHost,
    messaging::TimeoutMessage,
    router::RequestResponse,
    util::hash_request,
};

/// This function handles timeouts for Requests
pub fn handle<H>(host: &H, msg: TimeoutMessage) -> Result<MessageResult, Error>
where
    H: ISMPHost,
{
    let consensus_client = validate_state_machine(host, &msg.timeout_proof)?;
    let commitment = host.request_commitment(&msg.request)?;
    if commitment != hash_request::<H>(&msg.request) {
        return Err(Error::RequestCommitmentNotFound {
            nonce: msg.request.nonce(),
            source: msg.request.source_chain(),
            dest: msg.request.dest_chain(),
        })
    }

    let state = host.state_machine_commitment(msg.timeout_proof.height)?;
    if !msg.request.timed_out(state.timestamp()) {
        Err(Error::RequestTimeoutNotElapsed {
            nonce: msg.request.nonce(),
            source: msg.request.source_chain(),
            dest: msg.request.dest_chain(),
            timeout_timestamp: msg.request.timeout(),
            state_machine_time: state.timestamp(),
        })?
    }

    let key = consensus_client.state_trie_key(RequestResponse::Request(msg.request.clone()));

    let request = consensus_client.verify_state_proof(host, key, state, &msg.timeout_proof)?;

    if request.is_some() {
        Err(Error::ImplementationSpecific("Request not timed out".into()))?
    }

    let result = RequestResponseResult {
        dest_chain: msg.request.source_chain(),
        source_chain: msg.request.dest_chain(),
        nonce: msg.request.nonce(),
    };

    let router = host.ismp_router();
    router.dispatch_timeout(msg.request)?;

    Ok(MessageResult::Timeout(result))
}
