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

//! Weight information for the hyper-fungible-token pallet

use polkadot_sdk::sp_runtime::Weight;

/// Weight functions needed by `pallet-hyper-fungible-token`.
pub trait WeightInfo {
	/// Weight of `send`.
	fn send() -> Weight;
	/// Weight of `register_token`, over the number of chains registered.
	fn register_token(c: u32) -> Weight;
	/// Weight of `update_token`, over the number of chains added (`a`) and removed (`r`).
	fn update_token(a: u32, r: u32) -> Weight;
}

/// No-op [`WeightInfo`] for tests and genesis bootstrap.
impl WeightInfo for () {
	fn send() -> Weight {
		Weight::zero()
	}
	fn register_token(_c: u32) -> Weight {
		Weight::zero()
	}
	fn update_token(_a: u32, _r: u32) -> Weight {
		Weight::zero()
	}
}
