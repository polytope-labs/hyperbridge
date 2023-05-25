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

use crate::mocks::MOCK_CONSENSUS_CLIENT_ID;
use ismp::{
    consensus::{IntermediateState, StateCommitment, StateMachineHeight, StateMachineId},
    handlers::handle_incoming_message,
    host::{IsmpHost, StateMachine},
    messaging::{
        ConsensusMessage, Message, Proof, RequestMessage, ResponseMessage, TimeoutMessage,
    },
    router::{Post, PostResponse, Request, Response},
};

fn setup_mock_client<H: IsmpHost>(host: &H) -> IntermediateState {
    let intermediate_state = IntermediateState {
        height: StateMachineHeight {
            id: StateMachineId {
                state_id: StateMachine::Ethereum,
                consensus_client: MOCK_CONSENSUS_CLIENT_ID,
            },
            height: 1,
        },
        commitment: StateCommitment {
            timestamp: 1000,
            overlay_root: None,
            state_root: Default::default(),
        },
    };

    host.store_consensus_state(MOCK_CONSENSUS_CLIENT_ID, vec![]).unwrap();
    host.store_state_machine_commitment(intermediate_state.height, intermediate_state.commitment)
        .unwrap();

    intermediate_state
}
/*
    Consensus Client and State Machine checks
*/

/// Ensure challenge period rules are followed in all handlers
pub fn check_challenge_period<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
    let consensus_message = Message::Consensus(ConsensusMessage {
        consensus_proof: vec![],
        consensus_client_id: MOCK_CONSENSUS_CLIENT_ID,
    });
    let intermediate_state = setup_mock_client(host);
    // Set the previous update time
    let challenge_period = host.challenge_period(MOCK_CONSENSUS_CLIENT_ID);
    let previous_update_time = host.timestamp() - (challenge_period / 2);
    host.store_consensus_update_time(MOCK_CONSENSUS_CLIENT_ID, previous_update_time).unwrap();

    let res = handle_incoming_message::<H>(host, consensus_message);
    assert!(matches!(res, Err(ismp::error::Error::ChallengePeriodNotElapsed { .. })));

    let post = Post {
        source_chain: host.host_state_machine(),
        dest_chain: StateMachine::Kusama(2000),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
    };
    let request = Request::Post(post.clone());
    // Request message handling check
    let request_message = Message::Request(RequestMessage {
        requests: vec![request.clone()],
        proof: Proof { height: intermediate_state.height, proof: vec![] },
    });

    let res = handle_incoming_message(host, request_message);

    assert!(matches!(res, Err(ismp::error::Error::ChallengePeriodNotElapsed { .. })));

    // Response message handling check
    let response_message = Message::Response(ResponseMessage::Post {
        responses: vec![Response::Post(PostResponse { post, response: vec![] })],
        proof: Proof { height: intermediate_state.height, proof: vec![] },
    });

    let res = handle_incoming_message(host, response_message);
    assert!(matches!(res, Err(ismp::error::Error::ChallengePeriodNotElapsed { .. })));

    // Timeout mesaage handling check
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
        consensus_client_id: MOCK_CONSENSUS_CLIENT_ID,
    });
    setup_mock_client(host);
    // Set the previous update time
    let client = host.consensus_client(MOCK_CONSENSUS_CLIENT_ID).unwrap();
    let unbonding_period = client.unbonding_period();
    let previous_update_time = host.timestamp() - unbonding_period;
    host.store_consensus_update_time(MOCK_CONSENSUS_CLIENT_ID, previous_update_time).unwrap();

    let res = handle_incoming_message::<H>(host, consensus_message);
    assert!(matches!(res, Err(ismp::error::Error::UnbondingPeriodElapsed { .. })));

    Ok(())
}

