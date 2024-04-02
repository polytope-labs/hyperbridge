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
use pallet_ismp::{
    dispatcher::{Dispatcher, FeeMetadata},
    host::Host,
    mmr::primitives::{DataOrHash, MmrHasher},
    IntermediateLeaves, NodesUtils, ProofKeys, RequestCommitments, RequestReceipts,
};

use std::{
    ops::Range,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

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
    check_challenge_period, check_client_expiry, frozen_check, post_request_timeout_check,
    post_response_timeout_check, write_outgoing_commitments,
};
use merkle_mountain_range::MerkleProof;
use sp_core::{crypto::AccountId32, H256};

fn on_initialize() {
    let number = frame_system::Pallet::<Test>::block_number() + 1;
    let hash = H256::repeat_byte(number as u8);

    frame_system::Pallet::<Test>::reset_events();
    frame_system::Pallet::<Test>::initialize(&number, &hash, &Default::default());
    Ismp::on_initialize(number);
}

fn push_leaves(range: Range<u64>) -> (Vec<H256>, Vec<u64>) {
    // given
    let mut commitments = vec![];
    let mut positions = vec![];
    for nonce in range {
        let post = Post {
            source: StateMachine::Kusama(2000),
            dest: StateMachine::Kusama(2001),
            nonce,
            from: vec![0u8; 32],
            to: vec![18; 32],
            timeout_timestamp: 100 * nonce,
            data: vec![2u8; 64],
            gas_limit: 0,
        };

        let request = Request::Post(post);
        let commitment = hash_request::<Host<Test>>(&request);

        pallet_ismp::Pallet::<Test>::dispatch_request(
            request,
            FeeMetadata { origin: AccountId32::new([0u8; 32]), fee: 10u128 },
        )
        .unwrap();
        let metadata = RequestCommitments::<Test>::get(commitment).expect("Should exist");
        positions.push(metadata.mmr.pos);
        commitments.push(commitment)
    }

    (commitments, positions)
}

#[test]
fn should_generate_proofs_correctly_for_single_leaf_mmr() {
    let _ = env_logger::try_init();
    let mut ext = new_test_ext();
    let (root, (commitments, positions)) = ext.execute_with(|| {
        on_initialize();
        // push some leaves into the mmr
        let positions = push_leaves(0..12);
        Ismp::on_finalize(frame_system::Pallet::<Test>::block_number() + 1);
        let root = pallet_ismp::Pallet::<Test>::mmr_root();
        (root, positions)
    });
    ext.persist_offchain_overlay();

    // Try to generate proofs now. This requires the offchain extensions to be present
    // to retrieve full leaf data.

    ext.execute_with(move || {
        let (leaves, proof) =
            pallet_ismp::Pallet::<Test>::generate_proof(ProofKeys::Requests(vec![commitments[0]]))
                .unwrap();
        let mmr_size = NodesUtils::new(proof.leaf_count).size();
        let nodes = proof.items.into_iter().map(|h| DataOrHash::Hash(h.into())).collect();
        let proof = MerkleProof::<DataOrHash, MmrHasher<Host<Test>>>::new(mmr_size, nodes);
        let calculated_root = proof
            .calculate_root(vec![(positions[0], DataOrHash::Data(leaves[0].clone()))])
            .unwrap();

        assert_eq!(root, calculated_root.hash::<Host<Test>>());

        assert_eq!(IntermediateLeaves::<Test>::count(), 0);
    })
}

