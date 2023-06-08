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
/// Add the [`BenchmarkClient`] as one of the consensus clients available to pallet-ismp in the
/// runtime configuration.
/// In your module router configuration add the [`BenchmarkIsmpModule`] as one of the ismp modules
/// using the pallet id defined here as it's module id.
#[benchmarks(
where
<T as frame_system::Config>::Hash: From<H256>,
T: pallet_timestamp::Config,
<T as pallet_timestamp::Config>::Moment: From<u64>
)]
pub mod benchmarks {
    use super::*;
    use crate::{dispatcher::Receipt, primitives::ModuleId};
    use alloc::collections::BTreeMap;
    use frame_support::{traits::Hooks, PalletId};
    use frame_system::EventRecord;
    use ismp_rs::{
        consensus::{
            ConsensusClient, IntermediateState, StateCommitment, StateMachineClient,
            StateMachineHeight,
        },
        error::Error as IsmpError,
        messaging::{
            Message, Proof, RequestMessage, ResponseMessage, StateCommitmentHeight, TimeoutMessage,
        },
        module::IsmpModule,
        router::{Post, PostResponse, RequestResponse},
        util::{hash_request, hash_response},
    };
    use sp_std::prelude::Vec;

    /// Verify the the last event emitted
    fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
        let events = frame_system::Pallet::<T>::events();
        let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
        let EventRecord { event, .. } = &events[events.len() - 1];
        assert_eq!(event, &system_event);
    }

    /// A mock consensus client for benchmarking
    #[derive(Default)]
    pub struct BenchmarkClient;

    /// Consensus client id for benchmarking consensus client
    pub const BENCHMARK_CONSENSUS_CLIENT_ID: [u8; 4] = [1u8; 4];

    impl ConsensusClient for BenchmarkClient {
        fn verify_consensus(
            &self,
            _host: &dyn IsmpHost,
            _trusted_consensus_state: Vec<u8>,
            _proof: Vec<u8>,
        ) -> Result<(Vec<u8>, BTreeMap<StateMachine, StateCommitmentHeight>), IsmpError> {
            Ok(Default::default())
        }

        fn verify_fraud_proof(
            &self,
            _host: &dyn IsmpHost,
            _trusted_consensus_state: Vec<u8>,
            _proof_1: Vec<u8>,
            _proof_2: Vec<u8>,
        ) -> Result<(), IsmpError> {
            Ok(())
        }

        fn unbonding_period(&self) -> Duration {
            Duration::from_secs(60 * 60 * 60)
        }

        fn state_machine(
            &self,
            _id: StateMachine,
        ) -> Result<Box<dyn StateMachineClient>, IsmpError> {
            Ok(Box::new(BenchmarkStateMachine))
        }
    }

    /// Mock State Machine
    pub struct BenchmarkStateMachine;

    impl StateMachineClient for BenchmarkStateMachine {
        fn verify_membership(
            &self,
            _host: &dyn IsmpHost,
            _item: RequestResponse,
            _root: StateCommitment,
            _proof: &Proof,
        ) -> Result<(), IsmpError> {
            Ok(())
        }

        fn state_trie_key(&self, _request: Vec<Request>) -> Vec<Vec<u8>> {
            Default::default()
        }

        fn verify_state_proof(
            &self,
            _host: &dyn IsmpHost,
            _keys: Vec<Vec<u8>>,
            _root: StateCommitment,
            _proof: &Proof,
        ) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, IsmpError> {
            Ok(Default::default())
        }
    }

    /// This module should be added to the module router in runtime for benchmarks to pass
    pub struct BenchmarkIsmpModule;
    /// module id for the mock benchmarking module
    pub const MODULE_ID: ModuleId = ModuleId::Pallet(PalletId(*b"benchmak"));
    impl IsmpModule for BenchmarkIsmpModule {
        fn on_accept(&self, _request: Post) -> Result<(), ismp_rs::error::Error> {
            Ok(())
        }

        fn on_response(&self, _response: Response) -> Result<(), ismp_rs::error::Error> {
            Ok(())
        }

        fn on_timeout(&self, _request: Request) -> Result<(), ismp_rs::error::Error> {
            Ok(())
        }
    }

    /// Sets the current timestamp
    fn set_timestamp<T: pallet_timestamp::Config>()
    where
        <T as pallet_timestamp::Config>::Moment: From<u64>,
    {
        pallet_timestamp::Pallet::<T>::set_timestamp(1000_000_000u64.into());
    }

    #[benchmark]
    fn create_consensus_client() {
        set_timestamp::<T>();

        let message = CreateConsensusClient {
            consensus_state: Default::default(),
            consensus_client_id: BENCHMARK_CONSENSUS_CLIENT_ID,
            state_machine_commitments: vec![(
                StateMachineId {
                    state_id: StateMachine::Ethereum,
                    consensus_client: BENCHMARK_CONSENSUS_CLIENT_ID,
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
            Event::ConsensusClientCreated { consensus_client_id: BENCHMARK_CONSENSUS_CLIENT_ID }
                .into(),
        );
    }

    fn setup_mock_client<H: IsmpHost>(host: &H) -> IntermediateState {
        let intermediate_state = IntermediateState {
            height: StateMachineHeight {
                id: StateMachineId {
                    state_id: StateMachine::Ethereum,
                    consensus_client: BENCHMARK_CONSENSUS_CLIENT_ID,
                },
                height: 1,
            },
            commitment: StateCommitment {
                timestamp: 1000,
                overlay_root: None,
                state_root: Default::default(),
            },
        };

        host.store_consensus_state(BENCHMARK_CONSENSUS_CLIENT_ID, vec![]).unwrap();
        host.store_consensus_update_time(BENCHMARK_CONSENSUS_CLIENT_ID, Duration::from_secs(1000))
            .unwrap();
        host.store_state_machine_commitment(
            intermediate_state.height,
            intermediate_state.commitment,
        )
        .unwrap();

        intermediate_state
    }

    // The Benchmark consensus client should be added to the runtime for these benchmarks to work
    #[benchmark]
    fn handle_request_message() {
        set_timestamp::<T>();
        let host = Host::<T>::default();
        let intermediate_state = setup_mock_client(&host);
        let post = Post {
            source_chain: StateMachine::Ethereum,
            dest_chain: <T as Config>::StateMachine::get(),
            nonce: 0,
            from: MODULE_ID.encode(),
            to: MODULE_ID.encode(),
            timeout_timestamp: 5000,
            data: "handle_request_message".as_bytes().to_vec(),
        };

        let msg = RequestMessage {
            requests: vec![post.clone()],
            proof: Proof { height: intermediate_state.height, proof: vec![] },
        };
        let caller = whitelisted_caller();

        #[extrinsic_call]
        handle(RawOrigin::Signed(caller), vec![Message::Request(msg)]);

        let commitment = hash_request::<Host<T>>(&Request::Post(post));
        assert!(IncomingRequestAcks::<T>::get(commitment.0.to_vec()).is_some());
    }

    #[benchmark]
    fn handle_response_message() {
        set_timestamp::<T>();
        let host = Host::<T>::default();
        let intermediate_state = setup_mock_client(&host);
        let post = Post {
            source_chain: <T as Config>::StateMachine::get(),
            dest_chain: StateMachine::Ethereum,
            nonce: 0,
            from: MODULE_ID.encode(),
            to: MODULE_ID.encode(),
            timeout_timestamp: 5000,
            data: "handle_response_message".as_bytes().to_vec(),
        };
        let request = Request::Post(post.clone());

        let commitment = hash_request::<Host<T>>(&request);
        OutgoingRequestAcks::<T>::insert(commitment.0.to_vec(), Receipt::Ok);

        let response = Response::Post(PostResponse { post, response: vec![] });
        let response_commitment = hash_response::<Host<T>>(&response);
        let msg = ResponseMessage::Post {
            responses: vec![response],
            proof: Proof { height: intermediate_state.height, proof: vec![] },
        };

        let caller = whitelisted_caller();

        #[extrinsic_call]
        handle(RawOrigin::Signed(caller), vec![Message::Response(msg)]);

        assert!(IncomingResponseAcks::<T>::get(response_commitment.0.to_vec()).is_some());
    }

    #[benchmark]
    fn handle_timeout_message() {
        set_timestamp::<T>();
        let host = Host::<T>::default();
        let intermediate_state = setup_mock_client(&host);
        let post = Post {
            source_chain: <T as Config>::StateMachine::get(),
            dest_chain: StateMachine::Ethereum,
            nonce: 0,
            from: MODULE_ID.encode(),
            to: MODULE_ID.encode(),
            timeout_timestamp: 500,
            data: "handle_timeout_message".as_bytes().to_vec(),
        };
        let request = Request::Post(post.clone());

        let commitment = hash_request::<Host<T>>(&request);
        OutgoingRequestAcks::<T>::insert(commitment.0.to_vec(), Receipt::Ok);

        let msg = TimeoutMessage::Post {
            requests: vec![request],
            timeout_proof: Proof { height: intermediate_state.height, proof: vec![] },
        };
        let caller = whitelisted_caller();

        #[extrinsic_call]
        handle(RawOrigin::Signed(caller), vec![Message::Timeout(msg)]);

        assert!(OutgoingRequestAcks::<T>::get(commitment.0.to_vec()).is_none());
    }

    #[benchmark]
    fn on_finalize(x: Linear<1, 100>) {
        for nonce in 0..x {
            let post = ismp_rs::router::Post {
                source_chain: StateMachine::Kusama(2000),
                dest_chain: StateMachine::Kusama(2001),
                nonce: nonce.into(),
                from: vec![0u8; 32],
                to: vec![1u8; 32],
                timeout_timestamp: 100,
                data: vec![2u8; 64],
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

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::mock::Test);
}
