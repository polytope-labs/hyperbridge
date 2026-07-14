//! The execution block header, as it is laid out after Glamsterdam.
//!
//! After ePBS the beacon state no longer carries the execution payload header, only the execution
//! `block_hash`. Since a block hash is defined as `keccak256(rlp(header))`, handing the verifier
//! the header itself is enough to recover the execution state root: hash the bytes, check they
//! match the block hash consensus already vouched for, and read the fields off the header.
//!
//! The prover encodes this type and the verifier decodes it, so the field order below is the
//! single definition of the header layout. Glamsterdam appends `block_access_list_hash`
//! (EIP-7928) and `slot_number` to what Prague had, and a header encoded without them hashes to
//! the wrong block hash.

use alloy_primitives::{keccak256, Address, Bloom, Bytes, B256, B64, U256};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};

/// Recover the block hash from the rlp encoded header. The bytes are hashed exactly as supplied,
/// which is what makes the hash a binding commitment to every field inside them.
pub fn execution_block_hash(rlp: &[u8]) -> [u8; 32] {
	keccak256(rlp).0
}

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
#[cfg_attr(feature = "std", derive(serde::Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ExecutionHeader {
	pub parent_hash: B256,
	#[cfg_attr(feature = "std", serde(rename = "sha3Uncles"))]
	pub ommers_hash: B256,
	#[cfg_attr(feature = "std", serde(rename = "miner"))]
	pub beneficiary: Address,
	pub state_root: B256,
	pub transactions_root: B256,
	pub receipts_root: B256,
	pub logs_bloom: Bloom,
	pub difficulty: U256,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex_quantity"))]
	pub number: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex_quantity"))]
	pub gas_limit: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex_quantity"))]
	pub gas_used: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex_quantity"))]
	pub timestamp: u64,
	pub extra_data: Bytes,
	pub mix_hash: B256,
	pub nonce: B64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex_quantity"))]
	pub base_fee_per_gas: u64,
	pub withdrawals_root: B256,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex_quantity"))]
	pub blob_gas_used: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex_quantity"))]
	pub excess_blob_gas: u64,
	pub parent_beacon_block_root: B256,
	pub requests_hash: B256,
	pub block_access_list_hash: B256,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex_quantity"))]
	pub slot_number: u64,
}

impl ExecutionHeader {
	/// Decode an rlp encoded execution header. Callers must have already checked the bytes hash to
	/// a block hash they trust, otherwise the fields inside mean nothing.
	pub fn decode(rlp: &[u8]) -> Result<Self, alloy_rlp::Error> {
		<Self as alloy_rlp::Decodable>::decode(&mut &rlp[..])
	}

	/// Rlp encode the header. The encoding must round trip to the block hash the beacon state
	/// committed to, so every field the fork defines has to be present.
	pub fn encode(&self) -> alloc::vec::Vec<u8> {
		let mut out = alloc::vec::Vec::new();
		alloy_rlp::Encodable::encode(self, &mut out);
		out
	}
}
