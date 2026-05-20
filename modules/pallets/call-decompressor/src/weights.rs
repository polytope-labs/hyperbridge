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

//! Weight definitions for `pallet-call-decompressor`.
//!
//! The values in `SubstrateWeight` are a placeholder for environments that
//! have not yet generated a measured weight file. The runtime should use the
//! benchmarked weights produced by `frame-omni-bencher` when available.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use polkadot_sdk::*;
use frame_support::weights::Weight;
use core::marker::PhantomData;

pub trait WeightInfo {
	fn decompress_call() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn decompress_call() -> Weight {
		Weight::from_parts(500_000_000, 0)
	}
}

impl WeightInfo for () {
	fn decompress_call() -> Weight {
		Weight::from_parts(500_000_000, 0)
	}
}
