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
use alloc::{vec, vec::Vec};
use ssz_rs::{prelude::*, Deserialize, List, Vector};

#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
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

#[derive(Default, Clone, Debug, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Checkpoint {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub epoch: u64,
	pub root: Root,
}

#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Eth1Data {
	pub deposit_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub deposit_count: u64,
	pub block_hash: Hash32,
}

#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
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

#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ProposerSlashing {
	pub signed_header_1: SignedBeaconBlockHeader,
	pub signed_header_2: SignedBeaconBlockHeader,
}

#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedBeaconBlockHeader {
	pub message: BeaconBlockHeader,
	pub signature: BlsSignature,
}

#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexedAttestation<const MAX_VALIDATORS_PER_COMMITTEE: usize> {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub attesting_indices: List<u64, MAX_VALIDATORS_PER_COMMITTEE>,
	pub data: AttestationData,
	pub signature: BlsSignature,
}

#[derive(Default, Clone, Debug, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
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

#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AttesterSlashing<const MAX_VALIDATORS_PER_COMMITTEE: usize> {
	pub attestation_1: IndexedAttestation<MAX_VALIDATORS_PER_COMMITTEE>,
	pub attestation_2: IndexedAttestation<MAX_VALIDATORS_PER_COMMITTEE>,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Attestation<
	const MAX_VALIDATORS_PER_COMMITTEE: usize,
	const MAX_COMMITTEES_PER_SLOT: usize,
> {
	pub aggregation_bits: Bitlist<MAX_VALIDATORS_PER_COMMITTEE>,
	pub data: AttestationData,
	pub signature: BlsSignature,
	pub committee_bits: Bitvector<MAX_COMMITTEES_PER_SLOT>,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Deposit {
	pub proof: Vector<Hash32, DEPOSIT_PROOF_LENGTH>,
	pub data: DepositData,
}

#[derive(Default, Debug, Clone, SimpleSerialize, codec::Encode, codec::Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct DepositData {
	#[cfg_attr(feature = "std", serde(rename = "pubkey"))]
	pub public_key: BlsPublicKey,
	pub withdrawal_credentials: Hash32,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub amount: u64,
	pub signature: BlsSignature,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct VoluntaryExit {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub epoch: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub validator_index: u64,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedVoluntaryExit {
	pub message: VoluntaryExit,
	pub signature: BlsSignature,
}

#[derive(Default, Debug, Clone, SimpleSerialize, codec::Encode, codec::Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SyncAggregate<const SYNC_COMMITTEE_SIZE: usize> {
	pub sync_committee_bits: Bitvector<SYNC_COMMITTEE_SIZE>,
	pub sync_committee_signature: BlsSignature,
}

#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SyncCommittee<const SYNC_COMMITTEE_SIZE: usize> {
	#[cfg_attr(feature = "std", serde(rename = "pubkeys"))]
	pub public_keys: Vector<BlsPublicKey, SYNC_COMMITTEE_SIZE>,
	#[cfg_attr(feature = "std", serde(rename = "aggregate_pubkey"))]
	pub aggregate_public_key: BlsPublicKey,
}

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
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

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BlsToExecutionChange {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub validator_index: ValidatorIndex,
	#[cfg_attr(feature = "std", serde(rename = "from_bls_pubkey"))]
	pub from_bls_public_key: BlsPublicKey,
	pub to_execution_address: ExecutionAddress,
}

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedBlsToExecutionChange {
	message: BlsToExecutionChange,
	signature: BlsSignature,
}

