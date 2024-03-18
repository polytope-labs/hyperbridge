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

//! ISMP Testsuite

pub mod mocks;
#[cfg(test)]
mod tests;

use crate::mocks::{MOCK_CONSENSUS_CLIENT_ID, MOCK_PROXY_CONSENSUS_CLIENT_ID};
use ismp::{
    consensus::{
        ConsensusStateId, IntermediateState, StateCommitment, StateMachineHeight, StateMachineId,
    },
    handlers::handle_incoming_message,
    host::{Ethereum, IsmpHost, StateMachine},
    messaging::{
        ConsensusMessage, Message, Proof, RequestMessage, ResponseMessage, TimeoutMessage,
    },
    router::{
        DispatchPost, DispatchRequest, IsmpDispatcher, Post, PostResponse, Request,
        RequestResponse, Response,
    },
    util::{hash_post_response, hash_request, hash_response},
};

fn mock_consensus_state_id() -> ConsensusStateId {
    *b"mock"
}

fn mock_proxy_consensus_state_id() -> ConsensusStateId {
    *b"prxy"
}

fn setup_mock_client<H: IsmpHost>(host: &H) -> IntermediateState {
    let intermediate_state = IntermediateState {
        height: StateMachineHeight {
            id: StateMachineId {
                state_id: StateMachine::Ethereum(Ethereum::ExecutionLayer),
                consensus_state_id: mock_consensus_state_id(),
            },
            height: 1,
        },
        commitment: StateCommitment {
            timestamp: 1000,
            overlay_root: None,
            state_root: Default::default(),
        },
    };

    host.store_consensus_state(mock_consensus_state_id(), vec![]).unwrap();
    host.store_consensus_state_id(mock_consensus_state_id(), MOCK_CONSENSUS_CLIENT_ID)
        .unwrap();
    host.store_state_machine_commitment(intermediate_state.height, intermediate_state.commitment)
        .unwrap();

    intermediate_state
}

fn setup_mock_proxy_client<H: IsmpHost>(host: &H) -> IntermediateState {
    let proxy_state_commitment = IntermediateState {
        height: StateMachineHeight {
            id: StateMachineId {
                state_id: StateMachine::Kusama(2000),
                consensus_state_id: mock_proxy_consensus_state_id(),
            },
            height: 1,
        },
        commitment: StateCommitment {
            timestamp: 1000,
            overlay_root: None,
            state_root: Default::default(),
        },
    };

    host.store_consensus_state(mock_proxy_consensus_state_id(), vec![]).unwrap();
    host.store_consensus_state_id(mock_proxy_consensus_state_id(), MOCK_PROXY_CONSENSUS_CLIENT_ID)
        .unwrap();

    proxy_state_commitment
}
/*
    Consensus Client and State Machine checks
*/

/// Ensure challenge period rules are followed in all handlers
pub fn check_challenge_period<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
    let consensus_message = Message::Consensus(ConsensusMessage {
        consensus_proof: vec![],
        consensus_state_id: mock_consensus_state_id(),
    });
    let intermediate_state = setup_mock_client(host);
    // Set the previous update time
    let challenge_period = host.challenge_period(mock_consensus_state_id()).unwrap();
    let previous_update_time = host.timestamp() - (challenge_period / 2);
    host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
        .unwrap();
    host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
        .unwrap();
    let res = handle_incoming_message::<H>(host, consensus_message);
    assert!(matches!(res, Err(ismp::error::Error::ChallengePeriodNotElapsed { .. })));

    let post = Post {
        source: host.host_state_machine(),
        dest: StateMachine::Kusama(2000),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
        gas_limit: 0,
    };
    let request = Request::Post(post.clone());
    // Request message handling check
    let request_message = Message::Request(RequestMessage {
        requests: vec![post.clone()],
        proof: Proof { height: intermediate_state.height, proof: vec![] },
        signer: vec![],
    });

    let res = handle_incoming_message(host, request_message);

    assert!(matches!(res, Err(ismp::error::Error::ChallengePeriodNotElapsed { .. })));

    // Response message handling check
    let response_message = Message::Response(ResponseMessage {
        datagram: RequestResponse::Response(vec![Response::Post(PostResponse {
            post,
            response: vec![],
            timeout_timestamp: 0,
            gas_limit: 0,
        })]),
        proof: Proof { height: intermediate_state.height, proof: vec![] },
        signer: vec![],
    });

    let res = handle_incoming_message(host, response_message);
    assert!(matches!(res, Err(ismp::error::Error::ChallengePeriodNotElapsed { .. })));

    // Timeout message handling check
    let timeout_message = Message::Timeout(TimeoutMessage::Post {
        requests: vec![request],
        timeout_proof: Proof { height: intermediate_state.height, proof: vec![] },
    });

    let res = handle_incoming_message(host, timeout_message);
    assert!(matches!(res, Err(ismp::error::Error::ChallengePeriodNotElapsed { .. })));
    Ok(())
}

