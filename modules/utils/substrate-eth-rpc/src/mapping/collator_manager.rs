use super::{AbiType, EventSpec, FieldSpec, PalletSpec};

pub fn spec() -> PalletSpec {
	PalletSpec {
		key: "collatorManager",
		metadata_name: "CollatorManager",
		events: vec![EventSpec::new(
			"CollatorRewarded",
			vec![
				FieldSpec::indexed("collator", "collator", AbiType::Bytes32),
				FieldSpec::new("amount", "amount", AbiType::Uint(128)),
			],
		)],
	}
}
