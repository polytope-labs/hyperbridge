use sync_committee_primitives::consensus_types::Validator;

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
	pub(crate) data: ValidatorData,
	execution_optimistic: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ValidatorData {
	pub index: String,
	pub balance: String,
	pub status: String,
	pub(crate) validator: Validator,
}