/// Ensure expired client rules are followed in consensus update
pub fn check_client_expiry<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
    let consensus_message = Message::Consensus(ConsensusMessage {
        consensus_proof: vec![],
        consensus_state_id: mock_consensus_state_id(),
    });
    setup_mock_client(host);
    // Set the previous update time
    let unbonding_period = host.unbonding_period(mock_consensus_state_id()).unwrap();
    let previous_update_time = host.timestamp() - unbonding_period;
    host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
        .unwrap();

    let res = handle_incoming_message::<H>(host, consensus_message);
    assert!(matches!(res, Err(ismp::error::Error::UnbondingPeriodElapsed { .. })));

    Ok(())
}

/// Frozen state machine checks in message handlers
pub fn frozen_check<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
    let intermediate_state = setup_mock_client(host);
    // Set the previous update time
    let challenge_period = host.challenge_period(mock_consensus_state_id()).unwrap();
    let previous_update_time = host.timestamp() - (challenge_period * 2);
    host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
        .unwrap();
    host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
        .unwrap();
    host.freeze_state_machine(intermediate_state.height.id).unwrap();

    let post = Post {
        source: host.host_state_machine(),
        dest: StateMachine::Kusama(2000),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
        gas_limit: 0,
    };
    let request = Request::Post(post.clone());
    // Request message handling check
    let request_message = Message::Request(RequestMessage {
        requests: vec![post.clone()],
        proof: Proof { height: intermediate_state.height, proof: vec![] },
        signer: vec![],
    });

    let res = handle_incoming_message(host, request_message);

    assert!(matches!(res, Err(ismp::error::Error::FrozenStateMachine { .. })));

    // Response message handling check
    let response_message = Message::Response(ResponseMessage {
        datagram: RequestResponse::Response(vec![Response::Post(PostResponse {
            post,
            response: vec![],
            timeout_timestamp: 0,
            gas_limit: 0,
        })]),
        proof: Proof { height: intermediate_state.height, proof: vec![] },
        signer: vec![],
    });

    let res = handle_incoming_message(host, response_message);
    assert!(matches!(res, Err(ismp::error::Error::FrozenStateMachine { .. })));

    // Timeout mesaage handling check
    let timeout_message = Message::Timeout(TimeoutMessage::Post {
        requests: vec![request],
        timeout_proof: Proof { height: intermediate_state.height, proof: vec![] },
    });

    let res = handle_incoming_message(host, timeout_message);
    assert!(matches!(res, Err(ismp::error::Error::FrozenStateMachine { .. })));

    Ok(())
}

/// Ensure post request timeouts are handled properly
pub fn post_request_timeout_check<H, D>(host: &H, dispatcher: &D) -> Result<(), &'static str>
where
    H: IsmpHost,
    D: IsmpDispatcher,
    D::Account: From<[u8; 32]>,
    D::Balance: From<u32>,
{
    let intermediate_state = setup_mock_client(host);
    let challenge_period = host.challenge_period(mock_consensus_state_id()).unwrap();
    let previous_update_time = host.timestamp().saturating_sub(challenge_period * 2);
    host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
        .unwrap();
    host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
        .unwrap();
    let dispatch_post = DispatchPost {
        dest: intermediate_state.height.id.state_id,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: intermediate_state.commitment.timestamp,
        data: vec![0u8; 64],
        gas_limit: 0,
    };
    let post = Post {
        source: host.host_state_machine(),
        dest: StateMachine::Kusama(2000),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: intermediate_state.commitment.timestamp,
        data: vec![0u8; 64],
        gas_limit: 0,
    };
    let request = Request::Post(post);
    let dispatch_request = DispatchRequest::Post(dispatch_post);
    dispatcher
        .dispatch_request(dispatch_request, [0; 32].into(), 0u32.into())
        .unwrap();

    // Timeout message handling check
    let timeout_message = Message::Timeout(TimeoutMessage::Post {
        requests: vec![request.clone()],
        timeout_proof: Proof { height: intermediate_state.height, proof: vec![] },
    });

    handle_incoming_message(host, timeout_message).unwrap();

    // Assert that request commitment was deleted
    let commitment = hash_request::<H>(&request);
    let res = host.request_commitment(commitment);
    assert!(matches!(res, Err(..)));
    Ok(())
}

