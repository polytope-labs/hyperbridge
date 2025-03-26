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
	/// Beneficiary address
	pub beneficiary_address: Vec<u8>,
	/// Signature from the account that delivered the message
	///  over the keccak hash of the beneficiary address
	pub signature: Signature,
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
