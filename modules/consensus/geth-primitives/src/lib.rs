// Copyright (C) 2022 Polytope Labs.
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

use alloc::vec::Vec;
use alloy_primitives::{Address, FixedBytes, B256};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use ethabi::ethereum_types::{Bloom, H64};
#[cfg(feature = "std")]
use alloy_rpc_types_eth::Block;
use ismp::messaging::Keccak256;
use primitive_types::{H160, H256, U256};

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct Header {
	pub parent_hash: B256,
	pub uncle_hash: B256,
	pub coinbase: Address,
	pub state_root: B256,
	pub transactions_root: B256,
	pub receipts_root: B256,
	pub logs_bloom: FixedBytes<256>,
	pub difficulty: alloy_primitives::U256,
	pub number: alloy_primitives::U256,
	pub gas_limit: u64,
	pub gas_used: u64,
	pub timestamp: u64,
	pub extra_data: alloy_primitives::Bytes,
	pub mix_hash: B256,
	pub nonce: FixedBytes<8>,
	pub base_fee_per_gas: Option<alloy_primitives::U256>,
	pub withdrawals_hash: Option<B256>,
	pub blob_gas_used: Option<u64>,
	pub excess_blob_gas_used: Option<u64>,
	pub parent_beacon_root: Option<B256>,
	pub requests_hash: Option<B256>,
}

#[derive(codec::Encode, codec::Decode, Debug, Clone, scale_info::TypeInfo)]
pub struct CodecHeader {
	pub parent_hash: H256,
	pub uncle_hash: H256,
	pub coinbase: H160,
	pub state_root: H256,
	pub transactions_root: H256,
	pub receipts_root: H256,
	pub logs_bloom: Bloom,
	pub difficulty: U256,
	pub number: U256,
	pub gas_limit: u64,
	pub gas_used: u64,
	pub timestamp: u64,
	pub extra_data: Vec<u8>,
	pub mix_hash: H256,
	pub nonce: H64,
	pub base_fee_per_gas: Option<U256>,
	pub withdrawals_hash: Option<H256>,
	pub blob_gas_used: Option<u64>,
	pub excess_blob_gas_used: Option<u64>,
	pub parent_beacon_root: Option<H256>,
	pub requests_hash: Option<H256>,
}

impl AsRef<CodecHeader> for CodecHeader {
	fn as_ref(&self) -> &CodecHeader {
		&self
	}
}

/// Conversion from alloy Block to CodecHeader
#[cfg(feature = "std")]
impl<T> From<Block<T>> for CodecHeader {
	fn from(block: Block<T>) -> Self {
		let header = block.header;
		CodecHeader {
			parent_hash: H256::from_slice(header.parent_hash.as_slice()),
			uncle_hash: H256::from_slice(header.ommers_hash.as_slice()),
			coinbase: H160::from_slice(header.beneficiary.as_slice()),
			state_root: H256::from_slice(header.state_root.as_slice()),
			transactions_root: H256::from_slice(header.transactions_root.as_slice()),
			receipts_root: H256::from_slice(header.receipts_root.as_slice()),
			logs_bloom: Bloom::from_slice(header.logs_bloom.as_slice()),
			difficulty: alloy_u256_to_primitive(header.difficulty),
			number: header.number.into(),
			gas_limit: header.gas_limit,
			gas_used: header.gas_used,
			timestamp: header.timestamp,
			extra_data: header.extra_data.to_vec(),
			mix_hash: H256::from_slice(header.mix_hash.as_slice()),
			nonce: H64::from_slice(header.nonce.as_slice()),
			base_fee_per_gas: header.base_fee_per_gas.map(|v| U256::from(v)),
			withdrawals_hash: header.withdrawals_root.map(|h| H256::from_slice(h.as_slice())),
			blob_gas_used: header.blob_gas_used,
			excess_blob_gas_used: header.excess_blob_gas,
			parent_beacon_root: header.parent_beacon_block_root.map(|h| H256::from_slice(h.as_slice())),
			requests_hash: header.requests_hash.map(|h| H256::from_slice(h.as_slice())),
		}
	}
}

impl From<&CodecHeader> for Header {
	fn from(value: &CodecHeader) -> Self {
		Header {
			parent_hash: value.parent_hash.0.into(),
			uncle_hash: value.uncle_hash.0.into(),
			coinbase: value.coinbase.0.into(),
			state_root: value.state_root.0.into(),
			transactions_root: value.transactions_root.0.into(),
			receipts_root: value.receipts_root.0.into(),
			logs_bloom: value.logs_bloom.0.into(),
			difficulty: { alloy_primitives::U256::from_be_bytes(value.difficulty.to_big_endian()) },
			number: { alloy_primitives::U256::from_be_bytes(value.number.to_big_endian()) },
			gas_limit: value.gas_limit,
			gas_used: value.gas_used,
			timestamp: value.timestamp,
			extra_data: value.extra_data.clone().into(),
			mix_hash: value.mix_hash.0.into(),
			nonce: value.nonce.0.into(),
			base_fee_per_gas: value
				.base_fee_per_gas
				.map(|val| alloy_primitives::U256::from_be_bytes(val.to_big_endian())),
			withdrawals_hash: value.withdrawals_hash.map(|val| val.0.into()),
			blob_gas_used: value.blob_gas_used,
			excess_blob_gas_used: value.excess_blob_gas_used,
			parent_beacon_root: value.parent_beacon_root.map(|val| val.0.into()),
			requests_hash: value.requests_hash.map(|val| val.0.into()),
		}
	}
}

impl Header {
	pub fn hash<H: Keccak256>(self) -> H256 {
		let encoding = alloy_rlp::encode(self);
		H::keccak256(&encoding)
	}
}

/// Convert alloy U256 to primitive_types U256
#[cfg(feature = "std")]
pub fn alloy_u256_to_primitive(val: alloy_primitives::U256) -> U256 {
	U256::from_little_endian(&val.to_le_bytes::<32>())
}

/// Alias for backwards compatibility
#[cfg(feature = "std")]
pub use alloy_u256_to_primitive as new_u256;

/// Convert primitive_types U256 to alloy U256
#[cfg(feature = "std")]
pub fn primitive_u256_to_alloy(val: U256) -> alloy_primitives::U256 {
	let bytes = val.to_little_endian();
	alloy_primitives::U256::from_le_bytes(bytes)
}

/// Alias for backwards compatibility
#[cfg(feature = "std")]
pub use primitive_u256_to_alloy as old_u256;
