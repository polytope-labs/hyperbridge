use ethereum_consensus::bellatrix::mainnet::{
    SYNC_COMMITTEE_SIZE,
};
use ethereum_consensus::bellatrix::{SyncCommittee};


#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub(crate) data: SyncCommittee<SYNC_COMMITTEE_SIZE>,
    execution_optimistic: bool,
}
