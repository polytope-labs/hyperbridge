use sync_committee_primitives::{
	consensus_types::BeaconBlock,
	constants::{
		BYTES_PER_LOGS_BLOOM, MAX_ATTESTATIONS, MAX_ATTESTER_SLASHINGS,
		MAX_BLS_TO_EXECUTION_CHANGES, MAX_BYTES_PER_TRANSACTION, MAX_DEPOSITS,
		MAX_EXTRA_DATA_BYTES, MAX_PROPOSER_SLASHINGS, MAX_TRANSACTIONS_PER_PAYLOAD,
		MAX_VALIDATORS_PER_COMMITTEE, MAX_VOLUNTARY_EXITS, MAX_WITHDRAWALS_PER_PAYLOAD,
		SYNC_COMMITTEE_SIZE,
	},
};

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
	pub(crate) data: ResponseData,
	version: String,
	execution_optimistic: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseData {
	pub(crate) message: BeaconBlock<
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
	>,
	pub signature: String,
}
