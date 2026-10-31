use super::{AbiType, EventSpec, FieldSpec, PalletSpec};

pub fn spec() -> PalletSpec {
	PalletSpec {
		key: "balances",
		metadata_name: "Balances",
		events: vec![EventSpec::new(
			"Transfer",
			vec![
				FieldSpec::indexed("from", "from", AbiType::Bytes32),
				FieldSpec::indexed("to", "to", AbiType::Bytes32),
				FieldSpec::new("amount", "amount", AbiType::Uint(128)),
			],
		)],
	}
}