pub type Transaction<const MAX_BYTES_PER_TRANSACTION: usize> = ByteList<MAX_BYTES_PER_TRANSACTION>;

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutionPayload<
	const BYTES_PER_LOGS_BLOOM: usize,
	const MAX_EXTRA_DATA_BYTES: usize,
	const MAX_BYTES_PER_TRANSACTION: usize,
	const MAX_TRANSACTIONS_PER_PAYLOAD: usize,
	const MAX_WITHDRAWALS_PER_PAYLOAD: usize,
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
	pub base_fee_per_gas: U256,
	pub block_hash: Hash32,
	pub transactions: List<Transaction<MAX_BYTES_PER_TRANSACTION>, MAX_TRANSACTIONS_PER_PAYLOAD>,
	pub withdrawals: List<Withdrawal, MAX_WITHDRAWALS_PER_PAYLOAD>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub blob_gas_used: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub excess_blob_gas: u64,
}

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutionPayloadHeader<
	const BYTES_PER_LOGS_BLOOM: usize,
	const MAX_EXTRA_DATA_BYTES: usize,
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
	pub base_fee_per_gas: U256,
	pub block_hash: Hash32,
	pub transactions_root: Root,
	pub withdrawals_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub blob_gas_used: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub excess_blob_gas: u64,
}

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BeaconBlockBody<
	const MAX_PROPOSER_SLASHINGS: usize,
	const MAX_VALIDATORS_PER_COMMITTEE: usize,
	const MAX_ATTESTER_SLASHINGS: usize,
	const MAX_ATTESTATIONS: usize,
	const MAX_DEPOSITS: usize,
	const MAX_VOLUNTARY_EXITS: usize,
	const SYNC_COMMITTEE_SIZE: usize,
	const BYTES_PER_LOGS_BLOOM: usize,
	const MAX_EXTRA_DATA_BYTES: usize,
	const MAX_BYTES_PER_TRANSACTION: usize,
	const MAX_TRANSACTIONS_PER_PAYLOAD: usize,
	const MAX_WITHDRAWALS_PER_PAYLOAD: usize,
	const MAX_BLS_TO_EXECUTION_CHANGES: usize,
	const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize,
	const MAX_COMMITTEES_PER_SLOT: usize,
	const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize,
	const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize,
	const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize,
> {
	pub randao_reveal: BlsSignature,
	pub eth1_data: Eth1Data,
	pub graffiti: Bytes32,
	pub proposer_slashings: List<ProposerSlashing, MAX_PROPOSER_SLASHINGS>,
	pub attester_slashings:
		List<AttesterSlashing<MAX_VALIDATORS_PER_COMMITTEE>, MAX_ATTESTER_SLASHINGS>,
	pub attestations:
		List<Attestation<MAX_VALIDATORS_PER_COMMITTEE, MAX_COMMITTEES_PER_SLOT>, MAX_ATTESTATIONS>,
	pub deposits: List<Deposit, MAX_DEPOSITS>,
	pub voluntary_exits: List<SignedVoluntaryExit, MAX_VOLUNTARY_EXITS>,
	pub sync_aggregate: SyncAggregate<SYNC_COMMITTEE_SIZE>,
	pub execution_payload: ExecutionPayload<
		BYTES_PER_LOGS_BLOOM,
		MAX_EXTRA_DATA_BYTES,
		MAX_BYTES_PER_TRANSACTION,
		MAX_TRANSACTIONS_PER_PAYLOAD,
		MAX_WITHDRAWALS_PER_PAYLOAD,
	>,
	pub bls_to_execution_changes: List<SignedBlsToExecutionChange, MAX_BLS_TO_EXECUTION_CHANGES>,
	pub blob_kzg_commitments: List<KzgCommitment, MAX_BLOB_COMMITMENTS_PER_BLOCK>,
	pub execution_requests: ExecutionRequests<
		MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
		MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
		MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
	>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, SimpleSerialize, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BeaconBlock<
	const MAX_PROPOSER_SLASHINGS: usize,
	const MAX_VALIDATORS_PER_COMMITTEE: usize,
	const MAX_ATTESTER_SLASHINGS: usize,
	const MAX_ATTESTATIONS: usize,
	const MAX_DEPOSITS: usize,
	const MAX_VOLUNTARY_EXITS: usize,
	const SYNC_COMMITTEE_SIZE: usize,
	const BYTES_PER_LOGS_BLOOM: usize,
	const MAX_EXTRA_DATA_BYTES: usize,
	const MAX_BYTES_PER_TRANSACTION: usize,
	const MAX_TRANSACTIONS_PER_PAYLOAD: usize,
	const MAX_WITHDRAWALS_PER_PAYLOAD: usize,
	const MAX_BLS_TO_EXECUTION_CHANGES: usize,
	const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize,
	const MAX_COMMITTEES_PER_SLOT: usize,
	const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize,
	const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize,
	const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize,
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
}
#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Fork {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex"))]
	pub previous_version: Version,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex"))]
	pub current_version: Version,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub epoch: Epoch,
}

