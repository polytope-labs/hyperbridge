use ethereum_consensus::bellatrix::Checkpoint;

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Response {
    execution_optimistic: bool,
    pub data: ResponseData,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseData {
    previous_justified: Checkpoint,
    current_justified: Checkpoint,
    pub finalized: Checkpoint,
}
