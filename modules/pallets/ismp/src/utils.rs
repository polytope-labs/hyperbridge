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

//! Pallet utilities
use polkadot_sdk::*;

use alloc::collections::BTreeMap;

use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::PalletId;
use ismp::{
	consensus::{ConsensusClient, ConsensusStateId},
	host::StateMachine,
};
use sp_core::{
	crypto::{AccountId32, ByteArray},
	H160, H256,
};
use sp_std::prelude::*;

/// Params to update the unbonding period for a consensus state
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct UpdateConsensusState {
	/// Consensus state identifier
	pub consensus_state_id: ConsensusStateId,
	/// Unbonding duration
	pub unbonding_period: Option<u64>,
	/// Challenge period duration for different state machines
	pub challenge_periods: BTreeMap<StateMachine, u64>,
}

/// Holds a commitment to either a request or response
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub enum MessageCommitment {
	/// A request message
	Request(H256),
	/// A response message
	Response(H256),
}

/// Params to add more funds for request delivery
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct FundMessageParams<Balance> {
	/// Message commitment
	pub commitment: MessageCommitment,
	/// Amount to fund message by
	pub amount: Balance,
}

/// Receipt for a Response
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct ResponseReceipt {
	/// Hash of the response object
	pub response: H256,
	/// Address of the relayer
	pub relayer: Vec<u8>,
}

/// A  convenience trait that returns a list of all configured consensus clients
/// This trait should be implemented in the runtime
pub trait ConsensusClientProvider {
	/// Returns a list of all configured consensus clients
	fn consensus_clients() -> Vec<Box<dyn ConsensusClient>>;
}

fortuples::fortuples! {
	#[tuples::max_size(30)]
	impl ConsensusClientProvider for #Tuple
	where
		#(#Member: ConsensusClient + Default + 'static),*
	{

		fn consensus_clients() -> Vec<Box<dyn ConsensusClient>> {
			vec![
				#( Box::new(#Member::default()) as Box<dyn ConsensusClient> ),*
			]
		}
	}
}

/// Module identification types supported by ismp
#[derive(PartialEq, Eq, scale_info::TypeInfo)]
pub enum ModuleId {
	/// Unique Pallet identification in runtime
	Pallet(PalletId),
	/// Contract account id
	Contract(AccountId32),
	/// Evm contract
	Evm(H160),
}

impl ModuleId {
	/// Convert module id to raw bytes
	pub fn to_bytes(&self) -> Vec<u8> {
		match self {
			ModuleId::Pallet(pallet_id) => pallet_id.0.to_vec(),
			ModuleId::Contract(account_id) => account_id.as_slice().to_vec(),
			ModuleId::Evm(account_id) => account_id.0.to_vec(),
		}
	}

	/// Derive module id from raw bytes
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
		if bytes.len() == 8 {
			let mut inner = [0u8; 8];
			inner.copy_from_slice(bytes);
			Ok(Self::Pallet(PalletId(inner)))
		} else if bytes.len() == 32 {
			Ok(Self::Contract(AccountId32::from_slice(bytes).expect("Infallible")))
		} else if bytes.len() == 20 {
			Ok(Self::Evm(H160::from_slice(bytes)))
		} else {
			Err("Unknown Module ID format")
		}
	}
}

/// The `ConsensusEngineId` of ISMP `ConsensusDigest` in the parachain header.
pub const ISMP_ID: sp_runtime::ConsensusEngineId = *b"ISMP";

/// Consensus log digest for pallet ismp
#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Default)]
pub struct ConsensusDigest {
	/// Mmr root hash
	pub mmr_root: H256,
	/// Child trie root hash
	pub child_trie_root: H256,
}

/// The `ConsensusEngineId` of Ismp `TimestampDigest` in the parachain header.
pub const ISMP_TIMESTAMP_ID: sp_runtime::ConsensusEngineId = *b"ISTM";

/// Timestamp log digest for pallet ismp
#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Default)]
pub struct TimestampDigest {
	/// Timestamp value in seconds
	pub timestamp: u64,
}