#[test]
fn should_generate_and_verify_batch_proof_correctly() {
    let _ = env_logger::try_init();
    let mut ext = new_test_ext();
    // Try to generate proofs now. This requires the offchain extensions to be present
    // to retrieve full leaf data.
    let (root, (commitments, positions)) = ext.execute_with(|| {
        on_initialize();
        // push some leaves into the mmr
        let positions = push_leaves(0..12);
        Ismp::on_finalize(frame_system::Pallet::<Test>::block_number() + 1);
        let root = pallet_ismp::Pallet::<Test>::mmr_root();
        (root, positions)
    });
    ext.persist_offchain_overlay();

    ext.execute_with(move || {
        let proof_key = ProofKeys::Requests(vec![
            commitments[0],
            commitments[3],
            commitments[2],
            commitments[5],
        ]);
        let indices = vec![positions[0], positions[3], positions[2], positions[5]];
        let (leaves, proof) = pallet_ismp::Pallet::<Test>::generate_proof(proof_key).unwrap();

        let mmr_size = NodesUtils::new(proof.leaf_count).size();
        let nodes = proof.items.into_iter().map(|h| DataOrHash::Hash(h.into())).collect();
        let proof = MerkleProof::<DataOrHash, MmrHasher<Host<Test>>>::new(mmr_size, nodes);
        let calculated_root = proof
            .calculate_root(
                indices
                    .into_iter()
                    .zip(leaves.into_iter().map(|leaf| DataOrHash::Data(leaf)))
                    .collect(),
            )
            .unwrap();

        assert_eq!(root, calculated_root.hash::<Host<Test>>());
        assert_eq!(IntermediateLeaves::<Test>::count(), 0);
    })
}

#[test]
fn should_generate_and_verify_batch_proof_for_leaves_inserted_across_multiple_blocks_correctly() {
    let _ = env_logger::try_init();
    let mut ext = new_test_ext();
    // Try to generate proofs now. This requires the offchain extensions to be present
    // to retrieve full leaf data.
    let (root, (commitments, positions)) = ext.execute_with(|| {
        on_initialize();
        // push some leaves into the mmr
        let (mut commitments, mut positions) = push_leaves(0..100);
        Ismp::on_finalize(frame_system::Pallet::<Test>::block_number() + 1);
        on_initialize();
        let (commitments_second, positions_second) = push_leaves(100..200);
        Ismp::on_finalize(frame_system::Pallet::<Test>::block_number() + 1);
        let root = pallet_ismp::Pallet::<Test>::mmr_root();
        positions.extend_from_slice(&positions_second);
        commitments.extend_from_slice(&commitments_second);
        (root, (commitments, positions))
    });
    ext.persist_offchain_overlay();

    ext.execute_with(move || {
        let indices = vec![positions[0], positions[120], positions[50], positions[195]];
        let proof_key = ProofKeys::Requests(vec![
            commitments[0],
            commitments[120],
            commitments[50],
            commitments[195],
        ]);
        let (leaves, proof) = pallet_ismp::Pallet::<Test>::generate_proof(proof_key).unwrap();

        let mmr_size = NodesUtils::new(proof.leaf_count).size();
        let nodes = proof.items.into_iter().map(|h| DataOrHash::Hash(h.into())).collect();
        let proof = MerkleProof::<DataOrHash, MmrHasher<Host<Test>>>::new(mmr_size, nodes);
        let calculated_root = proof
            .calculate_root(
                indices
                    .into_iter()
                    .zip(leaves.into_iter().map(|leaf| DataOrHash::Data(leaf)))
                    .collect(),
            )
            .unwrap();

        assert_eq!(root, calculated_root.hash::<Host<Test>>());
        assert_eq!(IntermediateLeaves::<Test>::count(), 0);

        dbg!(NodesUtils::new(1_000_000_000).size() * 32);
    })
}

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
            gas_limit: 0,
        };

        let request_commitment = hash_request::<Host<Test>>(&Request::Post(post.clone()));
        RequestReceipts::<Test>::insert(request_commitment, &vec![0u8; 32]);
        write_outgoing_commitments(&host, &dispatcher).unwrap();
    })
}

#[test]
#[ignore]
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
        frozen_check(&host).unwrap()
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
                    gas_limit: 0,
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
                    gas_limit: 0,
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
                    gas_limit: 0,

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
                    gas_limit: 0,
                    keys: vec![vec![1u8; 32], vec![1u8; 32]],
                    height: 3,
                    timeout_timestamp: Duration::from_millis(Timestamp::now()).as_secs()
                        + 2_000_000_000,
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
