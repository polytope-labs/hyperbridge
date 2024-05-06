#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
	pub(crate) data: NodeSyncCommittee,
	execution_optimistic: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NodeSyncCommittee {
	pub validators: Vec<String>,
	pub validator_aggregates: Vec<Vec<String>>,
}
