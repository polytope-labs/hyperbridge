// Copyright (c) 2024 Polytope Labs.
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

//! Utilities for providing the static weights for module callbacks

use crate::Config;
use frame_support::weights::{constants::RocksDbWeight, Weight};
use ismp::{
	messaging::{Message, TimeoutMessage},
	router::{GetResponse, PostRequest, RequestResponse, Response, Timeout},
};

/// Interface for providing the weight information about [`IsmpModule`](ismp::module::IsmpModule)
/// callbacks
pub trait IsmpModuleWeight {
	/// Should return the weight used in processing this request
	fn on_accept(request: &PostRequest) -> Weight;
	/// Should return the weight used in processing this timeout
	fn on_timeout(request: &Timeout) -> Weight;
	/// Should return the weight used in processing this response
	fn on_response(response: &Response) -> Weight;
}

/// Just by estimation, require benchmark to generate weight for production in runtimes
impl IsmpModuleWeight for () {
	fn on_accept(_request: &PostRequest) -> Weight {
		Weight::from_parts(63_891_000, 0)
			.saturating_add(Weight::from_parts(0, 52674))
			.saturating_add(RocksDbWeight::get().reads(6))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	fn on_timeout(_request: &Timeout) -> Weight {
		Weight::from_parts(63_891_000, 0)
			.saturating_add(Weight::from_parts(0, 52674))
			.saturating_add(RocksDbWeight::get().reads(6))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	fn on_response(_response: &Response) -> Weight {
		Weight::from_parts(63_891_000, 0)
			.saturating_add(Weight::from_parts(0, 52674))
			.saturating_add(RocksDbWeight::get().reads(6))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
}

/// Returns the weight that would be consumed when executing a batch of messages
pub(crate) fn get_weight<T: Config>(messages: &[Message]) -> Weight {
	messages.into_iter().fold(Weight::zero(), |acc, msg| match msg {
		Message::Request(msg) => {
			let cb_weight = msg
				.requests
				.iter()
				.fold(Weight::zero(), |acc, req| acc + T::WeightProvider::on_accept(&req));
			acc + cb_weight
		},
		Message::Response(msg) => match &msg.datagram {
			RequestResponse::Response(responses) => {
				let cb_weight = responses
					.iter()
					.fold(Weight::zero(), |acc, res| acc + T::WeightProvider::on_response(&res));

				acc + cb_weight
			},
			RequestResponse::Request(requests) => {
				let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
					acc + T::WeightProvider::on_response(&Response::Get(GetResponse {
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
					acc + T::WeightProvider::on_timeout(&Timeout::Request(req.clone()))
				});

				acc + cb_weight
			},
			TimeoutMessage::PostResponse { responses, .. } => {
				let cb_weight = responses.iter().fold(Weight::zero(), |acc, res| {
					acc + T::WeightProvider::on_timeout(&Timeout::Response(res.clone()))
				});

				acc + cb_weight
			},
			TimeoutMessage::Get { requests } => {
				let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
					acc + T::WeightProvider::on_timeout(&Timeout::Request(req.clone()))
				});
				acc + cb_weight
			},
		},
		Message::Consensus(_) | Message::FraudProof(_) => acc,
	})
}
