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
    host::IsmpHost,
    messaging::TimeoutMessage,
    module::{DispatchError, DispatchSuccess},
    util::hash_request,
};
use alloc::{format, vec::Vec};

/// This function handles timeouts for Requests
pub fn handle<H>(host: &H, msg: TimeoutMessage) -> Result<MessageResult, Error>
where
    H: IsmpHost,
{
    let results = match msg {
        TimeoutMessage::Post { requests, timeout_proof } => {
            let state_machine = validate_state_machine(host, timeout_proof.height)?;
            let state = host.state_machine_commitment(timeout_proof.height)?;
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

            let key = state_machine.state_trie_key(requests.clone());

            let values = state_machine.verify_state_proof(host, key, state, &timeout_proof)?;

            if values.into_iter().any(|(_key, val)| val.is_some()) {
                Err(Error::ImplementationSpecific("Some Requests not timed out".into()))?
            }

            let router = host.ismp_router();
            requests
                .into_iter()
                .map(|request| {
                    let cb = router.module_for_id(request.source_module())?;
                    let res = cb
                        .on_timeout(request.clone())
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
                    host.delete_request_commitment(&request)?;
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
        TimeoutMessage::Get { requests } => {
            for request in &requests {
                let commitment = hash_request::<H>(request);
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
                        .on_timeout(request.clone())
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
                    host.delete_request_commitment(&request)?;
                    Ok(res)
                })
                .collect::<Result<Vec<_>, _>>()?
        },
    };

    Ok(MessageResult::Timeout(results))
}
