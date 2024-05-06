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

//! ISMP Consensus Client for the Beacon Chain's Sync Committee Consensus Protocol.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

pub mod prelude {
	pub use alloc::{boxed::Box, vec, vec::Vec};
}

pub mod beacon_client;
pub mod pallet;
pub mod types;

pub use beacon_client::*;

pub use sync_committee_primitives::constants;