#[derive(Default, Debug, SimpleSerialize, Clone, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ForkData {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex"))]
	pub current_version: Version,
	pub genesis_validators_root: Root,
}

#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct HistoricalSummary {
	pub block_summary_root: Root,
	pub state_summary_root: Root,
}

#[derive(Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BeaconState<
	const SLOTS_PER_HISTORICAL_ROOT: usize,
	const HISTORICAL_ROOTS_LIMIT: usize,
	const ETH1_DATA_VOTES_BOUND: usize,
	const VALIDATOR_REGISTRY_LIMIT: usize,
	const EPOCHS_PER_HISTORICAL_VECTOR: usize,
	const EPOCHS_PER_SLASHINGS_VECTOR: usize,
	const SYNC_COMMITTEE_SIZE: usize,
	const BYTES_PER_LOGS_BLOOM: usize,
	const MAX_EXTRA_DATA_BYTES: usize,
	const PENDING_DEPOSITS_LIMIT: usize,
	const PENDING_CONSOLIDATIONS_LIMIT: usize,
	const PENDING_PARTIAL_WITHDRAWALS_LIMIT: usize,
	const PROPOSER_LOOK_AHEAD_LIMIT: usize

> {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub genesis_time: u64,
	pub genesis_validators_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: Slot,
	pub fork: Fork,
	pub latest_block_header: BeaconBlockHeader,
	pub block_roots: Vector<Root, SLOTS_PER_HISTORICAL_ROOT>,
	pub state_roots: Vector<Root, SLOTS_PER_HISTORICAL_ROOT>,
	pub historical_roots: List<Root, HISTORICAL_ROOTS_LIMIT>,
	pub eth1_data: Eth1Data,
	pub eth1_data_votes: List<Eth1Data, ETH1_DATA_VOTES_BOUND>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub eth1_deposit_index: u64,
	pub validators: List<Validator, VALIDATOR_REGISTRY_LIMIT>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub balances: List<Gwei, VALIDATOR_REGISTRY_LIMIT>,
	pub randao_mixes: Vector<Bytes32, EPOCHS_PER_HISTORICAL_VECTOR>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub slashings: Vector<Gwei, EPOCHS_PER_SLASHINGS_VECTOR>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_u8_str_or_hex"))]
	pub previous_epoch_participation: List<ParticipationFlags, VALIDATOR_REGISTRY_LIMIT>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_u8_str_or_hex"))]
	pub current_epoch_participation: List<ParticipationFlags, VALIDATOR_REGISTRY_LIMIT>,
	pub justification_bits: Bitvector<JUSTIFICATION_BITS_LENGTH>,
	pub previous_justified_checkpoint: Checkpoint,
	pub current_justified_checkpoint: Checkpoint,
	pub finalized_checkpoint: Checkpoint,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
	pub inactivity_scores: List<u64, VALIDATOR_REGISTRY_LIMIT>,
	pub current_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	pub next_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
	pub latest_execution_payload_header:
		ExecutionPayloadHeader<BYTES_PER_LOGS_BLOOM, MAX_EXTRA_DATA_BYTES>,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub next_withdrawal_index: WithdrawalIndex,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub next_withdrawal_validator_index: ValidatorIndex,
	pub historical_summaries: List<HistoricalSummary, HISTORICAL_ROOTS_LIMIT>,
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
	pending_deposits: List<PendingDeposit, PENDING_DEPOSITS_LIMIT>,
	pending_partial_withdrawals: List<PendingPartialWithdrawal, PENDING_PARTIAL_WITHDRAWALS_LIMIT>,
	pending_consolidations: List<PendingConsolidation, PENDING_CONSOLIDATIONS_LIMIT>,
    //  [New in Fulu:EIP7917]
    #[cfg(feature = "fulu")]
    #[cfg_attr(feature = "std", serde(with = "serde_hex_utils::seq_of_str"))]
    proposer_lookahead: Vector<ValidatorIndex, PROPOSER_LOOK_AHEAD_LIMIT>
}
