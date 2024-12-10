use sync_committee_primitives::consensus_types::BeaconBlockHeader;

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
	pub data: ResponseData,
	execution_optimistic: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseData {
	root: String,
	canonical: bool,
	pub header: ResponseDataBeaconBlockHeaderMessage,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseDataBeaconBlockHeaderMessage {
	pub message: BeaconBlockHeader,
}
