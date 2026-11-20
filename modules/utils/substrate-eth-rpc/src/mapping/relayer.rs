use super::{AbiType, EventSpec, FieldSpec, PalletSpec};

pub fn spec() -> PalletSpec {
	PalletSpec {
		key: "relayer",
		metadata_name: "Relayer",
		events: vec![
			EventSpec::new(
				"AccumulateFees",
				vec![
					FieldSpec::new("relayer", "address", AbiType::Bytes),
					FieldSpec::new("stateMachine", "state_machine", AbiType::StateMachine),
					FieldSpec::new("amount", "amount", AbiType::Uint(256)),
				],
			),
			EventSpec::new(
				"Withdraw",
				vec![
					FieldSpec::new("relayer", "address", AbiType::Bytes),
					FieldSpec::new("beneficiary", "beneficiary_address", AbiType::Bytes),
					FieldSpec::new("stateMachine", "state_machine", AbiType::StateMachine),
					FieldSpec::new("amount", "amount", AbiType::Uint(256)),
				],
			),
		],
	}
}
