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
    handlers::{validate_state_machine, MessageResult},
    host::{IsmpHost, StateMachine},
    messaging::TimeoutMessage,
    module::{DispatchError, DispatchSuccess},
    router::Response,
    util::{hash_post_response, hash_request},
};
use alloc::{format, vec::Vec};

/// This function handles timeouts
pub fn handle<H>(host: &H, msg: TimeoutMessage) -> Result<MessageResult, Error>
where
    H: IsmpHost,
{
    let consensus_clients = host.consensus_clients();

    let check_for_consensus_client = |state_machine: StateMachine| {
        consensus_clients
            .iter()
            .find_map(|client| client.state_machine(state_machine).ok())
            .is_none()
    };

    let results = match msg {
        TimeoutMessage::Post { requests, timeout_proof } => {
            let state_machine = validate_state_machine(host, timeout_proof.height)?;
            let state = host.state_machine_commitment(timeout_proof.height)?;

            let requests = requests
                .into_iter()
                .map(|req| {
                    // check if the request destination chain matches the proof metadata 
                    // or if the proof metadata refers to the configured proxy 
                    // and we don't have a configured state machine client for the destination 
                    if req.dest_chain() == timeout_proof.height.id.state_id ||
                        (host.is_allowed_proxy(&timeout_proof.height.id.state_id) &&
                        check_for_consensus_client(req.dest_chain()))
                    {
                        Ok(req)
                    } else {
                        Err(Error::ImplementationSpecific(String::from("Timeout: Request does not meet the required criteria")))
                    }
                }).collect::<Result<Vec<_>, Error>>()?;

            

            for request in &requests {
                // Ensure a commitment exists for all requests in the batch
                let commitment = hash_request::<H>(request);
                host.request_commitment(commitment)?;

                if !request.timed_out(state.timestamp()) {
                    Err(Error::RequestTimeoutNotElapsed {
                        nonce: request.nonce(),
                        source: request.source_chain(),
                        dest: request.dest_chain(),
                        timeout_timestamp: request.timeout(),
                        state_machine_time: state.timestamp(),
                    })?
                }
            }

            let keys = state_machine.state_trie_key(requests.clone().into());
            let values = state_machine.verify_state_proof(host, keys, state, &timeout_proof)?;
            if values.into_iter().any(|(_key, val)| val.is_some()) {
                Err(Error::ImplementationSpecific("Some Requests not timed out".into()))?
            }

            let router = host.ismp_router();
            requests
                .into_iter()
                .map(|request| {
                    let cb = router.module_for_id(request.source_module())?;
                    let res = cb
                        .on_timeout(request.clone().into())
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
                    if res.is_ok() {
                        host.delete_request_commitment(&request)?;
                        // If the request was routed we delete it's receipt
                        if host.host_state_machine() != request.source_chain() {
                            host.delete_request_receipt(&request)?;
                        }
                    }
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
        TimeoutMessage::PostResponse { responses, timeout_proof } => {
            let state_machine = validate_state_machine(host, timeout_proof.height)?;
            let state = host.state_machine_commitment(timeout_proof.height)?;

            let responses = responses
                .into_iter()
                .filter(|res| {
                    // check if the destination chain matches the proof metadata
                    // or if the proof metadata refers to the configured proxy
                    // and we don't have a configured state machine client for the destination
                    res.dest_chain() == timeout_proof.height.id.state_id ||
                        host.is_allowed_proxy(&timeout_proof.height.id.state_id) &&
                            check_for_consensus_client(res.dest_chain())
                })
                .collect::<Vec<_>>();

            for response in &responses {
                // Ensure a commitment exists for all responses in the batch
                let commitment = hash_post_response::<H>(response);
                host.response_commitment(commitment)?;

                if response.timeout() > state.timestamp() {
                    Err(Error::RequestTimeoutNotElapsed {
                        nonce: response.nonce(),
                        source: response.source_chain(),
                        dest: response.dest_chain(),
                        timeout_timestamp: response.timeout(),
                        state_machine_time: state.timestamp(),
                    })?
                }
            }

            let items = responses.iter().map(|r| Into::into(r.clone())).collect::<Vec<Response>>();
            let keys = state_machine.state_trie_key(items.into());
            let values = state_machine.verify_state_proof(host, keys, state, &timeout_proof)?;
            if values.into_iter().any(|(_key, val)| val.is_some()) {
                Err(Error::ImplementationSpecific("Some Requests not timed out".into()))?
            }

            let router = host.ismp_router();
            responses
                .into_iter()
                .map(|response| {
                    let cb = router.module_for_id(response.source_module())?;
                    let res = cb
                        .on_timeout(response.clone().into())
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
                        host.delete_response_commitment(&response)?;
                        // If the response was routed we delete it's receipt
                        if host.host_state_machine() != response.source_chain() {
                            host.delete_response_receipt(&response)?;
                        }
                    }
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
        TimeoutMessage::Get { requests } => {
            for request in &requests {
                let commitment = hash_request::<H>(request);
                // if we have a commitment, it came from us
                host.request_commitment(commitment)?;

                // Ensure the get timeout has elapsed on the host
                if !request.timed_out(host.timestamp()) {
                    Err(Error::RequestTimeoutNotElapsed {
                        nonce: request.nonce(),
                        source: request.source_chain(),
                        dest: request.dest_chain(),
                        timeout_timestamp: request.timeout(),
                        state_machine_time: host.timestamp(),
                    })?
                }
            }
            let router = host.ismp_router();
            requests
                .into_iter()
                .map(|request| {
                    let cb = router.module_for_id(request.source_module())?;
                    let res = cb
                        .on_timeout(request.clone().into())
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
                    if res.is_ok() {
                        host.delete_request_commitment(&request)?;
                    }
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
    };

    Ok(MessageResult::Timeout(results))
}