/// Ensure post request timeouts are handled properly
pub fn post_response_timeout_check<H, D>(host: &H, dispatcher: &D) -> Result<(), &'static str>
where
    H: IsmpHost,
    D: IsmpDispatcher,
    D::Account: From<[u8; 32]>,
    D::Balance: From<u32>,
{
    let intermediate_state = setup_mock_client(host);
    let challenge_period = host.challenge_period(mock_consensus_state_id()).unwrap();
    let previous_update_time = host.timestamp() - (challenge_period * 2);
    host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
        .unwrap();
    host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
        .unwrap();

    let request = Post {
        source: intermediate_state.height.id.state_id,
        dest: host.host_state_machine(),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
        gas_limit: 0,
    };

    let request_message = Message::Request(RequestMessage {
        requests: vec![request.clone()],
        proof: Proof { height: intermediate_state.height, proof: vec![] },
        signer: vec![],
    });

    handle_incoming_message(host, request_message).unwrap();
    // Assert that request was acknowledged
    assert!(matches!(host.request_receipt(&Request::Post(request.clone())), Some(_)));

    let response =
        PostResponse { post: request, response: vec![], timeout_timestamp: 100, gas_limit: 0 };
    dispatcher
        .dispatch_response(response.clone(), [0; 32].into(), 0u32.into())
        .unwrap();

    let timeout_message = Message::Timeout(TimeoutMessage::PostResponse {
        responses: vec![response.clone()],
        timeout_proof: Proof { height: intermediate_state.height, proof: vec![] },
    });

    handle_incoming_message(host, timeout_message).unwrap();

    // Assert that response commitment was deleted
    let commitment = hash_post_response::<H>(&response);
    let res = host.response_commitment(commitment);
    assert!(matches!(res, Err(..)));
    Ok(())
}

/*
    Check correctness of router implementation
*/

/// Check that dispatcher stores commitments for outgoing requests and responses and rejects
/// duplicate responses
pub fn write_outgoing_commitments<H, D>(host: &H, dispatcher: &D) -> Result<(), &'static str>
where
    H: IsmpHost,
    D: IsmpDispatcher,
    D::Account: From<[u8; 32]>,
    D::Balance: From<u32>,
{
    let post = DispatchPost {
        dest: StateMachine::Kusama(2000),
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
        gas_limit: 0,
    };
    let dispatch_request = DispatchRequest::Post(post);
    // Dispatch the request the first time
    dispatcher
        .dispatch_request(dispatch_request, [0; 32].into(), 0u32.into())
        .map_err(|_| "Dispatcher failed to dispatch request")?;
    // Fetch commitment from storage
    let post = Post {
        source: host.host_state_machine(),
        dest: StateMachine::Kusama(2000),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
        gas_limit: 0,
    };
    let request = Request::Post(post);
    let commitment = hash_request::<H>(&request);
    host.request_commitment(commitment)
        .map_err(|_| "Expected Request commitment to be found in storage")?;
    let post = Post {
        source: StateMachine::Kusama(2000),
        dest: host.host_state_machine(),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
        gas_limit: 0,
    };
    let response = PostResponse { post, response: vec![], timeout_timestamp: 0, gas_limit: 0 };
    // Dispatch the outgoing response for the first time
    dispatcher
        .dispatch_response(response.clone(), [0; 32].into(), 0u32.into())
        .map_err(|_| "Router failed to dispatch request")?;
    // Dispatch the same response a second time
    let err = dispatcher.dispatch_response(response, [0; 32].into(), 0u32.into());
    assert!(err.is_err(), "Expected router to return error for duplicate response");

    Ok(())
}

