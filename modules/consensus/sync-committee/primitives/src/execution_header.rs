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
//!
//! Those two fields are optional because the first Gloas states still commit to a block from
//! before the fork. The payload for the first Gloas slot arrives in its own envelope, so the
//! state at that slot, which is also the first finalized checkpoint after the fork, carries the
//! hash of the last Prague block, and that header has neither field.

use alloy_primitives::{keccak256, Address, Bloom, Bytes, B256, B64, U256};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};

/// Recover the block hash from the rlp encoded header. The bytes are hashed exactly as supplied,
/// which is what makes the hash a binding commitment to every field inside them.
pub fn execution_block_hash(rlp: &[u8]) -> [u8; 32] {
	keccak256(rlp).0
}

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
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
	pub difficulty: alloy_primitives::U256,
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
	#[cfg_attr(feature = "std", serde(default))]
	pub block_access_list_hash: Option<B256>,
	#[cfg_attr(
		feature = "std",
		serde(default, deserialize_with = "serde_hex_utils::as_hex_quantity::deserialize_option")
	)]
	pub slot_number: Option<u64>,
}

impl ExecutionHeader {
	/// Decode an rlp encoded execution header. Callers must have already checked the bytes hash to
	/// a block hash they trust, otherwise the fields inside mean nothing.
	pub fn decode(rlp: &[u8]) -> Result<Self, alloy_rlp::Error> {
		<Self as alloy_rlp::Decodable>::decode(&mut &rlp[..])
	}

	/// Rlp encode the header. The encoding must round trip to the block hash the beacon state
	/// committed to, so every field the block's fork defines has to be present.
	pub fn encode(&self) -> alloc::vec::Vec<u8> {
		let mut out = alloc::vec::Vec::new();
		alloy_rlp::Encodable::encode(self, &mut out);
		out
	}
}

#[cfg(all(test, feature = "std"))]
mod tests {
	use super::*;

	/// The last block before the fork on a local reth devnet, which the first Gloas state commits
	/// to, and the first block after it.
	const PRAGUE_BLOCK: &str = r#"{"baseFeePerGas": "0x7", "blobGasUsed": "0x0", "difficulty": "0x0", "excessBlobGas": "0x0", "extraData": "0x726574682f76322e372e302f6c696e7578", "gasLimit": "0xbebc200", "gasUsed": "0x0", "hash": "0x2648510bb7819909b36373a3a2ccda82a51a0c8b154d03ff7710ef8d64b689e6", "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "miner": "0x8943545177806ed17b9f23f0a21ee5948ecaa776", "mixHash": "0x879d6ebf21224bb92a05f0c4d50a2823ae85c1bfda76dd171381184d6f4070c5", "nonce": "0x0000000000000000", "number": "0xbf", "parentBeaconBlockRoot": "0x9bdd621d2614dc45107bc9f80a2221de691a899d7740569523c6e0cc24f4ffd6", "parentHash": "0x74f90ca1c71db6591cc4510eb6e831bdf7fcc7beabcdcbfaa1e7cb0788840153", "receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421", "requestsHash": "0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347", "stateRoot": "0xd430244510b470423f038afbfb318f1e8b458357727dd4959845e6b0182fbae8", "timestamp": "0x6abd0dc7", "transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421", "withdrawalsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"}"#;
	const AMSTERDAM_BLOCK: &str = r#"{"baseFeePerGas": "0x7", "blobGasUsed": "0x0", "blockAccessListHash": "0x1d33a17375e8dd0aef5da65c3f51422043f1c40d52eceaaca238cc92501aa32e", "difficulty": "0x0", "excessBlobGas": "0x0", "extraData": "0x726574682f76322e372e302f6c696e7578", "gasLimit": "0xbebc200", "gasUsed": "0x0", "hash": "0x86e1583ca604ce5de648b44c41fb309b8dfe72ab21b57ad9f12eb0d92ac991f8", "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "miner": "0x8943545177806ed17b9f23f0a21ee5948ecaa776", "mixHash": "0x27184df4d2e2182675a2310a546d32e0d74a7accd4fbeafa84a903cd489dd40c", "nonce": "0x0000000000000000", "number": "0xc0", "parentBeaconBlockRoot": "0x1214de134cf32c261ff3c2b6f96435016e9b98bf7dcd4870ad0009a2acf79974", "parentHash": "0x2648510bb7819909b36373a3a2ccda82a51a0c8b154d03ff7710ef8d64b689e6", "receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421", "requestsHash": "0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347", "slotNumber": "0xc0", "stateRoot": "0x2338ab0cb5b76db31251fc2352e9142fd8b2bf19de6ac4a3c201b84d6ba737d3", "timestamp": "0x6abd0dcb", "transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421", "withdrawalsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"}"#;

	fn header_and_hash(block: &str) -> (ExecutionHeader, B256) {
		let header: ExecutionHeader = serde_json::from_str(block).unwrap();
		let json: serde_json::Value = serde_json::from_str(block).unwrap();
		(header, json["hash"].as_str().unwrap().parse().unwrap())
	}

	#[test]
	fn headers_on_both_sides_of_the_fork_hash_to_their_block_hash() {
		for (block, amsterdam) in [(PRAGUE_BLOCK, false), (AMSTERDAM_BLOCK, true)] {
			let (header, hash) = header_and_hash(block);
			assert_eq!(header.block_access_list_hash.is_some(), amsterdam);
			assert_eq!(header.slot_number.is_some(), amsterdam);

			let encoded = header.encode();
			assert_eq!(execution_block_hash(&encoded), hash.0);
			assert_eq!(ExecutionHeader::decode(&encoded).unwrap(), header);
		}
	}
}
