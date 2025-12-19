use serde::{Deserialize, Serialize};
use sync_committee_primitives::consensus_types::{BeaconBlockHeader};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub data: ResponseData,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ResponseData {
    pub header: SignedBeaconBlockHeader
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct SignedBeaconBlockHeader {
    pub message: BeaconBlockHeader,
    pub signature: String
}