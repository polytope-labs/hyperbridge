// Copyright (c) 2025 Polytope Labs.
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
use polkadot_sdk::*;

use crate::utils::ModuleId;
use alloc::boxed::Box;
use frame_support::weights::Weight;
use ismp::router::{PostRequest, Response, Timeout};

/// Interface for providing the weight information about [`IsmpModule`](ismp::module::IsmpModule)
/// callbacks
pub trait IsmpModuleWeight {
	/// Should return the weight used in processing this request
	fn on_accept(&self, request: &PostRequest) -> Weight;
	/// Should return the weight used in processing this timeout
	fn on_timeout(&self, request: &Timeout) -> Weight;
	/// Should return the weight used in processing this response
	fn on_response(&self, response: &Response) -> Weight;
}

impl IsmpModuleWeight for () {
	fn on_accept(&self, _request: &PostRequest) -> Weight {
		Weight::zero()
	}
	fn on_timeout(&self, _request: &Timeout) -> Weight {
		Weight::zero()
	}
	fn on_response(&self, _response: &Response) -> Weight {
		Weight::zero()
	}
}

/// An interface for querying the [`IsmpModuleWeight`] for a given
/// [`IsmpModule`](ismp::module::IsmpModule)
pub trait WeightProvider {
	/// Returns a reference to the weight provider for a module
	fn module_callback(dest_module: ModuleId) -> Option<Box<dyn IsmpModuleWeight>>;
}

impl WeightProvider for () {
	fn module_callback(_dest_module: ModuleId) -> Option<Box<dyn IsmpModuleWeight>> {
		None
	}
}
