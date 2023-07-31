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

//! Benchmarking
// Only enable this module for benchmarking.
#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

/// Running the benchmarks correctly.
/// Add the [`crate::ismp_mocks::MockConsensusClient`] as one of the consensus clients available to
/// pallet-ismp in the runtime configuration.
/// In your module router configuration add the [`crate::ismp_mocks::MockModule`] as one of the ismp
/// modules using the [`crate::ismp_mocks::ModuleId`] as it's module id
#[benchmarks(
where
<T as frame_system::Config>::Hash: From<H256>,
T: pallet_timestamp::Config,
<T as pallet_timestamp::Config>::Moment: From<u64>
)]
pub mod benchmarks {
    use super::*;
    use crate::{
        dispatcher::Dispatcher,
        host::Host,
        mocks::ismp::{setup_mock_client, MOCK_CONSENSUS_STATE_ID, MODULE_ID},
        Config, Event, Pallet, RequestCommitments, RequestReceipts, ResponseReceipts,
    };
    use frame_support::traits::{Get, Hooks};
    use frame_system::EventRecord;
    use ismp_primitives::{mmr::Leaf, LeafIndexQuery};
    use ismp_rs::{
        consensus::{StateCommitment, StateMachineId},
        host::{Ethereum, StateMachine},
        messaging::{
            CreateConsensusState, Message, Proof, RequestMessage, ResponseMessage,
            StateCommitmentHeight, TimeoutMessage,
        },
        router::{
            DispatchGet, DispatchPost, DispatchRequest, IsmpDispatcher, Post, PostResponse,
            Request, Response,
        },
        util::hash_request,
    };

