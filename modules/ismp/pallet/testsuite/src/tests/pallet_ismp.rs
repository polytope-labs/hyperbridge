// Copyright (C) 2023 Polytope Labs.
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

#![cfg(test)]

use crate::runtime::*;
use frame_support::pallet_prelude::Hooks;
use pallet_ismp::{child_trie::RequestReceipts, dispatcher::Dispatcher, host::Host};

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ismp::{
    consensus::{StateMachineHeight, StateMachineId},
    host::{Ethereum, IsmpHost, StateMachine},
    messaging::{Proof, ResponseMessage, TimeoutMessage},
    router::{
        DispatchGet, DispatchRequest, GetResponse, IsmpDispatcher, Post, Request, RequestResponse,
    },
    util::hash_request,
};

use ismp::{messaging::Message, router::Response};
use ismp_testsuite::{
    check_challenge_period, check_client_expiry, missing_state_commitment_check,
    post_request_timeout_check, post_response_timeout_check, write_outgoing_commitments,
};

use sp_core::H256;

fn set_timestamp(now: Option<u64>) {
    Timestamp::set_timestamp(
        now.unwrap_or(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64),
    );
}

#[test]
fn dispatcher_should_write_receipts_for_outgoing_requests_and_responses() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp(Some(1));
        let host = Host::<Test>::default();
        let dispatcher = Dispatcher::<Test>::default();
        let post = Post {
            source: StateMachine::Kusama(2000),
            dest: host.host_state_machine(),
            nonce: 0,
            from: vec![0u8; 32],
            to: vec![0u8; 32],
            timeout_timestamp: 0,
            data: vec![0u8; 64],
        };

        let request_commitment = hash_request::<Host<Test>>(&Request::Post(post.clone()));
        RequestReceipts::<Test>::insert(request_commitment, &vec![0u8; 32]);
        write_outgoing_commitments(&host, &dispatcher).unwrap();
    })
}

#[test]
fn should_reject_updates_within_challenge_period() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp(None);
        let host = Host::<Test>::default();
        host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 1_000_000).unwrap();
        check_challenge_period(&host).unwrap()
    })
}

#[test]
fn should_reject_messages_for_frozen_state_machines() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp(None);
        let host = Host::<Test>::default();
        host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 1_000_000).unwrap();
        missing_state_commitment_check(&host).unwrap()
    })
}

#[test]
fn should_reject_expired_check_clients() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp(None);
        let host = Host::<Test>::default();
        host.store_unbonding_period(MOCK_CONSENSUS_STATE_ID, 1_000_000).unwrap();
        host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 1_000_000).unwrap();
        check_client_expiry(&host).unwrap()
    })
}

#[test]
fn should_handle_post_request_timeouts_correctly() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp(Some(0));
        let host = Host::<Test>::default();
        let dispatcher = Dispatcher::<Test>::default();
        host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 0).unwrap();
        post_request_timeout_check(&host, &dispatcher).unwrap()
    })
}

#[test]
fn should_handle_post_response_timeouts_correctly() {
    let mut ext = new_test_ext();

    ext.execute_with(|| {
        set_timestamp(None);
        let host = Host::<Test>::default();
        let dispatcher = Dispatcher::<Test>::default();
        host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 1_000_000).unwrap();
        post_response_timeout_check(&host, &dispatcher).unwrap()
    })
}

#[test]
fn should_handle_get_request_timeouts_correctly() {
    let mut ext = new_test_ext();
    ext.execute_with(|| {
        let host = Host::<Test>::default();
        setup_mock_client::<_, Test>(&host);
        host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 0).unwrap();
        let requests = (0..2)
            .into_iter()
            .map(|i| {
                let msg = DispatchGet {
                    dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
                    from: vec![0u8; 32],
                    keys: vec![vec![1u8; 32], vec![1u8; 32]],
                    height: 2,
                    timeout_timestamp: 1000,
                };

                let dispatcher = Dispatcher::<Test>::default();
                dispatcher
                    .dispatch_request(DispatchRequest::Get(msg), [0u8; 32].into(), 0u32.into())
                    .unwrap();
                let get = ismp::router::Get {
                    source: host.host_state_machine(),
                    dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
                    nonce: i,
                    from: vec![0u8; 32],
                    keys: vec![vec![1u8; 32], vec![1u8; 32]],
                    height: 2,
                    timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs() + 1000,
                };
                ismp::router::Request::Get(get)
            })
            .collect::<Vec<_>>();

        let timeout_msg = TimeoutMessage::Get { requests: requests.clone() };

        set_timestamp(Some(Duration::from_secs(100_000_000).as_millis() as u64));
        pallet_ismp::Pallet::<Test>::handle_messages(vec![Message::Timeout(timeout_msg)]).unwrap();
        for request in requests {
            // commitments should not be found in storage after timeout has been processed
            let commitment = hash_request::<Host<Test>>(&request);
            assert!(host.request_commitment(commitment).is_err())
        }
    })
}

#[test]
fn should_handle_get_request_responses_correctly() {
    let mut ext = new_test_ext();
    ext.execute_with(|| {
        let host = Host::<Test>::default();
        setup_mock_client::<_, Test>(&host);
        host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 0).unwrap();
        let requests = (0..2)
            .into_iter()
            .map(|i| {
                let msg = DispatchGet {
                    dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
                    from: vec![0u8; 32],

                    keys: vec![vec![1u8; 32], vec![1u8; 32]],
                    height: 3,
                    timeout_timestamp: 2_000_000_000,
                };

                let dispatcher = Dispatcher::<Test>::default();
                dispatcher
                    .dispatch_request(DispatchRequest::Get(msg), [0u8; 32].into(), 0u32.into())
                    .unwrap();
                let get = ismp::router::Get {
                    source: host.host_state_machine(),
                    dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
                    nonce: i,
                    from: vec![0u8; 32],
                    keys: vec![vec![1u8; 32], vec![1u8; 32]],
                    height: 3,
                    timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs() +
                        2_000_000_000,
                };
                ismp::router::Request::Get(get)
            })
            .collect::<Vec<_>>();

        set_timestamp(Some(Duration::from_secs(100_000_000).as_millis() as u64));

        let response = ResponseMessage {
            datagram: RequestResponse::Request(requests.clone()),
            proof: Proof {
                height: StateMachineHeight {
                    id: StateMachineId {
                        state_id: StateMachine::Ethereum(Ethereum::ExecutionLayer),
                        consensus_state_id: MOCK_CONSENSUS_STATE_ID,
                    },
                    height: 3,
                },
                proof: vec![],
            },
            signer: vec![],
        };

        pallet_ismp::Pallet::<Test>::handle_messages(vec![Message::Response(response)]).unwrap();

        for request in requests {
            let Request::Get(get) = request else { panic!("Shouldn't be possible") };
            let response = Response::Get(GetResponse { get, values: Default::default() });
            assert!(host.response_receipt(&response).is_some())
        }
    })
}

#[test]
fn ensure_that_we_commit_requests_to_the_mmr_alongside_state() {
    // this should prevent requests from being committed to the mmr without being committed to the
    // state which would allow requests to be both delivered to the destination and
    // simultaneosly time out on the host.
}
