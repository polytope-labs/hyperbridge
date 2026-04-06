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

//! BLS12-381 cryptographic type definitions and utilities.

use crate::ssz::ByteVector;
use alloc::vec::Vec;
use bls::{errors::BLSError, types::G1ProjectivePoint};

/// Length of a BLS12-381 public key in bytes (compressed G1 point).
pub const BLS_PUBLIC_KEY_BYTES_LEN: usize = 48;

/// Length of a BLS12-381 signature in bytes (compressed G2 point).
pub const BLS_SIGNATURE_BYTES_LEN: usize = 96;

/// A BLS12-381 public key (48 bytes compressed).
pub type BlsPublicKey = ByteVector<BLS_PUBLIC_KEY_BYTES_LEN>;

/// A BLS12-381 signature (96 bytes compressed).
pub type BlsSignature = ByteVector<BLS_SIGNATURE_BYTES_LEN>;

/// Convert a compressed BLS public key to a projective point.
pub fn pubkey_to_projective(compressed_key: &BlsPublicKey) -> Result<G1ProjectivePoint, BLSError> {
	let affine_point = bls::pubkey_to_point(&compressed_key.to_vec())?;
	Ok(affine_point.into())
}

/// Aggregate multiple BLS public keys into a single public key.
pub fn aggregate_public_keys(keys: &[BlsPublicKey]) -> Vec<u8> {
	let aggregate = keys
		.iter()
		.filter_map(|key| pubkey_to_projective(key).ok())
		.fold(G1ProjectivePoint::default(), |acc, next| acc + next);

	bls::point_to_pubkey(aggregate.into())
}