    /// Verify the the last event emitted
    fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
        let events = frame_system::Pallet::<T>::events();
        let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
        let EventRecord { event, .. } = &events[events.len() - 1];
        assert_eq!(event, &system_event);
    }

    #[benchmark]
    fn create_consensus_client() {
        let message = CreateConsensusState {
            consensus_state: Default::default(),
            consensus_client_id: MOCK_CONSENSUS_STATE_ID,
            consensus_state_id: MOCK_CONSENSUS_STATE_ID,
            unbonding_period: u64::MAX,
            challenge_period: 0,
            state_machine_commitments: vec![(
                StateMachineId {
                    state_id: StateMachine::Ethereum(Ethereum::ExecutionLayer),
                    consensus_state_id: MOCK_CONSENSUS_STATE_ID,
                },
                StateCommitmentHeight {
                    commitment: StateCommitment {
                        timestamp: 1651280681,
                        overlay_root: None,
                        state_root: Default::default(),
                    },
                    height: 1,
                },
            )],
        };

        #[extrinsic_call]
        _(RawOrigin::Root, message);

        assert_last_event::<T>(
            Event::ConsensusClientCreated { consensus_client_id: MOCK_CONSENSUS_STATE_ID }.into(),
        );
    }

    // The Benchmark consensus client should be added to the runtime for these benchmarks to work
    #[benchmark]
    fn handle_request_message() {
        let host = Host::<T>::default();
        host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 60 * 60).unwrap();
        let height = setup_mock_client::<_, T>(&host);
        let post = Post {
            source: StateMachine::Ethereum(Ethereum::ExecutionLayer),
            dest: <T as Config>::StateMachine::get(),
            nonce: 0,
            from: MODULE_ID.to_bytes(),
            to: MODULE_ID.to_bytes(),
            timeout_timestamp: 5000,
            data: "handle_request_message".as_bytes().to_vec(),
            gas_limit: 0,
        };

        let msg =
            RequestMessage { requests: vec![post.clone()], proof: Proof { height, proof: vec![] } };
        let caller = whitelisted_caller();

        #[extrinsic_call]
        handle(RawOrigin::Signed(caller), vec![Message::Request(msg)]);

        let commitment = hash_request::<Host<T>>(&Request::Post(post));
        assert!(RequestReceipts::<T>::get(commitment).is_some());
    }

    #[benchmark]
    fn handle_response_message() {
        let host = Host::<T>::default();
        host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 60 * 60).unwrap();
        let height = setup_mock_client::<_, T>(&host);
        let post = Post {
            source: <T as Config>::StateMachine::get(),
            dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
            nonce: 0,
            from: MODULE_ID.to_bytes(),
            to: MODULE_ID.to_bytes(),
            timeout_timestamp: 5000,
            data: "handle_response_message".as_bytes().to_vec(),
            gas_limit: 0,
        };
        let request = Request::Post(post.clone());

        let commitment = hash_request::<Host<T>>(&request);
        RequestCommitments::<T>::insert(
            commitment,
            LeafIndexQuery { source_chain: post.source, dest_chain: post.dest, nonce: post.nonce },
        );

        let response = Response::Post(PostResponse { post, response: vec![] });
        let request_commitment = hash_request::<Host<T>>(&response.request());
        let msg = ResponseMessage::Post {
            responses: vec![response],
            proof: Proof { height, proof: vec![] },
        };

        let caller = whitelisted_caller();

        #[extrinsic_call]
        handle(RawOrigin::Signed(caller), vec![Message::Response(msg)]);

        assert!(ResponseReceipts::<T>::get(request_commitment).is_some());
    }

    #[benchmark]
    fn handle_timeout_message() {
        let host = Host::<T>::default();
        host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 60 * 60).unwrap();
        let height = setup_mock_client::<_, T>(&host);
        let post = Post {
            source: <T as Config>::StateMachine::get(),
            dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
            nonce: 0,
            from: MODULE_ID.to_bytes(),
            to: MODULE_ID.to_bytes(),
            timeout_timestamp: 500,
            data: "handle_timeout_message".as_bytes().to_vec(),
            gas_limit: 0,
        };
        let request = Request::Post(post.clone());

        let commitment = hash_request::<Host<T>>(&request);
        RequestCommitments::<T>::insert(
            commitment,
            LeafIndexQuery { source_chain: post.source, dest_chain: post.dest, nonce: post.nonce },
        );

        let msg = TimeoutMessage::Post {
            requests: vec![request],
            timeout_proof: Proof { height, proof: vec![] },
        };
        let caller = whitelisted_caller();

        #[extrinsic_call]
        handle(RawOrigin::Signed(caller), vec![Message::Timeout(msg)]);

        assert!(RequestCommitments::<T>::get(commitment).is_none());
    }

    #[benchmark]
    fn on_finalize(x: Linear<1, 100>) {
        for nonce in 0..x {
            let post = Post {
                source: StateMachine::Kusama(2000),
                dest: StateMachine::Kusama(2001),
                nonce: nonce.into(),
                from: vec![0u8; 32],
                to: vec![1u8; 32],
                timeout_timestamp: 100,
                data: vec![2u8; 64],
                gas_limit: 0,
            };

            let request = Request::Post(post);
            let leaf = Leaf::Request(request);

            Pallet::<T>::mmr_push(leaf.clone()).unwrap();
        }

        #[block]
        {
            Pallet::<T>::on_finalize(2u32.into())
        }
    }

    #[benchmark]
    fn dispatch_post_request() {
        let post = DispatchPost {
            dest: StateMachine::Kusama(2000),
            from: vec![0u8; 32],
            to: vec![1u8; 32],
            timeout_timestamp: 100,
            data: vec![2u8; 64],
            gas_limit: 0,
        };

        let dispatcher = Dispatcher::<T>::default();
        #[block]
        {
            dispatcher.dispatch_request(DispatchRequest::Post(post)).unwrap()
        }
    }

    #[benchmark]
    fn dispatch_get_request() {
        let get = DispatchGet {
            dest: StateMachine::Kusama(2000),
            from: vec![0u8; 32],
            keys: vec![vec![1u8; 32]; 32],
            height: 20,
            timeout_timestamp: 100,
            gas_limit: 0,
        };

        let dispatcher = Dispatcher::<T>::default();
        #[block]
        {
            dispatcher.dispatch_request(DispatchRequest::Get(get)).unwrap()
        }
    }

    #[benchmark]
    fn dispatch_response() {
        let post = Post {
            source: StateMachine::Kusama(2000),
            dest: StateMachine::Kusama(2001),
            nonce: 0,
            from: vec![0u8; 32],
            to: vec![1u8; 32],
            timeout_timestamp: 100,
            data: vec![2u8; 64],
            gas_limit: 0,
        };
        let request_commitment = hash_request::<Host<T>>(&Request::Post(post.clone()));
        RequestCommitments::<T>::insert(
            request_commitment,
            LeafIndexQuery { source_chain: post.source, dest_chain: post.dest, nonce: 0 },
        );

        let response = PostResponse { post, response: vec![1u8; 64] };

        let dispatcher = Dispatcher::<T>::default();
        #[block]
        {
            dispatcher.dispatch_response(response).unwrap()
        }
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::mocks::Test);
}
