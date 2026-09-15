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

//! BLS12-381 signature verification for the MinSig variant used by commonware:
//! signatures live in G1 (48-byte compressed) and public keys in G2 (96-byte
//! compressed). This is the mirror image of the Ethereum (MinPk) orientation
//! implemented by `bls_on_arkworks`, so we reuse its IETF point serialization
//! but hash to G1 and pair accordingly.

use alloc::vec::Vec;
use ark_bls12_381::{Bls12_381, G1Projective, G2Affine};
use ark_ec::{
	hashing::{curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher, HashToCurve},
	pairing::Pairing,
	AffineRepr, CurveGroup,
};
use ark_ff::{field_hashers::DefaultFieldHasher, One};
use sha2::Sha256;

use crate::{
	error::Error,
	primitives::{G1_LEN, G2_LEN},
};

/// Domain separation tag for messages hashed to G1 under the MinSig
/// proof-of-possession ciphersuite. This is what commonware uses for all
/// consensus vote and seed signatures.
pub const DST_G1: &[u8] = b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_POP_";

/// Hashes a payload to a G1 point per RFC 9380 (`XMD:SHA-256_SSWU_RO`).
pub fn hash_to_g1(payload: &[u8]) -> Result<ark_bls12_381::G1Affine, Error> {
	let hasher = MapToCurveBasedHasher::<
		G1Projective,
		DefaultFieldHasher<Sha256, 128>,
		WBMap<ark_bls12_381::g1::Config>,
	>::new(DST_G1)
	.map_err(|_| Error::InvalidPoint)?;
	hasher.hash(payload).map_err(|_| Error::InvalidPoint)
}

/// Decompresses a 96-byte G2 public key (IETF/ZCash format) and checks it is a
/// non-identity element of the prime-order subgroup.
///
/// The identity is rejected explicitly rather than relying on the decoder: the
/// point at infinity would satisfy the pairing equation against an infinity
/// signature, so accepting it as a group key would make every signature verify.
pub fn decode_public_key(bytes: &[u8; G2_LEN]) -> Result<G2Affine, Error> {
	let point =
		bls::signature_to_point(&Vec::from(bytes.as_slice())).map_err(|_| Error::InvalidPoint)?;
	if point.is_zero() || !bls::signature_subgroup_check(point) {
		return Err(Error::InvalidPoint);
	}
	Ok(point)
}

/// Verifies a MinSig BLS signature: `e(signature, G2::generator()) ==
/// e(hash_to_g1(payload), public_key)`.
pub fn verify_signature(
	public_key: &G2Affine,
	signature: &[u8; G1_LEN],
	payload: &[u8],
) -> Result<(), Error> {
	let signature =
		bls::pubkey_to_point(&Vec::from(signature.as_slice())).map_err(|_| Error::InvalidPoint)?;
	if signature.is_zero() || !bls::pubkey_subgroup_check(signature) {
		return Err(Error::InvalidPoint);
	}
	let message = hash_to_g1(payload)?;

	// e(sig, gen) == e(H(m), pk)  <=>  e(sig, -gen) · e(H(m), pk) == 1
	let generator = G2Affine::generator();
	let result = Bls12_381::multi_pairing(
		[signature.into_group(), message.into_group()].iter().map(|p| p.into_affine()),
		[(-generator.into_group()).into_affine(), *public_key],
	);
	if result.0.is_one() {
		Ok(())
	} else {
		Err(Error::SignatureVerificationFailed)
	}
}
