use crate::{
	consensus_types::{
		AttestationData, AttesterSlashing, Deposit, Eth1Data, ExecutionPayload, ProposerSlashing,
		SignedBlsToExecutionChange, SignedVoluntaryExit, SyncAggregate,
	},
	constants::{
		BlsPublicKey, BlsSignature, Bytes32, Epoch, ExecutionAddress, Gwei, ValidatorIndex,
	},
	deneb::KzgCommitment,
};
use ssz_rs::{prelude::*, Bitlist, Bitvector, Deserialize};

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Attestation<const MAX_VALIDATORS_PER_SLOT: usize, const MAX_COMMITTEES_PER_SLOT: usize> {
	pub aggregation_bits: Bitlist<MAX_VALIDATORS_PER_SLOT>,
	pub data: AttestationData,
	pub signature: BlsSignature,
	pub committee_bits: Bitvector<MAX_COMMITTEES_PER_SLOT>,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct DepositRequest {
	pub pub_key: BlsPublicKey,
	pub withdrawal_credentials: Bytes32,
	pub amount: Gwei,
	pub signature: BlsSignature,
	pub index: u64,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawalRequest {
	pub source_address: ExecutionAddress,
	pub validator_pubkey: BlsPublicKey,
	pub amount: Gwei,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ConsolidationRequest {
	pub source_address: ExecutionAddress,
	pub source_pubkey: BlsPublicKey,
	pub target_pubkey: BlsPublicKey,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutionRequests<
	const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize,
	const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize,
	const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize,
> {
	pub deposits: List<DepositRequest, MAX_DEPOSIT_REQUESTS_PER_PAYLOAD>,
	pub withdrawals: List<WithdrawalRequest, MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD>,
	pub consolidations: List<ConsolidationRequest, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD>,
}

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BeaconBlockBody<
	const MAX_PROPOSER_SLASHINGS: usize,
	const MAX_VALIDATORS_PER_SLOT: usize,
	const MAX_COMMITTEES_PER_SLOT: usize,
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
	const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize,
	const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize,
	const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize,
> {
	pub randao_reveal: BlsSignature,
	pub eth1_data: Eth1Data,
	pub graffiti: Bytes32,
	pub proposer_slashings: List<ProposerSlashing, MAX_PROPOSER_SLASHINGS>,
	pub attester_slashings: List<AttesterSlashing<MAX_VALIDATORS_PER_SLOT>, MAX_ATTESTER_SLASHINGS>,
	pub attestations:
		List<Attestation<MAX_VALIDATORS_PER_SLOT, MAX_COMMITTEES_PER_SLOT>, MAX_ATTESTATIONS>,
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

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PendingPartialWithdrawal {
	pub validator_index: ValidatorIndex,
	pub amount: Gwei,
	pub withdrawable_epoch: Epoch,
}

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PendingConsolidation {
	pub source_index: ValidatorIndex,
	pub target_index: ValidatorIndex,
}
