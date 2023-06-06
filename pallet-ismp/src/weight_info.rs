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

//! Users of ismp should benchmark consensus clients and module callbacks
//! This module provides a guide on how to provide static weights for consensus clients and module
//! callbacks

use crate::Config;
use alloc::boxed::Box;
use frame_support::weights::Weight;
use ismp_rs::{
    consensus::{ConsensusClientId, StateMachineId},
    messaging::{
        ConsensusMessage, FraudProofMessage, Message, Proof, ResponseMessage, TimeoutMessage,
    },
    router::{GetResponse, Post, Request, Response},
};

/// A trait that provides information about how consensus client execute in the runtime
pub trait ConsensusClientWeight {
    /// Returns the weight that would be used in processing this consensus message
    fn verify_consensus(&self, msg: &ConsensusMessage) -> Weight;
    /// Returns the weight that would be used in processing this fraud proof message
    fn verify_fraud_proof(&self, msg: &FraudProofMessage) -> Weight;
    /// Returns weight used in verifying this membership proof
    /// `items` is the number of values being verified
    /// The weight should ideally depend on the number of items being verified
    fn verify_membership(
        &self,
        state_machine: StateMachineId,
        items: usize,
        proof: &Proof,
    ) -> Weight;
    /// Returns weight used in verifying this state proof
    /// `items` is the number of keys being verified
    /// The weight should ideally depend on the number of items being verified
    fn verify_state_proof(
        &self,
        state_machine: StateMachineId,
        items: usize,
        proof: &Proof,
    ) -> Weight;
}

impl ConsensusClientWeight for () {
    fn verify_consensus(&self, _msg: &ConsensusMessage) -> Weight {
        Weight::zero()
    }

    fn verify_fraud_proof(&self, _msg: &FraudProofMessage) -> Weight {
        Weight::zero()
    }

    fn verify_membership(
        &self,
        _state_machine: StateMachineId,
        _items: usize,
        _proof: &Proof,
    ) -> Weight {
        Weight::zero()
    }

    fn verify_state_proof(
        &self,
        _state_machine: StateMachineId,
        _items: usize,
        _proof: &Proof,
    ) -> Weight {
        Weight::zero()
    }
}

/// A trait that provides weight information about how module callbacks execute
pub trait IsmpModuleWeight {
    /// Returns the weight used in processing this request
    fn on_accept(&self, request: &Post) -> Weight;
    /// Returns the weight used in processing this timeout
    fn on_timeout(&self, request: &Request) -> Weight;
    /// Returns the weight used in processing this response
    fn on_response(&self, response: &Response) -> Weight;
}

impl IsmpModuleWeight for () {
    fn on_accept(&self, _request: &Post) -> Weight {
        Weight::zero()
    }

    fn on_timeout(&self, _request: &Request) -> Weight {
        Weight::zero()
    }

    fn on_response(&self, _response: &Response) -> Weight {
        Weight::zero()
    }
}

/// Provides references to consensus and module weight providers
pub trait WeightProvider {
    /// Returns a reference to the weight provider for a consensus client
    fn consensus_client(id: ConsensusClientId) -> Option<Box<dyn ConsensusClientWeight>>;

    /// Returns a reference to the weight provider for a module
    fn module_callback(dest_module: &[u8]) -> Option<Box<dyn IsmpModuleWeight>>;
}

impl WeightProvider for () {
    fn consensus_client(_id: ConsensusClientId) -> Option<Box<dyn ConsensusClientWeight>> {
        None
    }

    fn module_callback(_dest_module: &[u8]) -> Option<Box<dyn IsmpModuleWeight>> {
        None
    }
}

/// These functions account for storage reads and writes in the ismp message handlers
/// They do not take into account proof verification, that is delegated to the Consensus client
/// weight provider
pub trait WeightInfo {
    /// Returns the weight used in finalizing the mmr
    fn on_finalize(n: u32) -> Weight;
    /// Returns the weight consumed in creating a consensus client
    fn create_consensus_client() -> Weight;
    /// Returns the weight consumed in handling a request
    fn handle_request_message() -> Weight;
    /// Returns the weight consumed in handling a response
    fn handle_response_message() -> Weight;
    /// Returns the weight consumed in handling a timeout
    fn handle_timeout_message() -> Weight;
}

impl WeightInfo for () {
    fn on_finalize(_n: u32) -> Weight {
        Weight::zero()
    }

    fn create_consensus_client() -> Weight {
        Weight::zero()
    }

    fn handle_request_message() -> Weight {
        Weight::zero()
    }

    fn handle_response_message() -> Weight {
        Weight::zero()
    }

    fn handle_timeout_message() -> Weight {
        Weight::zero()
    }
}

