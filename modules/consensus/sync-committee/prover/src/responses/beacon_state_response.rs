use sync_committee_primitives::{
    consensus_types::BeaconState,
    constants::{
        BYTES_PER_LOGS_BLOOM, EPOCHS_PER_HISTORICAL_VECTOR, EPOCHS_PER_SLASHINGS_VECTOR,
        ETH1_DATA_VOTES_BOUND, HISTORICAL_ROOTS_LIMIT, MAX_EXTRA_DATA_BYTES,
        MAX_VALIDATORS_PER_COMMITTEE, SLOTS_PER_HISTORICAL_ROOT, SYNC_COMMITTEE_SIZE,
        VALIDATOR_REGISTRY_LIMIT,
    },
};

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
    version: String,
    pub(crate) data: BeaconState<
        SLOTS_PER_HISTORICAL_ROOT,
        HISTORICAL_ROOTS_LIMIT,
        ETH1_DATA_VOTES_BOUND,
        VALIDATOR_REGISTRY_LIMIT,
        EPOCHS_PER_HISTORICAL_VECTOR,
        EPOCHS_PER_SLASHINGS_VECTOR,
        MAX_VALIDATORS_PER_COMMITTEE,
        SYNC_COMMITTEE_SIZE,
        BYTES_PER_LOGS_BLOOM,
        MAX_EXTRA_DATA_BYTES,
    >,
}
