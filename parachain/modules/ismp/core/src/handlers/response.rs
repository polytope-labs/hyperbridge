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
    host::IsmpHost,
    messaging::{sufficient_proof_height, ResponseMessage},
    module::{DispatchError, DispatchSuccess},
    router::{GetResponse, RequestResponse, Response},
    util::hash_request,
};
use alloc::{format, string::ToString, vec::Vec};

/// Validate the state machine, verify the response message and dispatch the message to the router
pub fn handle<H>(host: &H, msg: ResponseMessage) -> Result<MessageResult, Error>
where
    H: IsmpHost,
{
    let state_machine = validate_state_machine(host, msg.proof().height)?;

    let state = host.state_machine_commitment(msg.proof().height)?;

    let result = match msg {
        ResponseMessage::Post { responses, proof } => {
            // For a response to be valid a request commitment must be present in storage
            // Also we must not have received a response for this request
            let responses = responses
                .into_iter()
                .filter(|response| {
                    let request = response.request();
                    let commitment = hash_request::<H>(&request);
                    host.request_commitment(commitment).is_ok() &&
                        host.response_receipt(&request).is_none()
                })
                .collect::<Vec<_>>();
            // Verify membership proof
            state_machine.verify_membership(
                host,
                RequestResponse::Response(responses.clone()),
                state,
                &proof,
            )?;

            let router = host.ismp_router();

            responses
                .into_iter()
                .map(|response| {
                    let cb = router.module_for_id(response.destination_module())?;
                    let res = cb
                        .on_response(response.clone())
                        .map(|_| DispatchSuccess {
                            dest_chain: response.dest_chain(),
                            source_chain: response.source_chain(),
                            nonce: response.nonce(),
                        })
                        .map_err(|e| DispatchError {
                            msg: format!("{e:?}"),
                            nonce: response.nonce(),
                            source_chain: response.source_chain(),
                            dest_chain: response.dest_chain(),
                        });
                    host.store_response_receipt(&response.request())?;
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
        ResponseMessage::Get { requests, proof } => {
            let requests = requests
                .into_iter()
                .filter(|request| {
                    let commitment = hash_request::<H>(request);
                    host.request_commitment(commitment).is_ok() &&
                        host.response_receipt(request).is_none()
                })
                .collect::<Vec<_>>();
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
                    let values = state_machine.verify_state_proof(host, keys, state, &proof)?;

                    let router = host.ismp_router();
                    let cb = router.module_for_id(request.source_module())?;
                    let res = cb
                        .on_response(Response::Get(GetResponse {
                            get: request.get_request()?,
                            values,
                        }))
                        .map(|_| DispatchSuccess {
                            dest_chain: request.dest_chain(),
                            source_chain: request.source_chain(),
                            nonce: request.nonce(),
                        })
                        .map_err(|e| DispatchError {
                            msg: format!("{e:?}"),
                            nonce: request.nonce(),
                            source_chain: request.source_chain(),
                            dest_chain: request.dest_chain(),
                        });
                    host.store_response_receipt(&request)?;
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
    };

    Ok(MessageResult::Response(result))
}
