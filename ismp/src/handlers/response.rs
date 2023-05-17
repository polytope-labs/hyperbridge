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
    handlers::{validate_state_machine, MessageResult},
    host::ISMPHost,
    messaging::{sufficient_proof_height, ResponseMessage},
    router::{RequestResponse, Response},
    util::hash_request,
};
use alloc::{string::ToString, vec::Vec};

/// Validate the state machine, verify the response message and dispatch the message to the router
pub fn handle<H>(host: &H, msg: ResponseMessage) -> Result<MessageResult, Error>
where
    H: ISMPHost,
{
    let consensus_client = validate_state_machine(host, msg.proof().height)?;
    for request in &msg.requests() {
        // For a response to be valid a request commitment must be present in storage
        let commitment = host.request_commitment(request)?;

        if commitment != hash_request::<H>(request) {
            return Err(Error::RequestCommitmentNotFound {
                nonce: request.nonce(),
                source: request.source_chain(),
                dest: request.dest_chain(),
            })
        }
    }

    let state = host.state_machine_commitment(msg.proof().height)?;

    let result = match msg {
        ResponseMessage::Post { responses, proof } => {
            // Verify membership proof
            consensus_client.verify_membership(
                host,
                RequestResponse::Response(responses.clone()),
                state,
                &proof,
            )?;

            let router = host.ismp_router();

            responses
                .into_iter()
                .map(|response| {
                    let request = response.request();
                    let res = router.write_response(response);
                    host.delete_request_commitment(&request)?;
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        ResponseMessage::Get { requests, proof } => {
            // Ensure the proof height is greater than each retrieval height specified in the Get
            // requests
            sufficient_proof_height(&requests, &proof)?;
            // Since each get request can  contain multiple storage keys, we should handle them
            // individually
            requests
                .into_iter()
                .map(|request| {
                    let keys = request.keys().ok_or_else(|| {
                        Error::ImplementationSpecific("Missing keys for get request".to_string())
                    })?;
                    let values =
                        consensus_client.verify_state_proof(host, keys.clone(), state, &proof)?;

                    let router = host.ismp_router();
                    let res = router.write_response(Response::Get {
                        get: request.get_request()?,
                        values: keys.into_iter().zip(values.into_iter()).collect(),
                    });
                    host.delete_request_commitment(&request)?;
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };

    Ok(MessageResult::Response(result))
}
