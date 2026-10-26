use alloy_primitives::{Address, Bloom, Bytes, B256};
use serde::{Deserialize, Serialize};
use sp_crypto_hashing::{blake2_256, keccak_256};

pub type H256 = B256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Hasher {
	Keccak,
	Blake2,
}

impl Hasher {
	pub fn hash(&self, data: &[u8]) -> H256 {
		match self {
			Hasher::Keccak => B256::from(keccak_256(data)),
			Hasher::Blake2 => B256::from(blake2_256(data)),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
	ApplyExtrinsic(u32),
	Finalization,
	Initialization,
}

#[derive(Debug, Clone)]
pub struct DecodedEvent {
	pub pallet: String,
	pub variant: String,
	pub phase: Phase,
	pub event_index: u32,
	pub fields: scale_value::Composite<u32>,
}

#[derive(Debug, Clone)]
pub struct EthLog {
	pub address: Address,
	pub topics: Vec<B256>,
	pub data: Bytes,
	pub block_hash: B256,
	pub block_number: u64,
	pub block_timestamp: u64,
	pub transaction_hash: B256,
	pub transaction_index: u64,
	pub log_index: u64,
}

#[derive(Debug, Clone)]
pub struct DecodedBlock {
	pub hash: B256,
	pub parent_hash: B256,
	pub number: u64,
	pub timestamp: u64,
	pub timestamp_ms: u64,
	pub state_root: B256,
	pub extrinsics_root: B256,
	pub spec_version: u32,
	pub extrinsic_hashes: Vec<B256>,
	pub logs: Vec<EthLog>,
	pub logs_bloom: Bloom,
}

pub fn pallet_address(pallet: &str) -> Address {
	let mut preimage = b"substrate-pallet:".to_vec();
	preimage.extend_from_slice(pallet.as_bytes());
	Address::from_slice(&keccak_256(&preimage)[12..])
}

pub fn storage_address() -> Address {
	Address::from_slice(&keccak_256(b"substrate-storage")[12..])
}

#[cfg(test)]
mod tests {
	use super::*;
	use alloy_primitives::{address, b256};

	#[test]
	fn keccak_of_empty_input() {
		assert_eq!(
			Hasher::Keccak.hash(b""),
			b256!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
		);
	}

	#[test]
	fn pinned_pallet_address() {
		assert_eq!(pallet_address("ismp"), address!("a2692e50566b4b43335b59fec7ebc738b47f8b50"));
	}

	#[test]
	fn pinned_storage_address() {
		assert_eq!(storage_address(), address!("401e375f643e635ef3b498cedbc47931d2474364"));
	}
}
