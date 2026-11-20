use super::{state_machine_id, AbiType, EventSpec, FieldSpec, PalletSpec};

pub fn spec() -> PalletSpec {
	PalletSpec {
		key: "ismp",
		metadata_name: "Ismp",
		events: vec![
			EventSpec::new(
				"StateMachineUpdated",
				vec![
					FieldSpec::new("stateMachineId", "state_machine_id", state_machine_id()),
					FieldSpec::new("latestHeight", "latest_height", AbiType::Uint(64)),
				],
			),
			EventSpec::new(
				"Request",
				vec![
					FieldSpec::new("destChain", "dest_chain", AbiType::StateMachine),
					FieldSpec::new("sourceChain", "source_chain", AbiType::StateMachine),
					FieldSpec::new("requestNonce", "request_nonce", AbiType::Uint(64)),
					FieldSpec::indexed("commitment", "commitment", AbiType::Bytes32),
				],
			),
			EventSpec::new(
				"Response",
				vec![
					FieldSpec::new("destChain", "dest_chain", AbiType::StateMachine),
					FieldSpec::new("sourceChain", "source_chain", AbiType::StateMachine),
					FieldSpec::new("requestNonce", "request_nonce", AbiType::Uint(64)),
					FieldSpec::indexed("commitment", "commitment", AbiType::Bytes32),
					FieldSpec::indexed("reqCommitment", "req_commitment", AbiType::Bytes32),
				],
			),
			EventSpec::new("PostRequestHandled", request_response_handled()),
			EventSpec::new("GetRequestHandled", request_response_handled()),
			EventSpec::new("PostRequestTimeoutHandled", timeout_handled()),
			EventSpec::new("GetRequestTimeoutHandled", timeout_handled()),
		],
	}
}

fn request_response_handled() -> Vec<FieldSpec> {
	vec![
		FieldSpec::indexed("commitment", "commitment", AbiType::Bytes32),
		FieldSpec::new("relayer", "relayer", AbiType::Bytes),
	]
}

fn timeout_handled() -> Vec<FieldSpec> {
	vec![
		FieldSpec::indexed("commitment", "commitment", AbiType::Bytes32),
		FieldSpec::new("source", "source", AbiType::StateMachine),
		FieldSpec::new("dest", "dest", AbiType::StateMachine),
	]
}
