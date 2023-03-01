use ethereum_consensus::bellatrix::BeaconBlockHeader;

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
	pub(crate) data: ResponseData,
	execution_optimistic: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseData {
	root: String,
	canonical: bool,
	pub(crate) header: ResponseDataBeaconBlockHeaderMessage,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseDataBeaconBlockHeaderMessage {
	pub message: BeaconBlockHeader,
}
