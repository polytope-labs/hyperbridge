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
    events::{Event, TimeoutHandled},
    handlers::{validate_state_machine, MessageResult},
    host::{IsmpHost, StateMachine},
    messaging::{hash_post_response, hash_request, TimeoutMessage},
    router::Response,
};
use alloc::vec::Vec;

/// This function handles timeouts
pub fn handle<H>(host: &H, msg: TimeoutMessage) -> Result<MessageResult, Error>
where
    H: IsmpHost,
{
    let consensus_clients = host.consensus_clients();

    let check_state_machine_client = |state_machine: StateMachine| {
        consensus_clients
            .iter()
            .find_map(|client| client.state_machine(state_machine).ok())
            .is_none()
    };

    let results = match msg {
        TimeoutMessage::Post { requests, timeout_proof } => {
            let state_machine = validate_state_machine(host, timeout_proof.height)?;
            let state = host.state_machine_commitment(timeout_proof.height)?;

            for request in &requests {
                // check if the destination chain does not match the proof metadata in which case
                // the proof metadata must be the configured proxy
                // and we must not have a configured state machine client for the destination
                if request.dest_chain() != timeout_proof.height.id.state_id &&
                    !(host.is_allowed_proxy(&timeout_proof.height.id.state_id) &&
                        check_state_machine_client(request.dest_chain()))
                {
                    Err(Error::RequestProxyProhibited { meta: request.into() })?
                }

                // Ensure a commitment exists for all requests in the batch
                let commitment = hash_request::<H>(request);
                if host.request_commitment(commitment).is_err() {
                    Err(Error::UnknownRequest { meta: request.into() })?
                }

                if !request.timed_out(state.timestamp()) {
                    Err(Error::RequestTimeoutNotElapsed {
                        meta: request.into(),
                        timeout_timestamp: request.timeout(),
                        state_machine_time: state.timestamp(),
                    })?
                }
            }

            let keys = state_machine.state_trie_key(requests.clone().into());
            let values = state_machine.verify_state_proof(host, keys, state, &timeout_proof)?;
            if values.into_iter().any(|(_key, val)| val.is_some()) {
                Err(Error::ImplementationSpecific(
                    "Some Requests in the batch have been delivered".into(),
                ))?
            }

            let router = host.ismp_router();
            requests
                .into_iter()
                .map(|request| {
                    let cb = router.module_for_id(request.source_module())?;
                    // Delete commitment to prevent rentrancy attack
                    let meta = host.delete_request_commitment(&request)?;
                    let mut signer = None;
                    // If it was a routed request delete the receipt
                    if host.host_state_machine() != request.source_chain() {
                        signer = host.delete_request_receipt(&request).ok();
                    }
                    let res = cb.on_timeout(request.clone().into()).map(|_| {
                        let commitment = hash_request::<H>(&request);
                        Event::PostRequestTimeoutHandled(TimeoutHandled { commitment })
                    });
                    // If module callback failed restore commitment so it can be retried
                    if res.is_err() {
                        host.store_request_commitment(&request, meta)?;
                        // If the request was routed we store it's receipt
                        if host.host_state_machine() != request.source_chain() && signer.is_some() {
                            host.store_request_receipt(&request, &signer.expect("Infaliible"))?;
                        }
                    }
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
        TimeoutMessage::PostResponse { responses, timeout_proof } => {
            let state_machine = validate_state_machine(host, timeout_proof.height)?;
            let state = host.state_machine_commitment(timeout_proof.height)?;
            for response in &responses {
                // check if the destination chain does not match the proof metadata in which case
                // the proof metadata must be the configured proxy
                // and we must not have a configured state machine client for the destination
                if response.dest_chain() != timeout_proof.height.id.state_id &&
                    !(host.is_allowed_proxy(&timeout_proof.height.id.state_id) &&
                        check_state_machine_client(response.dest_chain()))
                {
                    Err(Error::ResponseProxyProhibited {
                        meta: Response::Post(response.clone()).into(),
                    })?
                }
                // Ensure a commitment exists for all responses in the batch
                let commitment = hash_post_response::<H>(response);
                if host.response_commitment(commitment).is_err() {
                    Err(Error::UnknownResponse { meta: Response::Post(response.clone()).into() })?
                }

                if response.timeout() > state.timestamp() {
                    Err(Error::RequestTimeoutNotElapsed {
                        meta: response.into(),
                        timeout_timestamp: response.timeout(),
                        state_machine_time: state.timestamp(),
                    })?
                }
            }

            let items = responses.iter().map(|r| Into::into(r.clone())).collect::<Vec<Response>>();
            let keys = state_machine.state_trie_key(items.into());
            let values = state_machine.verify_state_proof(host, keys, state, &timeout_proof)?;
            if values.into_iter().any(|(_key, val)| val.is_some()) {
                Err(Error::ImplementationSpecific(
                    "Some responses in the batch have been delivered".into(),
                ))?
            }

            let router = host.ismp_router();
            responses
                .into_iter()
                .map(|response| {
                    let cb = router.module_for_id(response.source_module())?;
                    // Delete commitment to prevent rentrancy
                    let meta = host.delete_response_commitment(&response)?;
                    // If the response was routed we delete it's receipt
                    let mut signer = None;
                    if host.host_state_machine() != response.source_chain() {
                        signer =
                            host.delete_response_receipt(&Response::Post(response.clone())).ok();
                    }
                    let res = cb.on_timeout(response.clone().into()).map(|_| {
                        let commitment = hash_post_response::<H>(&response);
                        Event::PostResponseTimeoutHandled(TimeoutHandled { commitment })
                    });
                    // If module callback failed restore commitment so it can be retried
                    if res.is_err() {
                        host.store_response_commitment(&response, meta)?;
                        if host.host_state_machine() != response.source_chain() && signer.is_some()
                        {
                            host.store_response_receipt(
                                &Response::Post(response),
                                &signer.expect("Infallible"),
                            )?;
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
                if host.request_commitment(commitment).is_err() {
                    Err(Error::UnknownRequest { meta: request.into() })?
                }

                // Ensure the get timeout has elapsed on the host
                if !request.timed_out(host.timestamp()) {
                    Err(Error::RequestTimeoutNotElapsed {
                        meta: request.into(),
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
                    // Delete commitment to prevent reentrancy
                    let meta = host.delete_request_commitment(&request)?;
                    let res = cb.on_timeout(request.clone().into()).map(|_| {
                        let commitment = hash_request::<H>(&request);
                        Event::GetRequestTimeoutHandled(TimeoutHandled { commitment })
                    });
                    // If module callback failed, restore commitment so it can be retried
                    if res.is_err() {
                        host.store_request_commitment(&request, meta)?;
                    }
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
    };

    Ok(MessageResult::Timeout(results))
}
