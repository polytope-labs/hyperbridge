use crate::BeaconStateType;

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response<const ETH1_DATA_VOTES_BOUND: usize> {
	version: String,
	pub(crate) data: BeaconStateType<ETH1_DATA_VOTES_BOUND>,
}
