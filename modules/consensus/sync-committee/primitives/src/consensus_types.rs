use crate::{
	constants::{
		BlsPublicKey, BlsSignature, Bytes32, Epoch, ExecutionAddress, Gwei, Hash32, Root,
		ValidatorIndex, Version, WithdrawalIndex,
	},
	ssz::{ByteList, ByteVector},
};
use ssz_types::{typenum::Unsigned, BitVector, FixedVector};

#[derive(
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	Clone,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BeaconBlockHeader {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub proposer_index: u64,
	pub parent_root: Root,
	pub state_root: Root,
	pub body_root: Root,
}

#[derive(
	Default,
	Clone,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Checkpoint {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub epoch: u64,
	pub root: Root,
}

#[derive(
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	Clone,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Eth1Data {
	pub deposit_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub deposit_count: u64,
	pub block_hash: Hash32,
}

#[derive(
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	Clone,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Validator {
	#[cfg_attr(feature = "std", serde(rename = "pubkey"))]
	pub public_key: BlsPublicKey,
	pub withdrawal_credentials: Bytes32,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub effective_balance: Gwei,
	pub slashed: bool,
	// Status epochs
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub activation_eligibility_epoch: Epoch,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub activation_epoch: Epoch,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub exit_epoch: Epoch,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub withdrawable_epoch: Epoch,
}

#[derive(
	Default,
	Debug,
	Clone,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	PartialEq,
	Eq,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct SyncAggregate<SYNC_COMMITTEE_SIZE: Unsigned> {
	pub sync_committee_bits: BitVector<SYNC_COMMITTEE_SIZE>,
	pub sync_committee_signature: BlsSignature,
}

#[derive(
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	Clone,
	PartialEq,
	Eq,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct SyncCommittee<SYNC_COMMITTEE_SIZE: Unsigned> {
	#[cfg_attr(feature = "std", serde(rename = "pubkeys"))]
	pub public_keys: FixedVector<BlsPublicKey, SYNC_COMMITTEE_SIZE>,
	#[cfg_attr(feature = "std", serde(rename = "aggregate_pubkey"))]
	pub aggregate_public_key: BlsPublicKey,
}

#[derive(
	Default,
	Debug,
	Clone,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Withdrawal {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub index: WithdrawalIndex,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub validator_index: ValidatorIndex,
	pub address: ExecutionAddress,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub amount: Gwei,
}

#[derive(
	Default,
	Debug,
	Clone,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	PartialEq,
	Eq,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct ExecutionPayloadHeader<BYTES_PER_LOGS_BLOOM: Unsigned, MAX_EXTRA_DATA_BYTES: Unsigned> {
	pub parent_hash: Hash32,
	pub fee_recipient: ExecutionAddress,
	pub state_root: Bytes32,
	pub receipts_root: Bytes32,
	pub logs_bloom: ByteVector<BYTES_PER_LOGS_BLOOM>,
	pub prev_randao: Bytes32,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub block_number: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub gas_limit: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub gas_used: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub timestamp: u64,
	pub extra_data: ByteList<MAX_EXTRA_DATA_BYTES>,
	pub base_fee_per_gas: alloy_primitives::U256,
	pub block_hash: Hash32,
	pub transactions_root: Root,
	pub withdrawals_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub blob_gas_used: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub excess_blob_gas: u64,
}

#[derive(
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	Clone,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Fork {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex"))]
	pub previous_version: Version,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex"))]
	pub current_version: Version,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub epoch: Epoch,
}

#[derive(
	Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ForkData {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex"))]
	pub current_version: Version,
	pub genesis_validators_root: Root,
}

#[derive(
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	Clone,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct HistoricalSummary {
	pub block_summary_root: Root,
	pub state_summary_root: Root,
}
