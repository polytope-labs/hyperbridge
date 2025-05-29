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

//! Pallet error definitions and conversions
use polkadot_sdk::*;

use alloc::string::ToString;
use codec::{Decode, DecodeWithMemTracking, Encode};
use sp_core::ConstU32;
use sp_runtime::BoundedVec;
use sp_std::prelude::*;

#[derive(
	Clone, Debug, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
#[allow(missing_docs)]
pub struct HandlingError {
	message: BoundedVec<u8, ConstU32<1000>>,
}

impl From<anyhow::Error> for HandlingError {
	fn from(value: anyhow::Error) -> Self {
		let mut message = value.to_string().as_bytes().to_vec();
		message.truncate(1000);
		Self { message: message.try_into().unwrap_or_default() }
	}
}
