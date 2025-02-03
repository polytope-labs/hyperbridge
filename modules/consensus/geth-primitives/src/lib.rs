#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::vec::Vec;
use alloy_primitives::{Address, FixedBytes, B256};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};

use ethabi::ethereum_types::{Bloom, H160, H256, H64, U256};
#[cfg(feature = "std")]
use ethers::types::{Block, U64};
use ismp::messaging::Keccak256;

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

#[cfg(feature = "std")]
impl From<Block<H256>> for CodecHeader {
	fn from(block: Block<H256>) -> Self {
		CodecHeader {
			parent_hash: block.parent_hash,
			uncle_hash: block.uncles_hash,
			coinbase: block.author.unwrap_or_default(),
			state_root: block.state_root,
			transactions_root: block.transactions_root,
			receipts_root: block.receipts_root,
			logs_bloom: block.logs_bloom.unwrap_or_default(),
			difficulty: block.difficulty,
			number: block.number.unwrap_or_default().as_u64().into(),
			gas_limit: block.gas_limit.low_u64(),
			gas_used: block.gas_used.low_u64(),
			timestamp: block.timestamp.low_u64(),
			extra_data: block.extra_data.0.into(),
			mix_hash: block.mix_hash.unwrap_or_default(),
			nonce: block.nonce.unwrap_or_default(),
			base_fee_per_gas: block.base_fee_per_gas,
			withdrawals_hash: block.withdrawals_root,
			blob_gas_used: block
				.other
				.get_deserialized::<U64>("blobGasUsed")
				.and_then(|val| val.ok().map(|val| val.as_u64())),
			excess_blob_gas_used: block
				.other
				.get_deserialized::<U64>("excessBlobGas")
				.and_then(|val| val.ok().map(|val| val.as_u64())),
			parent_beacon_root: block
				.other
				.get_deserialized::<H256>("parentBeaconBlockRoot")
				.and_then(|val| val.ok()),
			requests_hash: {
				if let Some(request_root) = block.other.get_deserialized::<H256>("requestsRoot") {
					request_root.ok()
				} else {
					block.other.get_deserialized::<H256>("requestsHash").and_then(|val| val.ok())
				}
			},
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
			difficulty: {
				let mut bytes = [0u8; 32];
				value.difficulty.to_big_endian(&mut bytes);
				alloy_primitives::U256::from_be_bytes(bytes)
			},
			number: {
				let mut bytes = [0u8; 32];
				value.number.to_big_endian(&mut bytes);
				alloy_primitives::U256::from_be_bytes(bytes)
			},
			gas_limit: value.gas_limit,
			gas_used: value.gas_used,
			timestamp: value.timestamp,
			extra_data: value.extra_data.clone().into(),
			mix_hash: value.mix_hash.0.into(),
			nonce: value.nonce.0.into(),
			base_fee_per_gas: value.base_fee_per_gas.map(|val| {
				let mut bytes = [0u8; 32];
				val.to_big_endian(&mut bytes);
				alloy_primitives::U256::from_be_bytes(bytes)
			}),
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
