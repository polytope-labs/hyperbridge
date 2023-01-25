use ethereum_consensus::bellatrix::{BeaconBlock, BeaconBlockHeader};

//TODO: Remove all these and use the light client primitive consts in the other PR
const MAX_PROPOSER_SLASHINGS: usize = 0;
const MAX_VALIDATORS_PER_COMMITTEE: usize = 0;
const MAX_ATTESTER_SLASHINGS: usize = 0;
const MAX_ATTESTATIONS: usize = 0;
const MAX_DEPOSITS: usize = 0;
const MAX_VOLUNTARY_EXITS: usize = 0;
const SYNC_COMMITTEE_SIZE: usize = 0;
const BYTES_PER_LOGS_BLOOM: usize = 0;
const MAX_EXTRA_DATA_BYTES: usize = 0;
const MAX_BYTES_PER_TRANSACTION: usize = 0;
const MAX_TRANSACTIONS_PER_PAYLOAD: usize = 0;

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub(crate) data: ResponseData,
    version: String,
    execution_optimistic: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseData {
    root: String,
    canonical: bool,
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
    >,
    pub signature: String,
}