/// This should prevent a request from timing out on a proxy when there exists a consensus client
/// for the request destination
pub fn prevent_request_timeout_on_proxy_with_known_state_machine() {}

/// This should prevent a response from timing out on a proxy when there exists a consensus client
/// for the request destination
pub fn prevent_response_timeout_on_proxy_with_known_state_machine() {}

/// This should check that if a proxy isn't configured, requests are not valid if they don't come
/// from the state machine claimed in the proof as well as check that the request destination
/// matches the host state machine.
pub fn check_request_source_and_destination() {}

/// This should check that if a proxy isn't configured, responses are not valid if they don't come
/// from the state machine claimed in the proof
pub fn check_response_source<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
    let intermediate_state = setup_mock_client(host);
    // Set the previous update time
    let challenge_period = host.challenge_period(mock_consensus_state_id()).unwrap();
    let previous_update_time = host.timestamp() - (challenge_period * 2);
    host.store_consensus_update_time(mock_consensus_state_id(), previous_update_time)
        .unwrap();
    host.store_state_machine_update_time(intermediate_state.height, previous_update_time)
        .unwrap();

    assert!(host.allowed_proxy().is_none());

    let post = Post {
        source: intermediate_state.height.id.state_id,
        dest: host.host_state_machine(),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
        gas_limit: 0,
    };

    // Request message handling check for source and destination chain
    let request_message = Message::Request(RequestMessage {
        requests: vec![post.clone()],
        proof: Proof { height: intermediate_state.height, proof: vec![] },
        signer: vec![0u8; 32],
    });

    handle_incoming_message(host, request_message).unwrap();
    // Assert that request was acknowledged
    assert!(host.request_receipt(&Request::Post(post)).is_some());

    let post = Post {
        source: StateMachine::Kusama(2000),
        dest: host.host_state_machine(),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
        gas_limit: 0,
    };
    let response =
        PostResponse { post: post.clone(), response: vec![], timeout_timestamp: 0, gas_limit: 0 };
    // Response message handling check
    let response_message = Message::Response(ResponseMessage {
        datagram: RequestResponse::Response(vec![Response::Post(response.clone())]),
        proof: Proof { height: intermediate_state.height, proof: vec![] },
        signer: vec![],
    });

    handle_incoming_message(host, response_message).unwrap();
    // Assert that response is not acknowledged
    assert!(host.response_receipt(&Response::Post(response)).is_none());
    Ok(())
}

/// Check that proxies can dispatch requests & responses.
pub fn sanity_check_for_proxies<H, D>(host: &H, dispatcher: &D) -> Result<(), &'static str>
where
    H: IsmpHost,
    D: IsmpDispatcher,
    D::Account: From<[u8; 32]>,
    D::Balance: From<u32>,
{
    let proxy_state = setup_mock_proxy_client(host);
    // Assert that a proxy is configured
    assert!(host.allowed_proxy().is_some());

    let post = DispatchPost {
        dest: StateMachine::Polygon,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
        gas_limit: 0,
    };
    let dispatch_request = DispatchRequest::Post(post);
    // Dispatch the request the first time
    dispatcher
        .dispatch_request(dispatch_request, [0; 32].into(), 0u32.into())
        .map_err(|_| "Dispatcher failed to dispatch request")?;
    // Fetch commitment from storage
    let post = Post {
        source: proxy_state.height.id.state_id,
        dest: StateMachine::Polygon,
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
        gas_limit: 0,
    };
    let request = Request::Post(post.clone());
    assert_ne!(request.dest_chain(), host.host_state_machine());

    let commitment = hash_request::<H>(&request);
    host.request_commitment(commitment)
        .map_err(|_| "Expected Request commitment to be found in storage")?;

    let response = PostResponse { post, response: vec![], timeout_timestamp: 0, gas_limit: 0 };
    // Dispatch the outgoing response for the first time
    dispatcher
        .dispatch_response(response.clone(), [0; 32].into(), 0u32.into())
        .map_err(|_| "Router failed to dispatch request")?;
    let commitment = hash_response::<H>(&Response::Post(response));
    host.response_commitment(commitment)
        .map_err(|_| "Expected Request commitment to be found in storage")?;

    Ok(())
}
