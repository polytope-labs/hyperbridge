#[cfg(not(feature = "glamsterdam"))]
use crate::deneb::KzgCommitment;
use crate::{
	constants::{
		BlsPublicKey, BlsSignature, Bytes32, Epoch, ExecutionAddress, Gwei, Hash32,
		ParticipationFlags, Root, Slot, ValidatorIndex, Version, WithdrawalIndex,
		DEPOSIT_PROOF_LENGTH, JUSTIFICATION_BITS_LENGTH,
	},
	electra::*,
	ssz::{ByteList, ByteVector},
};
use alloc::{vec, vec::Vec};
#[cfg(feature = "glamsterdam")]
use ssz_types::ProgressiveList;
use ssz_types::{typenum::Unsigned, BitList, BitVector, FixedVector, VariableList};

/// The state lists EIP-7688 made progressive at Gloas.
///
/// Before Gloas these are ordinary bounded lists, merkleized into a tree padded out to the bound.
/// From Gloas they grow with the data instead, so the bound goes away and only the element type
/// carries over. The alias keeps both shapes on one field declaration.
#[cfg(not(feature = "glamsterdam"))]
pub type StateList<T, N> = VariableList<T, N>;
#[cfg(feature = "glamsterdam")]
pub use crate::ssz::StateList;

#[cfg(feature = "glamsterdam")]
use crate::{
	constants::{BUILDER_PENDING_PAYMENTS_LIMIT, PTC_WINDOW_LIMIT},
	deneb::MAX_BLOB_COMMITMENTS_PER_BLOCK,
	gloas::{
		Builder, BuilderIndex, BuilderPendingPayment, BuilderPendingWithdrawal,
		ExecutionPayloadBid, PayloadAttestation, SignedExecutionPayloadBid,
		MAX_PAYLOAD_ATTESTATIONS, PTC_SIZE,
	},
};

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
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

