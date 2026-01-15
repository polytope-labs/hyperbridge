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

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod bls;
pub mod ssz;
pub mod verification;

pub use bls::{
	aggregate_public_keys, pubkey_to_projective, BlsPublicKey, BlsSignature,
	BLS_PUBLIC_KEY_BYTES_LEN, BLS_SIGNATURE_BYTES_LEN,
};
pub use ssz::ByteVector;
