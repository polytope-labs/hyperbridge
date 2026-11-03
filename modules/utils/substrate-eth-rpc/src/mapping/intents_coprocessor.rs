use super::{AbiType, EventSpec, FieldSpec, PalletSpec};

pub fn spec() -> PalletSpec {
	PalletSpec {
		key: "intentsCoprocessor",
		metadata_name: "IntentsCoprocessor",
		events: vec![EventSpec::new(
			"BidPlaced",
			vec![
				FieldSpec::indexed("filler", "filler", AbiType::Bytes32),
				FieldSpec::indexed("commitment", "commitment", AbiType::Bytes32),
				FieldSpec::new("bid", "bid", AbiType::Bytes32),
				FieldSpec::new("deposit", "deposit", AbiType::Uint(128)),
			],
		)],
	}
}
