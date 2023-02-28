use ethereum_consensus::bellatrix::Checkpoint;

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Response {
	execution_optimistic: bool,
	pub data: FinalityCheckpoint,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FinalityCheckpoint {
	pub previous_justified: Checkpoint,
	pub current_justified: Checkpoint,
	pub finalized: Checkpoint,
}