#[derive(Default, Clone, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Checkpoint {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub epoch: u64,
	pub root: Root,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Eth1Data {
	pub deposit_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub deposit_count: u64,
	pub block_hash: Hash32,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
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

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ProposerSlashing {
	pub signed_header_1: SignedBeaconBlockHeader,
	pub signed_header_2: SignedBeaconBlockHeader,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedBeaconBlockHeader {
	pub message: BeaconBlockHeader,
	pub signature: BlsSignature,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct IndexedAttestation<MAX_VALIDATORS_PER_COMMITTEE: Unsigned> {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub attesting_indices: VariableList<u64, MAX_VALIDATORS_PER_COMMITTEE>,
	pub data: AttestationData,
	pub signature: BlsSignature,
}

#[derive(Default, Clone, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, PartialEq, Eq, codec::Encode, codec::Decode)]
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

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct AttesterSlashing<MAX_VALIDATORS_PER_COMMITTEE: Unsigned> {
	pub attestation_1: IndexedAttestation<MAX_VALIDATORS_PER_COMMITTEE>,
	pub attestation_2: IndexedAttestation<MAX_VALIDATORS_PER_COMMITTEE>,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct Attestation<
	MAX_VALIDATORS_PER_COMMITTEE: Unsigned,
	MAX_COMMITTEES_PER_SLOT: Unsigned,
> {
	pub aggregation_bits: BitList<MAX_VALIDATORS_PER_COMMITTEE>,
	pub data: AttestationData,
	pub signature: BlsSignature,
	pub committee_bits: BitVector<MAX_COMMITTEES_PER_SLOT>,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Deposit {
	pub proof: FixedVector<Hash32, DEPOSIT_PROOF_LENGTH>,
	pub data: DepositData,
}

#[derive(Default, Debug, Clone, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, codec::Encode, codec::Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct DepositData {
	#[cfg_attr(feature = "std", serde(rename = "pubkey"))]
	pub public_key: BlsPublicKey,
	pub withdrawal_credentials: Hash32,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub amount: u64,
	pub signature: BlsSignature,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct VoluntaryExit {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub epoch: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub validator_index: u64,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedVoluntaryExit {
	pub message: VoluntaryExit,
	pub signature: BlsSignature,
}

#[derive(Default, Debug, Clone, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, codec::Encode, codec::Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct SyncAggregate<SYNC_COMMITTEE_SIZE: Unsigned> {
	pub sync_committee_bits: BitVector<SYNC_COMMITTEE_SIZE>,
	pub sync_committee_signature: BlsSignature,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct SyncCommittee<SYNC_COMMITTEE_SIZE: Unsigned> {
	#[cfg_attr(feature = "std", serde(rename = "pubkeys"))]
	pub public_keys: FixedVector<BlsPublicKey, SYNC_COMMITTEE_SIZE>,
	#[cfg_attr(feature = "std", serde(rename = "aggregate_pubkey"))]
	pub aggregate_public_key: BlsPublicKey,
}

#[derive(Default, Debug, Clone, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, PartialEq, Eq, codec::Encode, codec::Decode)]
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

#[derive(Default, Debug, Clone, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BlsToExecutionChange {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub validator_index: ValidatorIndex,
	#[cfg_attr(feature = "std", serde(rename = "from_bls_pubkey"))]
	pub from_bls_public_key: BlsPublicKey,
	pub to_execution_address: ExecutionAddress,
}

#[derive(Default, Debug, Clone, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedBlsToExecutionChange {
	message: BlsToExecutionChange,
	signature: BlsSignature,
}

pub type Transaction<MAX_BYTES_PER_TRANSACTION: Unsigned> = ByteList<MAX_BYTES_PER_TRANSACTION>;

#[derive(Default, Debug, Clone, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, PartialEq, Eq, codec::Encode, codec::Decode)]
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
	pub transactions: VariableList<Transaction<MAX_BYTES_PER_TRANSACTION>, MAX_TRANSACTIONS_PER_PAYLOAD>,
	pub withdrawals: VariableList<Withdrawal, MAX_WITHDRAWALS_PER_PAYLOAD>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub blob_gas_used: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub excess_blob_gas: u64,
}

#[derive(Default, Debug, Clone, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct ExecutionPayloadHeader<
	BYTES_PER_LOGS_BLOOM: Unsigned,
	MAX_EXTRA_DATA_BYTES: Unsigned,
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
	pub transactions_root: Root,
	pub withdrawals_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub blob_gas_used: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub excess_blob_gas: u64,
}

#[derive(Default, Debug, Clone, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct BeaconBlockBody<
	MAX_PROPOSER_SLASHINGS: Unsigned,
	MAX_VALIDATORS_PER_COMMITTEE: Unsigned,
	MAX_ATTESTER_SLASHINGS: Unsigned,
	MAX_ATTESTATIONS: Unsigned,
	MAX_DEPOSITS: Unsigned,
	MAX_VOLUNTARY_EXITS: Unsigned,
	SYNC_COMMITTEE_SIZE: Unsigned,
	BYTES_PER_LOGS_BLOOM: Unsigned,
	MAX_EXTRA_DATA_BYTES: Unsigned,
	MAX_BYTES_PER_TRANSACTION: Unsigned,
	MAX_TRANSACTIONS_PER_PAYLOAD: Unsigned,
	MAX_WITHDRAWALS_PER_PAYLOAD: Unsigned,
	MAX_BLS_TO_EXECUTION_CHANGES: Unsigned,
	MAX_BLOB_COMMITMENTS_PER_BLOCK: Unsigned,
	MAX_COMMITTEES_PER_SLOT: Unsigned,
	MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: Unsigned,
	MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: Unsigned,
	MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: Unsigned,
> {
	pub randao_reveal: BlsSignature,
	pub eth1_data: Eth1Data,
	pub graffiti: Bytes32,
	pub proposer_slashings: VariableList<ProposerSlashing, MAX_PROPOSER_SLASHINGS>,
	pub attester_slashings:
		VariableList<AttesterSlashing<MAX_VALIDATORS_PER_COMMITTEE>, MAX_ATTESTER_SLASHINGS>,
	pub attestations:
		VariableList<Attestation<MAX_VALIDATORS_PER_COMMITTEE, MAX_COMMITTEES_PER_SLOT>, MAX_ATTESTATIONS>,
	pub deposits: VariableList<Deposit, MAX_DEPOSITS>,
	pub voluntary_exits: VariableList<SignedVoluntaryExit, MAX_VOLUNTARY_EXITS>,
	pub sync_aggregate: SyncAggregate<SYNC_COMMITTEE_SIZE>,
	#[cfg(not(feature = "glamsterdam"))]
	pub execution_payload: ExecutionPayload<
		BYTES_PER_LOGS_BLOOM,
		MAX_EXTRA_DATA_BYTES,
		MAX_BYTES_PER_TRANSACTION,
		MAX_TRANSACTIONS_PER_PAYLOAD,
		MAX_WITHDRAWALS_PER_PAYLOAD,
	>,
	pub bls_to_execution_changes: VariableList<SignedBlsToExecutionChange, MAX_BLS_TO_EXECUTION_CHANGES>,
	#[cfg(not(feature = "glamsterdam"))]
	pub blob_kzg_commitments: VariableList<KzgCommitment, MAX_BLOB_COMMITMENTS_PER_BLOCK>,
	#[cfg(not(feature = "glamsterdam"))]
	pub execution_requests: ExecutionRequests<
		MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
		MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
		MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
	>,
	// [New in Gloas:EIP7732] the payload is no longer in the body. The builder's bid commits to
	// the execution block hash and the payload itself is revealed separately.
	#[cfg(feature = "glamsterdam")]
	pub signed_execution_payload_bid: SignedExecutionPayloadBid,
	#[cfg(feature = "glamsterdam")]
	pub payload_attestations: VariableList<PayloadAttestation, MAX_PAYLOAD_ATTESTATIONS>,
	#[cfg(feature = "glamsterdam")]
	pub parent_execution_requests: ExecutionRequests<
		MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
		MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
		MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
	>,
	/// Gloas moves the execution payload out of the block body, so the payload bounds no longer
	/// appear in any field. An unused type parameter is an error where an unused const parameter
	/// was not, so they are parked here. Every derive skips this field, so it never reaches the
	/// wire and does not affect the hash tree root.
	#[cfg(feature = "glamsterdam")]
	#[ssz(skip_serializing, skip_deserializing)]
	#[tree_hash(skip_hashing)]
	#[codec(skip)]
	#[cfg_attr(feature = "std", serde(skip))]
	pub phantom: core::marker::PhantomData<(
		BYTES_PER_LOGS_BLOOM,
		MAX_EXTRA_DATA_BYTES,
		MAX_BYTES_PER_TRANSACTION,
		MAX_TRANSACTIONS_PER_PAYLOAD,
		MAX_WITHDRAWALS_PER_PAYLOAD,
		MAX_BLOB_COMMITMENTS_PER_BLOCK,
	)>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
pub struct BeaconBlock<
	MAX_PROPOSER_SLASHINGS: Unsigned,
	MAX_VALIDATORS_PER_COMMITTEE: Unsigned,
	MAX_ATTESTER_SLASHINGS: Unsigned,
	MAX_ATTESTATIONS: Unsigned,
	MAX_DEPOSITS: Unsigned,
	MAX_VOLUNTARY_EXITS: Unsigned,
	SYNC_COMMITTEE_SIZE: Unsigned,
	BYTES_PER_LOGS_BLOOM: Unsigned,
	MAX_EXTRA_DATA_BYTES: Unsigned,
	MAX_BYTES_PER_TRANSACTION: Unsigned,
	MAX_TRANSACTIONS_PER_PAYLOAD: Unsigned,
	MAX_WITHDRAWALS_PER_PAYLOAD: Unsigned,
	MAX_BLS_TO_EXECUTION_CHANGES: Unsigned,
	MAX_BLOB_COMMITMENTS_PER_BLOCK: Unsigned,
	MAX_COMMITTEES_PER_SLOT: Unsigned,
	MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: Unsigned,
	MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: Unsigned,
	MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: Unsigned,
> {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: Slot,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub proposer_index: ValidatorIndex,
	pub parent_root: Root,
	pub state_root: Root,
	pub body: BeaconBlockBody<
		MAX_PROPOSER_SLASHINGS,
		MAX_VALIDATORS_PER_COMMITTEE,
		MAX_ATTESTER_SLASHINGS,
		MAX_ATTESTATIONS,
		MAX_DEPOSITS,
		MAX_VOLUNTARY_EXITS,
		SYNC_COMMITTEE_SIZE,
		BYTES_PER_LOGS_BLOOM,
		MAX_EXTRA_DATA_BYTES,
		MAX_BYTES_PER_TRANSACTION,
		MAX_TRANSACTIONS_PER_PAYLOAD,
		MAX_WITHDRAWALS_PER_PAYLOAD,
		MAX_BLS_TO_EXECUTION_CHANGES,
		MAX_BLOB_COMMITMENTS_PER_BLOCK,
		MAX_COMMITTEES_PER_SLOT,
		MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
		MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
		MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
	>,
	/// Gloas moves the execution payload out of the block body, so the payload bounds no longer
	/// appear in any field. An unused type parameter is an error where an unused const parameter
	/// was not, so they are parked here. Every derive skips this field, so it never reaches the
	/// wire and does not affect the hash tree root.
	#[cfg(feature = "glamsterdam")]
	#[ssz(skip_serializing, skip_deserializing)]
	#[tree_hash(skip_hashing)]
	#[codec(skip)]
	#[cfg_attr(feature = "std", serde(skip))]
	pub phantom: core::marker::PhantomData<(
		BYTES_PER_LOGS_BLOOM,
		MAX_EXTRA_DATA_BYTES,
		MAX_BYTES_PER_TRANSACTION,
		MAX_TRANSACTIONS_PER_PAYLOAD,
		MAX_WITHDRAWALS_PER_PAYLOAD,
		MAX_BLOB_COMMITMENTS_PER_BLOCK,
	)>,
}
#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Fork {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex"))]
	pub previous_version: Version,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex"))]
	pub current_version: Version,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub epoch: Epoch,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ForkData {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex"))]
	pub current_version: Version,
	pub genesis_validators_root: Root,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct HistoricalSummary {
	pub block_summary_root: Root,
	pub state_summary_root: Root,
}

#[derive(Default, Debug, ssz_derive::Encode, ssz_derive::Decode, tree_hash_derive::TreeHash, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(bound = ""))]
// [Modified in Gloas:EIP7688] the state hashes as a progressive container, which is what keeps a
// field's generalized index from moving when a later fork adds or drops one.
#[cfg_attr(
	feature = "glamsterdam",
	tree_hash(struct_behaviour = "progressive_container", active_fields(1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1))
)]
pub struct BeaconState<
	SLOTS_PER_HISTORICAL_ROOT: Unsigned,
	HISTORICAL_ROOTS_LIMIT: Unsigned,
	ETH1_DATA_VOTES_BOUND: Unsigned,
	VALIDATOR_REGISTRY_LIMIT: Unsigned,
	EPOCHS_PER_HISTORICAL_VECTOR: Unsigned,
	EPOCHS_PER_SLASHINGS_VECTOR: Unsigned,
	SYNC_COMMITTEE_SIZE: Unsigned,
	BYTES_PER_LOGS_BLOOM: Unsigned,
	MAX_EXTRA_DATA_BYTES: Unsigned,
	PENDING_DEPOSITS_LIMIT: Unsigned,
	PENDING_CONSOLIDATIONS_LIMIT: Unsigned,
	PENDING_PARTIAL_WITHDRAWALS_LIMIT: Unsigned,
	PROPOSER_LOOK_AHEAD_LIMIT: Unsigned,
> {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub genesis_time: u64,
	pub genesis_validators_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: Slot,
	pub fork: Fork,
	pub latest_block_header: BeaconBlockHeader,
	pub block_roots: FixedVector<Root, SLOTS_PER_HISTORICAL_ROOT>,
	pub state_roots: FixedVector<Root, SLOTS_PER_HISTORICAL_ROOT>,
	pub historical_roots: VariableList<Root, HISTORICAL_ROOTS_LIMIT>,
	pub eth1_data: Eth1Data,
	pub eth1_data_votes: VariableList<Eth1Data, ETH1_DATA_VOTES_BOUND>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub eth1_deposit_index: u64,
	pub validators: StateList<Validator, VALIDATOR_REGISTRY_LIMIT>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub balances: StateList<Gwei, VALIDATOR_REGISTRY_LIMIT>,
	pub randao_mixes: FixedVector<Bytes32, EPOCHS_PER_HISTORICAL_VECTOR>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub slashings: FixedVector<Gwei, EPOCHS_PER_SLASHINGS_VECTOR>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_u8_str_or_hex"))]
	pub previous_epoch_participation: StateList<ParticipationFlags, VALIDATOR_REGISTRY_LIMIT>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_u8_str_or_hex"))]
	pub current_epoch_participation: StateList<ParticipationFlags, VALIDATOR_REGISTRY_LIMIT>,
	pub justification_bits: BitVector<JUSTIFICATION_BITS_LENGTH>,
	pub previous_justified_checkpoint: Checkpoint,
	pub current_justified_checkpoint: Checkpoint,
	pub finalized_checkpoint: Checkpoint,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub inactivity_scores: StateList<u64, VALIDATOR_REGISTRY_LIMIT>,
	pub current_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	pub next_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	#[cfg(not(feature = "glamsterdam"))]
	pub latest_execution_payload_header:
		ExecutionPayloadHeader<BYTES_PER_LOGS_BLOOM, MAX_EXTRA_DATA_BYTES>,
	// [New in Gloas:EIP7732] takes over the slot the payload header used to occupy, so the
	// generalized index of the execution leaf is unchanged.
	#[cfg(feature = "glamsterdam")]
	pub latest_block_hash: Hash32,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub next_withdrawal_index: WithdrawalIndex,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub next_withdrawal_validator_index: ValidatorIndex,
	pub historical_summaries: VariableList<HistoricalSummary, HISTORICAL_ROOTS_LIMIT>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub deposit_requests_start_index: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub deposit_balance_to_consume: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub exit_balance_to_consume: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub earliest_exit_epoch: Epoch,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub consolidation_balance_to_consume: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub earliest_consolidation_epoch: Epoch,
	pending_deposits: StateList<PendingDeposit, PENDING_DEPOSITS_LIMIT>,
	pending_partial_withdrawals:
		StateList<PendingPartialWithdrawal, PENDING_PARTIAL_WITHDRAWALS_LIMIT>,
	pending_consolidations: StateList<PendingConsolidation, PENDING_CONSOLIDATIONS_LIMIT>,
	//  [New in Fulu:EIP7917]
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	proposer_lookahead: FixedVector<ValidatorIndex, PROPOSER_LOOK_AHEAD_LIMIT>,
	// [New in Gloas:EIP7732] the builder registry and payload timeliness bookkeeping. Nothing
	// here is proven, but the fields are part of the container, so they have to be present for
	// the state to hash to the root the sync committee signed over.
	#[cfg(feature = "glamsterdam")]
	builders: ProgressiveList<Builder>,
	#[cfg(feature = "glamsterdam")]
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	next_withdrawal_builder_index: BuilderIndex,
	#[cfg(feature = "glamsterdam")]
	execution_payload_availability: BitVector<SLOTS_PER_HISTORICAL_ROOT>,
	#[cfg(feature = "glamsterdam")]
	builder_pending_payments: FixedVector<BuilderPendingPayment, BUILDER_PENDING_PAYMENTS_LIMIT>,
	#[cfg(feature = "glamsterdam")]
	builder_pending_withdrawals: ProgressiveList<BuilderPendingWithdrawal>,
	#[cfg(feature = "glamsterdam")]
	latest_execution_payload_bid: ExecutionPayloadBid,
	#[cfg(feature = "glamsterdam")]
	payload_expected_withdrawals: ProgressiveList<Withdrawal>,
	#[cfg(feature = "glamsterdam")]
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_seq_of_str"))]
	ptc_window: FixedVector<FixedVector<ValidatorIndex, PTC_SIZE>, PTC_WINDOW_LIMIT>,
	/// Gloas replaces the execution payload header with a block hash, so the payload bounds no
	/// longer appear in any field. Skipped by every derive, so it does not reach the wire and the
	/// progressive `active_fields` count is unaffected.
	#[cfg(feature = "glamsterdam")]
	#[ssz(skip_serializing, skip_deserializing)]
	#[tree_hash(skip_hashing)]
	#[codec(skip)]
	#[cfg_attr(feature = "std", serde(skip))]
	pub phantom: core::marker::PhantomData<(BYTES_PER_LOGS_BLOOM, MAX_EXTRA_DATA_BYTES)>,
}
