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

use alloc::vec::Vec;
use anyhow::anyhow;
use codec::{Decode, DecodeWithMemTracking, Encode};

#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub enum Signature {
	/// An Evm Address and signature
	Evm { address: Vec<u8>, signature: Vec<u8> },
	/// An Sr25519 public key and signature
	Sr25519 { public_key: Vec<u8>, signature: Vec<u8> },
	/// An Ed25519 public key and signature
	Ed25519 { public_key: Vec<u8>, signature: Vec<u8> },
}

impl Signature {
	/// verify the signature with the public key in the enum or optionally provide a public key
	/// to be used to verify the signature
	pub fn verify(
		&self,
		msg: &[u8; 32],
		public_key_op: Option<Vec<u8>>,
	) -> Result<Vec<u8>, anyhow::Error> {
		match self {
			Signature::Evm { signature, .. } => {
				if signature.len() != 65 {
					Err(anyhow!("Invalid Signature"))?
				}

				let mut sig = [0u8; 65];
				sig.copy_from_slice(&signature);
				let pub_key = sp_io::crypto::secp256k1_ecdsa_recover(&sig, msg)
					.map_err(|_| anyhow!("Signature Verification failed"))?;
				let signer = sp_io::hashing::keccak_256(&pub_key[..])[12..].to_vec();
				Ok(signer)
			},
			Signature::Sr25519 { signature, public_key } => {
				Self::verify_sr25519(signature, public_key, msg, &public_key_op)?;
				Ok(public_key_op.unwrap_or(public_key.clone()))
			},
			Signature::Ed25519 { signature, public_key, .. } => {
				let signature =
					signature.as_slice().try_into().map_err(|_| anyhow!("Invalid Signature"))?;
				let pub_key = public_key_op
					.clone()
					.unwrap_or(public_key.clone())
					.as_slice()
					.try_into()
					.map_err(|_| anyhow!("Invalid Public Key"))?;
				if !sp_io::crypto::ed25519_verify(&signature, msg, &pub_key) {
					Err(anyhow!("Signature Verification failed"))?
				}
				Ok(public_key_op.unwrap_or(public_key.clone()))
			},
		}
	}

	fn verify_sr25519(
		signature: &Vec<u8>,
		public_key: &Vec<u8>,
		msg: &[u8; 32],
		public_key_op: &Option<Vec<u8>>,
	) -> Result<[u8; 32], anyhow::Error> {
		let signature =
			signature.as_slice().try_into().map_err(|_| anyhow!("Invalid Signature"))?;

		let pub_key = public_key_op
			.clone()
			.unwrap_or(public_key.clone())
			.as_slice()
			.try_into()
			.map_err(|_| anyhow!("Invalid Public Key"))?;

		if !sp_io::crypto::sr25519_verify(&signature, msg, &pub_key) {
			return Err(anyhow!("Sr25519 signature verification failed"));
		}

		Ok(pub_key.into())
	}

	pub fn verify_and_get_sr25519_pubkey(
		&self,
		msg: &[u8; 32],
		public_key_op: Option<Vec<u8>>,
	) -> Result<[u8; 32], anyhow::Error> {
		match self {
			Signature::Sr25519 { public_key, signature } =>
				Self::verify_sr25519(signature, public_key, msg, &public_key_op),
			_ => Err(anyhow!("Signature is not of type Sr25519")),
		}
	}

	pub fn signer(&self) -> Vec<u8> {
		match self {
			Signature::Evm { address, .. } => address.clone(),
			Signature::Sr25519 { public_key, .. } => public_key.clone(),
			Signature::Ed25519 { public_key, .. } => public_key.clone(),
		}
	}
}
