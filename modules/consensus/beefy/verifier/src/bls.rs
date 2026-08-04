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

//! A `w3f-bls` backed implementation of the aggregate BLS check, matching the scheme substrate's
//! BEEFY signs with.
//!
//! A host wires this up by implementing [`crate::BlsAggregateVerify`] and delegating to
//! [`aggregate_verify`]. Runtimes should prefer an implementation backed by host functions, since
//! a BLS12-381 pairing executed inside wasm is considerably more expensive than the `ecrecover`
//! calls it replaces. The saving of the BLS path is that there is one pairing regardless of how
//! many validators signed, not that each individual operation is cheap.

use beefy_verifier_primitives::{BLS_G1_SIGNATURE_LEN, BLS_G2_PUBLIC_KEY_LEN};
use w3f_bls::{EngineBLS, Message, PublicKey, SerializableToBytes, Signature, TinyBLS381};

/// Verify that `signature` is the sum of the signatures over `message` produced by the holders of
/// `public_keys`, in a single pairing check.
///
/// The message is hashed onto the signature curve exactly as `w3f-bls` does when signing, with an
/// empty context, matching sp-core's `bls381::Pair::sign`.
///
/// Errors when a point fails to decode; returns `Ok(false)` when the points are well formed but
/// the aggregate does not verify.
pub fn aggregate_verify(
	message: &[u8],
	signature: &[u8; BLS_G1_SIGNATURE_LEN],
	public_keys: &[[u8; BLS_G2_PUBLIC_KEY_LEN]],
) -> anyhow::Result<bool> {
	if public_keys.is_empty() {
		return Ok(false);
	}

	let signature = Signature::<TinyBLS381>::from_bytes(signature)
		.map_err(|_| anyhow::anyhow!("invalid G1 signature encoding"))?;

	let mut aggregate: Option<<TinyBLS381 as EngineBLS>::PublicKeyGroup> = None;
	for key in public_keys {
		let public_key = PublicKey::<TinyBLS381>::from_bytes(key)
			.map_err(|_| anyhow::anyhow!("invalid G2 public key encoding"))?;
		aggregate = Some(aggregate.map_or(public_key.0, |sum| sum + public_key.0));
	}

	let aggregate = aggregate.expect("public_keys is non-empty, checked above; qed");

	Ok(signature.verify(&Message::new(b"", message), &PublicKey::<TinyBLS381>(aggregate)))
}