/// Returns the weight that would be consumed when executing a batch of messages
pub fn get_weight<T: Config>(messages: &[Message]) -> Weight {
    messages.into_iter().fold(Weight::zero(), |acc, msg| match msg {
        Message::Consensus(msg) => {
            let consensus_handler =
                <T as Config>::WeightProvider::consensus_client(msg.consensus_client_id)
                    .unwrap_or(Box::new(()));
            consensus_handler.verify_consensus(msg)
        }
        Message::Request(msg) => {
            let state_machine = msg.proof.height.id;
            let cb_weight = msg.requests.iter().fold(Weight::zero(), |acc, req| {
                let dest_module = req.to.as_slice();
                let handle = <T as Config>::WeightProvider::module_callback(dest_module)
                    .unwrap_or(Box::new(()));
                acc + handle.on_accept(&req)
            });

            let consensus_handler = <T as Config>::WeightProvider::consensus_client(
                msg.proof.height.id.consensus_client,
            )
            .unwrap_or(Box::new(()));

            let proof_verification_weight =
                consensus_handler.verify_membership(state_machine, msg.requests.len(), &msg.proof);

            acc + cb_weight +
                proof_verification_weight +
                <T as Config>::WeightInfo::handle_request_message()
        }
        Message::Response(msg) => match msg {
            ResponseMessage::Post { responses, proof } => {
                let state_machine = proof.height.id;
                let cb_weight = responses.iter().fold(Weight::zero(), |acc, res| {
                    let dest_module = match res {
                        Response::Post(ref post) => post.post.from.as_slice(),
                        _ => return acc,
                    };
                    let handle = <T as Config>::WeightProvider::module_callback(dest_module)
                        .unwrap_or(Box::new(()));
                    acc + handle.on_response(&res)
                });

                let consensus_handler = <T as Config>::WeightProvider::consensus_client(
                    proof.height.id.consensus_client,
                )
                .unwrap_or(Box::new(()));

                let proof_verification_weight =
                    consensus_handler.verify_membership(state_machine, responses.len(), &proof);

                acc + cb_weight +
                    proof_verification_weight +
                    <T as Config>::WeightInfo::handle_response_message()
            }
            ResponseMessage::Get { requests, proof } => {
                let state_machine = proof.height.id;
                let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
                    let dest_module = match req {
                        Request::Get(ref get) => get.from.as_slice(),
                        _ => return acc,
                    };
                    let handle = <T as Config>::WeightProvider::module_callback(dest_module)
                        .unwrap_or(Box::new(()));
                    acc + handle.on_response(&Response::Get(GetResponse {
                        get: req.get_request().unwrap(),
                        values: Default::default(),
                    }))
                });

                let consensus_handler = <T as Config>::WeightProvider::consensus_client(
                    proof.height.id.consensus_client,
                )
                .unwrap_or(Box::new(()));

                let proof_verification_weight =
                    consensus_handler.verify_state_proof(state_machine, requests.len(), &proof);

                acc + cb_weight +
                    proof_verification_weight +
                    <T as Config>::WeightInfo::handle_response_message()
            }
        },
        Message::Timeout(msg) => match msg {
            TimeoutMessage::Post { requests, timeout_proof } => {
                let state_machine = timeout_proof.height.id;
                let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
                    let dest_module = match req {
                        Request::Post(ref post) => post.from.as_slice(),
                        _ => return acc,
                    };
                    let handle = <T as Config>::WeightProvider::module_callback(dest_module)
                        .unwrap_or(Box::new(()));
                    acc + handle.on_timeout(&req)
                });

                let consensus_handler = <T as Config>::WeightProvider::consensus_client(
                    timeout_proof.height.id.consensus_client,
                )
                .unwrap_or(Box::new(()));

                let proof_verification_weight = consensus_handler.verify_state_proof(
                    state_machine,
                    requests.len(),
                    &timeout_proof,
                );

                acc + cb_weight +
                    proof_verification_weight +
                    <T as Config>::WeightInfo::handle_response_message()
            }
            TimeoutMessage::Get { requests } => {
                let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
                    let dest_module = match req {
                        Request::Get(ref get) => get.from.as_slice(),
                        _ => return acc,
                    };
                    let handle = <T as Config>::WeightProvider::module_callback(dest_module)
                        .unwrap_or(Box::new(()));
                    acc + handle.on_timeout(&req)
                });
                acc + cb_weight + <T as Config>::WeightInfo::handle_timeout_message()
            }
        },

        Message::FraudProof(msg) => {
            let consensus_handler =
                <T as Config>::WeightProvider::consensus_client(msg.consensus_client_id)
                    .unwrap_or(Box::new(()));
            consensus_handler.verify_fraud_proof(msg)
        }
    })
}
