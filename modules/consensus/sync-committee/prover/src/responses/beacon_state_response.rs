use crate::BeaconStateType;

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response<const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize> {
	version: String,
	pub(crate) data: BeaconStateType<ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>,
}
