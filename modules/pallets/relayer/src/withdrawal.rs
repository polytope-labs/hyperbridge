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

use alloc::{vec, vec::Vec};
use alloy_primitives::{Address, B256};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use anyhow::anyhow;
use codec::{Decode, Encode};
use ismp::{host::StateMachine, messaging::Proof};
use polkadot_sdk::*;
use sp_core::{H160, H256, U256};

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub enum Key {
	Request(H256),
	Response { request_commitment: H256, response_commitment: H256 },
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct WithdrawalProof {
	/// Request and response commitments delivered from source to destination
	pub commitments: Vec<Key>,
	/// Request and response commitments on source chain
	pub source_proof: Proof,
	/// Request and response receipts on destination chain
	pub dest_proof: Proof,
	/// Beneficiary address and Signature from the account that delivered the message
	///  over the keccak hash of the beneficiary address
	pub beneficiary_details: Option<(Vec<u8>, Signature)>,
}

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct ResponseReceipt {
	pub response_commitment: B256,
	pub relayer: Address,
}

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct FeeMetadata {
	pub fee: alloy_primitives::U256,
	pub sender: Address,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct WithdrawalInputData {
	/// Signature data to prove account ownership
	pub signature: Signature,
	/// Chain to withdraw funds from
	pub dest_chain: StateMachine,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
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
				let signature =
					signature.as_slice().try_into().map_err(|_| anyhow!("Invalid Signature"))?;
				let pub_key = public_key_op
					.clone()
					.unwrap_or(public_key.clone())
					.as_slice()
					.try_into()
					.map_err(|_| anyhow!("Invalid Public Key"))?;
				if !sp_io::crypto::sr25519_verify(&signature, msg, &pub_key) {
					Err(anyhow!("Signature Verification failed"))?
				}

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
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct WithdrawalParams {
	pub beneficiary_address: Vec<u8>,
	pub amount: U256,
	pub native: bool,
}

impl WithdrawalParams {
	pub fn abi_encode(&self) -> Vec<u8> {
		let mut data = vec![0];
		let tokens = [
			ethabi::Token::Address(H160::from_slice(&self.beneficiary_address)),
			ethabi::Token::Uint(self.amount),
			ethabi::Token::Bool(self.native),
		];
		let params = ethabi::encode(&tokens);
		data.extend_from_slice(&params);
		data
	}
}

#[cfg(test)]
mod test {
	use crate::withdrawal::WithdrawalParams;
	use ethabi::ethereum_types::H160;
	use polkadot_sdk::*;
	use sp_core::U256;
	#[test]
	fn check_decoding() {
		let params = WithdrawalParams {
			beneficiary_address: H160::random().0.to_vec(),
			amount: U256::from(500_00_000_000u128),
			native: false,
		};

		let encoding = params.abi_encode();

		assert_eq!(encoding.len(), 97);
	}
}
