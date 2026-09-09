use sync_committee_primitives::{beacon_state::BeaconBlockSummary, constants::SYNC_COMMITTEE_SIZE};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
	pub(crate) data: ResponseData,
	version: String,
	execution_optimistic: bool,
}

// Only the fields this client reads are named, so one shape deserializes a block from either side
// of the Gloas fork. Blocks are never merkleized here, so the parts left out do not matter.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseData {
	pub(crate) message: BeaconBlockSummary<SYNC_COMMITTEE_SIZE>,
	pub signature: String,
}
