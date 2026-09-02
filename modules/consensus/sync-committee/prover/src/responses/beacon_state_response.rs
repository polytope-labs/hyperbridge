use ssz_types::typenum::Unsigned;
use crate::BeaconStateType;

// The bounds are type level integers that never appear in a field, so serde's derived
// `T: Deserialize` bounds are spurious and have to be switched off.
#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(bound = "")]
pub struct Response<ETH1_DATA_VOTES_BOUND: Unsigned, PROPOSER_LOOK_AHEAD_LIMIT: Unsigned> {
	version: String,
	pub(crate) data: BeaconStateType<ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>,
}