/// Frozen state machine checks in message handlers
pub fn frozen_check<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
    let intermediate_state = setup_mock_client(host);
    // Set the previous update time
    let challenge_period = host.challenge_period(MOCK_CONSENSUS_CLIENT_ID);
    let previous_update_time = host.timestamp() - (challenge_period * 2);
    host.store_consensus_update_time(MOCK_CONSENSUS_CLIENT_ID, previous_update_time).unwrap();

    let frozen_height = StateMachineHeight {
        id: intermediate_state.height.id,
        height: intermediate_state.height.height - 1,
    };
    host.freeze_state_machine(frozen_height).unwrap();

    let post = Post {
        source_chain: host.host_state_machine(),
        dest_chain: StateMachine::Kusama(2000),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
    };
    let request = Request::Post(post.clone());
    // Request message handling check
    let request_message = Message::Request(RequestMessage {
        requests: vec![request.clone()],
        proof: Proof { height: intermediate_state.height, proof: vec![] },
    });

    let res = handle_incoming_message(host, request_message);

    assert!(matches!(res, Err(ismp::error::Error::FrozenStateMachine { .. })));

    // Response message handling check
    let response_message = Message::Response(ResponseMessage::Post {
        responses: vec![Response::Post(PostResponse { post, response: vec![] })],
        proof: Proof { height: intermediate_state.height, proof: vec![] },
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

/// Ensure all timeout post processing is correctly done.
pub fn timeout_post_processing_check<H: IsmpHost>(host: &H) -> Result<(), &'static str> {
    let intermediate_state = setup_mock_client(host);
    let challenge_period = host.challenge_period(MOCK_CONSENSUS_CLIENT_ID);
    let previous_update_time = host.timestamp() - (challenge_period * 2);
    host.store_consensus_update_time(MOCK_CONSENSUS_CLIENT_ID, previous_update_time).unwrap();

    let post = Post {
        source_chain: host.host_state_machine(),
        dest_chain: StateMachine::Kusama(2000),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: intermediate_state.commitment.timestamp,
        data: vec![0u8; 64],
    };
    let request = Request::Post(post);
    let router = host.ismp_router();
    router.handle_request(request.clone()).unwrap();

    // Timeout mesaage handling check
    let timeout_message = Message::Timeout(TimeoutMessage::Post {
        requests: vec![request.clone()],
        timeout_proof: Proof { height: intermediate_state.height, proof: vec![] },
    });

    handle_incoming_message(host, timeout_message).unwrap();

    // Assert that request commitment was deleted
    let commitment = host.request_commitment(&request);
    assert!(matches!(commitment, Err(..)));
    Ok(())
}

/*
    Check correctness of router implementation
*/

/// Check that router stores commitments for outgoing requests and responses and rejects duplicates
pub fn write_outgoing_commitments(host: &dyn IsmpHost) -> Result<(), &'static str> {
    let router = host.ismp_router();
    let post = Post {
        source_chain: host.host_state_machine(),
        dest_chain: StateMachine::Kusama(2000),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
    };
    let request = Request::Post(post);
    // Dispatch the request the first time
    router.handle_request(request.clone()).map_err(|_| "Router failed to dispatch request")?;
    // Fetch commitment from storage
    host.request_commitment(&request)
        .map_err(|_| "Expected Request commitment to be found in storage")?;
    // Dispatch the same request a second time
    let err = router.handle_request(request);
    assert!(err.is_err(), "Expected router to return error for duplicate request");
    let post = Post {
        source_chain: StateMachine::Kusama(2000),
        dest_chain: host.host_state_machine(),
        nonce: 0,
        from: vec![0u8; 32],
        to: vec![0u8; 32],
        timeout_timestamp: 0,
        data: vec![0u8; 64],
    };
    let response = Response::Post(PostResponse { post, response: vec![] });
    // Dispatch the outgoing response for the first time
    router.handle_response(response.clone()).map_err(|_| "Router failed to dispatch request")?;
    // Dispatch the same response a second time
    let err = router.handle_response(response);
    assert!(err.is_err(), "Expected router to return error for duplicate response");

    Ok(())
}
