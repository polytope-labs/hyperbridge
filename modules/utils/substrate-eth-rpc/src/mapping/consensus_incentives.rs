use super::{state_machine_id, AbiType, EventSpec, FieldSpec, PalletSpec};

pub fn spec() -> PalletSpec {
	PalletSpec {
		key: "consensusIncentives",
		metadata_name: "ConsensusIncentives",
		events: vec![EventSpec::new(
			"RelayerRewarded",
			vec![
				FieldSpec::indexed("relayer", "relayer", AbiType::Bytes32),
				FieldSpec::new("amount", "amount", AbiType::Uint(128)),
				FieldSpec::new(
					"stateMachineHeight",
					"state_machine_height",
					AbiType::Tuple(vec![
						FieldSpec::new("id", "id", state_machine_id()),
						FieldSpec::new("height", "height", AbiType::Uint(64)),
					]),
				),
			],
		)],
	}
}
