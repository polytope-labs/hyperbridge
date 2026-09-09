use crate::{
	constants::{
		BlsPublicKey, BlsSignature, Bytes32, Epoch, ExecutionAddress, Gwei, Hash32,
		ParticipationFlags, Root, Slot, ValidatorIndex, Version, WithdrawalIndex,
		DEPOSIT_PROOF_LENGTH, JUSTIFICATION_BITS_LENGTH,
	},
	deneb::KzgCommitment,
	electra::*,
	ssz::{ByteList, ByteVector},
};
use ssz_types::{typenum::Unsigned, BitList, BitVector, FixedVector, VariableList};

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
pub struct ProposerSlashing {
	pub signed_header_1: SignedBeaconBlockHeader,
	pub signed_header_2: SignedBeaconBlockHeader,
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
pub struct SignedBeaconBlockHeader {
	pub message: BeaconBlockHeader,
	pub signature: BlsSignature,
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
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct IndexedAttestation<MAX_VALIDATORS_PER_COMMITTEE: Unsigned> {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub attesting_indices: VariableList<u64, MAX_VALIDATORS_PER_COMMITTEE>,
	pub data: AttestationData,
	pub signature: BlsSignature,
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
pub struct AttestationData {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub index: u64,
	pub beacon_block_root: Root,
	pub source: Checkpoint,
	pub target: Checkpoint,
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
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct AttesterSlashing<MAX_VALIDATORS_PER_COMMITTEE: Unsigned> {
	pub attestation_1: IndexedAttestation<MAX_VALIDATORS_PER_COMMITTEE>,
	pub attestation_2: IndexedAttestation<MAX_VALIDATORS_PER_COMMITTEE>,
}

#[derive(
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	codec::Encode,
	codec::Decode,
	Clone,
	PartialEq,
	Eq,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct Attestation<MAX_VALIDATORS_PER_COMMITTEE: Unsigned, MAX_COMMITTEES_PER_SLOT: Unsigned> {
	pub aggregation_bits: BitList<MAX_VALIDATORS_PER_COMMITTEE>,
	pub data: AttestationData,
	pub signature: BlsSignature,
	pub committee_bits: BitVector<MAX_COMMITTEES_PER_SLOT>,
}

#[derive(
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	codec::Encode,
	codec::Decode,
	Clone,
	PartialEq,
	Eq,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Deposit {
	pub proof: FixedVector<Hash32, DEPOSIT_PROOF_LENGTH>,
	pub data: DepositData,
}

#[derive(
	Default,
	Debug,
	Clone,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	codec::Encode,
	codec::Decode,
	PartialEq,
	Eq,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct DepositData {
	#[cfg_attr(feature = "std", serde(rename = "pubkey"))]
	pub public_key: BlsPublicKey,
	pub withdrawal_credentials: Hash32,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub amount: u64,
	pub signature: BlsSignature,
}

#[derive(
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	codec::Encode,
	codec::Decode,
	Clone,
	PartialEq,
	Eq,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct VoluntaryExit {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub epoch: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub validator_index: u64,
}

#[derive(
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	codec::Encode,
	codec::Decode,
	Clone,
	PartialEq,
	Eq,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedVoluntaryExit {
	pub message: VoluntaryExit,
	pub signature: BlsSignature,
}

#[derive(
	Default,
	Debug,
	Clone,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	codec::Encode,
	codec::Decode,
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
	codec::Encode,
	codec::Decode,
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
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BlsToExecutionChange {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub validator_index: ValidatorIndex,
	#[cfg_attr(feature = "std", serde(rename = "from_bls_pubkey"))]
	pub from_bls_public_key: BlsPublicKey,
	pub to_execution_address: ExecutionAddress,
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
pub struct SignedBlsToExecutionChange {
	message: BlsToExecutionChange,
	signature: BlsSignature,
}

pub type Transaction<MAX_BYTES_PER_TRANSACTION: Unsigned> = ByteList<MAX_BYTES_PER_TRANSACTION>;

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
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct ExecutionPayload<
	BYTES_PER_LOGS_BLOOM: Unsigned,
	MAX_EXTRA_DATA_BYTES: Unsigned,
	MAX_BYTES_PER_TRANSACTION: Unsigned,
	MAX_TRANSACTIONS_PER_PAYLOAD: Unsigned,
	MAX_WITHDRAWALS_PER_PAYLOAD: Unsigned,
> {
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
	pub base_fee_per_gas: crate::ssz::U256,
	pub block_hash: Hash32,
	pub transactions:
		VariableList<Transaction<MAX_BYTES_PER_TRANSACTION>, MAX_TRANSACTIONS_PER_PAYLOAD>,
	pub withdrawals: VariableList<Withdrawal, MAX_WITHDRAWALS_PER_PAYLOAD>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub blob_gas_used: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub excess_blob_gas: u64,
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
	pub base_fee_per_gas: crate::ssz::U256,
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
	Default,
	Debug,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	Clone,
	codec::Encode,
	codec::Decode,
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
