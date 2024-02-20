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
    router::{GetResponse, Request, RequestResponse, Response},
    util::hash_request,
};
use alloc::{format, vec::Vec};

/// Validate the state machine, verify the response message and dispatch the message to the modules
pub fn handle<H>(host: &H, msg: ResponseMessage) -> Result<MessageResult, Error>
where
    H: IsmpHost,
{
    let proof = msg.proof();
    let state_machine = validate_state_machine(host, proof.height)?;
    let state = host.state_machine_commitment(proof.height)?;

    let state_machine_client = host
        .consensus_client_id(proof.height.id.consensus_state_id)
        .and_then(|id| host.consensus_client(id).ok())
        .and_then(|client| client.state_machine(proof.height.id.state_id).ok());
    let result = match &msg.datagram {
        RequestResponse::Response(responses) => {
            // For a response to be valid a request commitment must be present in storage
            // Also we must not have received a response for this request
            let responses = responses
                .iter()
                .filter(|response| {
                    let request = response.request();
                    let commitment = hash_request::<H>(&request);
                    host.request_commitment(commitment).is_ok() &&
                        host.response_receipt(&response).is_none() &&
                        !response.timed_out(host.timestamp()) &&
                        // either the proof metadata matches the source chain, or it's coming from a proxy
                        // in which case, we must NOT have a configured state machine for the source
                        (response.source_chain() == msg.proof.height.id.state_id ||
                            host.is_allowed_proxy(&msg.proof.height.id.state_id) &&
                                state_machine_client.is_none())
                })
                .cloned()
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
                    if res.is_ok() {
                        host.store_response_receipt(&response, &msg.signer)?;
                    }
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
        RequestResponse::Request(requests) => {
            let requests = requests
                .into_iter()
                .filter(|req| {
                    !req.timed_out(host.timestamp()) && req.dest_chain() == proof.height.id.state_id
                })
                .filter_map(|req| match req {
                    Request::Post(_) => None,
                    Request::Get(get) => {
                        let commitment = hash_request::<H>(&Request::Get(get.clone()));
                        if host.request_commitment(commitment).is_ok() &&
                            host.response_receipt(&Response::Get(GetResponse {
                                get: get.clone(),
                                values: Default::default(),
                            }))
                            .is_none()
                        {
                            Some(get)
                        } else {
                            None
                        }
                    },
                })
                .cloned()
                .collect::<Vec<_>>();
            // Ensure the proof height is greater than each retrieval height specified in the Get
            // requests
            sufficient_proof_height(&requests, &proof)?;
            // Since each get request can  contain multiple storage keys, we should handle them
            // individually
            requests
                .into_iter()
                .map(|request| {
                    let keys = request.keys.clone();
                    let values = state_machine.verify_state_proof(host, keys, state, &proof)?;

                    let router = host.ismp_router();
                    let cb = router.module_for_id(request.from.clone())?;
                    let res = cb
                        .on_response(Response::Get(GetResponse { get: request.clone(), values }))
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
                    let response = Response::Get(GetResponse {
                        get: request.clone(),
                        values: Default::default(),
                    });
                    if res.is_ok() {
                        host.store_response_receipt(&response, &msg.signer)?;
                    }
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
    };

    Ok(MessageResult::Response(result))
}
