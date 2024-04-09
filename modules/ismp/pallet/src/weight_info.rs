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

use crate::{primitives::ModuleId, Config};
use alloc::boxed::Box;
use frame_support::weights::Weight;
use ismp::{
    consensus::ConsensusClientId,
    messaging::{ConsensusMessage, FraudProofMessage, Message, TimeoutMessage},
    router::{GetResponse, Post, Request, RequestResponse, Response, Timeout},
};

/// A trait that provides information about how consensus client execute in the runtime
pub trait ConsensusClientWeight {
    /// Returns the weight that would be used in processing this consensus message
    fn verify_consensus(&self, msg: &ConsensusMessage) -> Weight;
    /// Returns the weight that would be used in processing this fraud proof message
    fn verify_fraud_proof(&self, msg: &FraudProofMessage) -> Weight;
}

impl ConsensusClientWeight for () {
    fn verify_consensus(&self, _msg: &ConsensusMessage) -> Weight {
        Weight::zero()
    }

    fn verify_fraud_proof(&self, _msg: &FraudProofMessage) -> Weight {
        Weight::zero()
    }
}

/// A trait that provides weight information about how module callbacks execute
pub trait IsmpModuleWeight {
    /// Returns the weight used in processing this request
    fn on_accept(&self, request: &Post) -> Weight;
    /// Returns the weight used in processing this timeout
    fn on_timeout(&self, request: &Timeout) -> Weight;
    /// Returns the weight used in processing this response
    fn on_response(&self, response: &Response) -> Weight;
}

impl IsmpModuleWeight for () {
    fn on_accept(&self, _request: &Post) -> Weight {
        Weight::zero()
    }

    fn on_timeout(&self, _request: &Timeout) -> Weight {
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
    fn module_callback(dest_module: ModuleId) -> Option<Box<dyn IsmpModuleWeight>>;
}

impl WeightProvider for () {
    fn consensus_client(_id: ConsensusClientId) -> Option<Box<dyn ConsensusClientWeight>> {
        None
    }

    fn module_callback(_dest_module: ModuleId) -> Option<Box<dyn IsmpModuleWeight>> {
        None
    }
}

/// Returns the weight that would be consumed when executing a batch of messages
pub fn get_weight<T: Config>(messages: &[Message]) -> Weight {
    messages.into_iter().fold(Weight::zero(), |acc, msg| match msg {
        Message::Consensus(msg) => {
            let consensus_handler =
                <T as Config>::WeightProvider::consensus_client(msg.consensus_state_id)
                    .unwrap_or(Box::new(()));
            consensus_handler.verify_consensus(&msg)
        },
        Message::Request(msg) => {
            let cb_weight = msg.requests.iter().fold(Weight::zero(), |acc, req| {
                let dest_module = ModuleId::from_bytes(req.to.as_slice()).ok();
                let handle = dest_module
                    .map(|id| <T as Config>::WeightProvider::module_callback(id))
                    .flatten()
                    .unwrap_or(Box::new(()));
                acc + handle.on_accept(&req)
            });
            acc + cb_weight
        },
        Message::Response(msg) => match &msg.datagram {
            RequestResponse::Response(responses) => {
                let cb_weight = responses.iter().fold(Weight::zero(), |acc, res| {
                    let dest_module = match res {
                        Response::Post(ref post) =>
                            ModuleId::from_bytes(post.post.from.as_slice()).ok(),
                        _ => return acc,
                    };

                    let handle = dest_module
                        .map(|id| <T as Config>::WeightProvider::module_callback(id))
                        .flatten()
                        .unwrap_or(Box::new(()));
                    acc + handle.on_response(&res)
                });

                acc + cb_weight
            },
            RequestResponse::Request(requests) => {
                let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
                    let dest_module = match req {
                        Request::Get(ref get) => ModuleId::from_bytes(get.from.as_slice()).ok(),
                        _ => return acc,
                    };
                    let handle = dest_module
                        .map(|id| <T as Config>::WeightProvider::module_callback(id))
                        .flatten()
                        .unwrap_or(Box::new(()));
                    acc + handle.on_response(&Response::Get(GetResponse {
                        get: req.get_request().expect("Infallible"),
                        values: Default::default(),
                    }))
                });

                acc + cb_weight
            },
        },
        Message::Timeout(msg) => match msg {
            TimeoutMessage::Post { requests, .. } => {
                let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
                    let dest_module = match req {
                        Request::Post(ref post) => ModuleId::from_bytes(post.from.as_slice()).ok(),
                        _ => return acc,
                    };
                    let handle = dest_module
                        .map(|id| <T as Config>::WeightProvider::module_callback(id))
                        .flatten()
                        .unwrap_or(Box::new(()));
                    acc + handle.on_timeout(&Timeout::Request(req.clone()))
                });

                acc + cb_weight
            },
            TimeoutMessage::PostResponse { responses, .. } => {
                let cb_weight = responses.iter().fold(Weight::zero(), |acc, res| {
                    let dest_module = ModuleId::from_bytes(&res.post.to).ok();
                    let handle = dest_module
                        .map(|id| <T as Config>::WeightProvider::module_callback(id))
                        .flatten()
                        .unwrap_or(Box::new(()));
                    acc + handle.on_timeout(&Timeout::Response(res.clone()))
                });

                acc + cb_weight
            },
            TimeoutMessage::Get { requests } => {
                let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
                    let dest_module = match req {
                        Request::Get(ref get) => ModuleId::from_bytes(get.from.as_slice()).ok(),
                        _ => return acc,
                    };
                    let handle = dest_module
                        .map(|id| <T as Config>::WeightProvider::module_callback(id))
                        .flatten()
                        .unwrap_or(Box::new(()));
                    acc + handle.on_timeout(&Timeout::Request(req.clone()))
                });
                acc + cb_weight
            },
        },

        Message::FraudProof(msg) => {
            let consensus_handler =
                <T as Config>::WeightProvider::consensus_client(msg.consensus_state_id)
                    .unwrap_or(Box::new(()));
            consensus_handler.verify_fraud_proof(&msg)
        },
    })
}
