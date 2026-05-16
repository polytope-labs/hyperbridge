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
use alloy_primitives::Address;
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use codec::{Decode, DecodeWithMemTracking, Encode};
pub use crypto_utils::verification::Signature;
use ismp::{host::StateMachine, messaging::Proof};
use polkadot_sdk::*;
use sp_core::H256;

#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct WithdrawalProof {
	/// Request commitments delivered from source to destination
	pub commitments: Vec<H256>,
	/// Request commitments on source chain
	pub source_proof: Proof,
	/// Request receipts on destination chain
	pub dest_proof: Proof,
	/// Beneficiary address and Signature from the account that delivered the message
	///  over the keccak hash of the beneficiary address
	pub beneficiary_details: Option<(Vec<u8>, Signature)>,
}

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct FeeMetadata {
	pub fee: alloy_primitives::U256,
	pub sender: Address,
}

#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct WithdrawalInputData {
	/// Signature data to prove account ownership
	pub signature: Signature,
	/// Chain to withdraw funds from
	pub dest_chain: StateMachine,
	/// The withdrawal beneficiary
	pub beneficiary: Option<Vec<u8>>,
}
