use std::collections::BTreeMap;

use codec::{Compact, Encode};
use primitive_types::H160;
use subxt::{
	dynamic::Value,
	ext::scale_value::{value, Composite},
	Config,
};

use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{
		CreateConsensusState, Message, Proof, ResponseMessage, StateCommitmentHeight,
		TimeoutMessage,
	},
	router::{GetRequest, PostRequest, PostResponse, Request, RequestResponse, Response},
};
use ismp_parachain::ParachainData;
use pallet_hyperbridge::{SubstrateHostParams, VersionedHostParams};
use pallet_ismp_demo::{EvmParams, GetRequest as GetRequestIsmpDemo, TransferParams};
use pallet_ismp_host_executive::{EvmHostParam, HostParam, PerByteFee};
use pallet_ismp_relayer::withdrawal::{Key, Signature, WithdrawalInputData, WithdrawalProof};
use pallet_state_coprocessor::impls::GetRequestsWithProof;

fn to_single_message_value(message: &Message) -> Value<()> {
	match message {
		Message::Consensus(msg) => {
			let inner_struct = Value::named_composite(vec![
				("consensus_proof", Value::from_bytes(msg.consensus_proof.clone())),
				("consensus_state_id", Value::from_bytes(msg.consensus_state_id.to_vec())),
				("signer", Value::from_bytes(msg.signer.clone())),
			]);
			Value::variant("Consensus", Composite::unnamed(vec![inner_struct]))
		},
		Message::FraudProof(msg) => {
			let inner_struct = Value::named_composite(vec![
				("proof_1", Value::from_bytes(msg.proof_1.clone())),
				("proof_2", Value::from_bytes(msg.proof_2.clone())),
				("consensus_state_id", Value::from_bytes(msg.consensus_state_id.to_vec())),
				("signer", Value::from_bytes(msg.signer.clone())),
			]);
			Value::variant("FraudProof", Composite::unnamed(vec![inner_struct]))
		},
		Message::Request(msg) => {
			let inner_struct = Value::named_composite(vec![
				(
					"requests",
					Value::unnamed_composite(msg.requests.iter().map(post_request_to_value)),
				),
				("proof", proof_to_value(&msg.proof)),
				("signer", Value::from_bytes(msg.signer.clone())),
			]);
			Value::variant("Request", Composite::unnamed(vec![inner_struct]))
		},
		Message::Response(msg) => Value::variant("Response", response_message_to_composite(msg)),
		Message::Timeout(msg) => {
			let timeout_variant = timeout_message_to_value(msg);
			Value::variant("Timeout", Composite::unnamed(vec![timeout_variant]))
		},
	}
}

pub fn messages_to_value(messages: Vec<Message>) -> Value<()> {
	let message_values: Vec<Value<()>> = messages.iter().map(to_single_message_value).collect();
	Value::unnamed_composite(message_values)
}

fn response_message_to_composite(msg: &ResponseMessage) -> Composite<()> {
	let datagram_value = match &msg.datagram {
		RequestResponse::Request(reqs) =>
			Value::variant("Request", Composite::unnamed(reqs.iter().map(request_to_value))),
		RequestResponse::Response(resps) =>
			Value::variant("Response", Composite::unnamed(resps.iter().map(response_to_value))),
	};
	Composite::named(vec![
		("datagram".to_string(), datagram_value),
		("proof".to_string(), proof_to_value(&msg.proof)),
		("signer".to_string(), Value::from_bytes(msg.signer.clone())),
	])
}

fn timeout_message_to_value(msg: &TimeoutMessage) -> Value<()> {
	match msg {
		TimeoutMessage::Post { requests, timeout_proof } => Value::variant(
			"Post",
			Composite::named(vec![
				(
					"requests".to_string(),
					Value::unnamed_composite(requests.iter().map(request_to_value)),
				),
				("timeout_proof".to_string(), proof_to_value(timeout_proof)),
			]),
		),
		TimeoutMessage::PostResponse { responses, timeout_proof } => Value::variant(
			"PostResponse",
			Composite::named(vec![
				(
					"responses".to_string(),
					Value::unnamed_composite(responses.iter().map(post_response_to_value)),
				),
				("timeout_proof".to_string(), proof_to_value(timeout_proof)),
			]),
		),
		TimeoutMessage::Get { requests } => Value::variant(
			"Get",
			Composite::named(vec![(
				"requests".to_string(),
				Value::unnamed_composite(requests.iter().map(request_to_value)),
			)]),
		),
	}
}

fn request_to_value(req: &Request) -> Value<()> {
	match req {
		Request::Post(post) => Value::variant(
			"Post",
			Composite::named(vec![
				("source".to_string(), state_machine_to_value(&post.source)),
				("dest".to_string(), state_machine_to_value(&post.dest)),
				("nonce".to_string(), Value::u128(post.nonce.into())),
				("from".to_string(), Value::from_bytes(post.from.clone())),
				("to".to_string(), Value::from_bytes(post.to.clone())),
				("timeout_timestamp".to_string(), Value::u128(post.timeout_timestamp.into())),
				("body".to_string(), Value::from_bytes(post.body.clone())),
			]),
		),
		Request::Get(get) => Value::variant(
			"Get",
			Composite::named(vec![
				("source".to_string(), state_machine_to_value(&get.source)),
				("dest".to_string(), state_machine_to_value(&get.dest)),
				("nonce".to_string(), Value::u128(get.nonce.into())),
				("from".to_string(), Value::from_bytes(get.from.clone())),
				(
					"keys".to_string(),
					Value::unnamed_composite(get.keys.iter().map(|k| Value::from_bytes(k.clone()))),
				),
				("height".to_string(), Value::u128(get.height.into())),
				("context".to_string(), Value::from_bytes(get.context.clone())),
				("timeout_timestamp".to_string(), Value::u128(get.timeout_timestamp.into())),
			]),
		),
	}
}

fn response_to_value(resp: &Response) -> Value<()> {
	match resp {
		Response::Post(post) => Value::variant(
			"Post",
			Composite::named(vec![
				("post".to_string(), post_response_to_value(post)),
				("response".to_string(), Value::from_bytes(post.response.clone())),
				("timeout_timestamp".to_string(), Value::u128(post.timeout_timestamp.into())),
			]),
		),
		Response::Get(get) => Value::variant(
			"Get",
			Composite::named(vec![
				("get".to_string(), get_request_to_value(&get.get)),
				(
					"values".to_string(),
					Value::unnamed_composite(get.values.iter().map(|v| {
						Value::named_composite(vec![
							("key".to_string(), Value::from_bytes(v.key.clone())),
							(
								"value".to_string(),
								Value::from_bytes(v.value.clone().unwrap_or_default()),
							),
						])
					})),
				),
			]),
		),
	}
}

fn post_request_to_value(post: &PostRequest) -> Value<()> {
	Value::named_composite(vec![
		("source".to_string(), state_machine_to_value(&post.source)),
		("dest".to_string(), state_machine_to_value(&post.dest)),
		("nonce".to_string(), Value::u128(post.nonce.into())),
		("from".to_string(), Value::from_bytes(post.from.clone())),
		("to".to_string(), Value::from_bytes(post.to.clone())),
		("timeout_timestamp".to_string(), Value::u128(post.timeout_timestamp.into())),
		("body".to_string(), Value::from_bytes(post.body.clone())),
	])
}

fn get_request_to_value(get: &GetRequest) -> Value<()> {
	Value::named_composite(vec![
		("source".to_string(), state_machine_to_value(&get.source)),
		("dest".to_string(), state_machine_to_value(&get.dest)),
		("nonce".to_string(), Value::u128(get.nonce.into())),
		("from".to_string(), Value::from_bytes(get.from.clone())),
		(
			"keys".to_string(),
			Value::unnamed_composite(get.keys.iter().map(|k| Value::from_bytes(k.clone()))),
		),
		("height".to_string(), Value::u128(get.height.into())),
		("context".to_string(), Value::from_bytes(get.context.clone())),
		("timeout_timestamp".to_string(), Value::u128(get.timeout_timestamp.into())),
	])
}

fn post_response_to_value(post: &PostResponse) -> Value<()> {
	Value::named_composite(vec![
		("post".to_string(), post_request_to_value(&post.post)),
		("response".to_string(), Value::from_bytes(post.response.clone())),
		("timeout_timestamp".to_string(), Value::u128(post.timeout_timestamp.into())),
	])
}

pub fn state_machine_to_value(sm: &StateMachine) -> Value<()> {
	match sm {
		StateMachine::Evm(id) =>
			Value::variant("Evm", Composite::unnamed(vec![Value::u128((*id).into())])),
		StateMachine::Polkadot(id) =>
			Value::variant("Polkadot", Composite::unnamed(vec![Value::u128((*id).into())])),
		StateMachine::Kusama(id) =>
			Value::variant("Kusama", Composite::unnamed(vec![Value::u128((*id).into())])),
		StateMachine::Substrate(id) =>
			Value::variant("Substrate", Composite::unnamed(vec![Value::from_bytes(id.to_vec())])),
		StateMachine::Tendermint(id) =>
			Value::variant("Tendermint", Composite::unnamed(vec![Value::from_bytes(id.to_vec())])),
		StateMachine::Relay { relay, para_id } => {
			let composite = Composite::named(vec![
				("relay".to_string(), Value::from_bytes(relay.to_vec())),
				("para_id".to_string(), Value::u128((*para_id).into())),
			]);
			Value::variant("Relay", composite)
		},
	}
}

pub fn state_machine_id_to_value(state_machine_id: &StateMachineId) -> Value {
	let state_id_value = state_machine_to_value(&state_machine_id.state_id);

	let state_machine_id_value = value!({
		state_id: state_id_value,
		consensus_state_id: state_machine_id.consensus_state_id.to_vec()
	});

	state_machine_id_value
}

pub fn state_machine_height_to_value(height: &StateMachineHeight) -> Value<()> {
	Value::named_composite(vec![
		("id".to_string(), state_machine_id_to_value(&height.id)),
		("height".to_string(), Value::u128(height.height.into())),
	])
}

pub fn create_consensus_state_to_value(data: &CreateConsensusState) -> Value<()> {
	let challenge_periods_value =
		Value::unnamed_composite(data.challenge_periods.iter().map(|(sm, period)| {
			Value::unnamed_composite(vec![
				state_machine_to_value(sm),
				Value::u128((*period).into()),
			])
		}));

	let state_machine_commitments_value = Value::unnamed_composite(
		data.state_machine_commitments.iter().map(|(id, commitment_height)| {
			Value::unnamed_composite(vec![
				state_machine_id_to_value(id),
				state_commitment_height_to_value(commitment_height),
			])
		}),
	);

	Value::named_composite(vec![
		("consensus_state".to_string(), Value::from_bytes(data.consensus_state.clone())),
		("consensus_client_id".to_string(), Value::from_bytes(data.consensus_client_id.to_vec())),
		("consensus_state_id".to_string(), Value::from_bytes(data.consensus_state_id.to_vec())),
		("unbonding_period".to_string(), Value::u128(data.unbonding_period.into())),
		("challenge_periods".to_string(), challenge_periods_value),
		("state_machine_commitments".to_string(), state_machine_commitments_value),
	])
}

fn state_commitment_height_to_value(sch: &StateCommitmentHeight) -> Value<()> {
	Value::named_composite(vec![
		("commitment".to_string(), state_commitment_to_value(&sch.commitment)),
		("height".to_string(), Value::u128(sch.height.into())),
	])
}

fn state_commitment_to_value(sc: &StateCommitment) -> Value<()> {
	let overlay_root_value = match sc.overlay_root {
		Some(root) => Value::variant(
			"Some",
			Composite::unnamed(vec![Value::from_bytes(root.as_bytes().to_vec())]),
		),
		None => Value::variant("None", Composite::unnamed(vec![])),
	};

	Value::named_composite(vec![
		("timestamp".to_string(), Value::u128(sc.timestamp.into())),
		("overlay_root".to_string(), overlay_root_value),
		("state_root".to_string(), Value::from_bytes(sc.state_root.as_bytes().to_vec())),
	])
}

fn versioned_host_params_to_composite(params: &VersionedHostParams<u128>) -> Composite<()> {
	match params {
		VersionedHostParams::V1(p) => {
			let inner_struct_as_value = Value::named_composite(substrate_host_params_to_vec(p));

			Composite::unnamed(vec![inner_struct_as_value])
		},
	}
}

fn substrate_host_params_to_vec(params: &SubstrateHostParams<u128>) -> Vec<(String, Value<()>)> {
	let per_byte_fees_value =
		Value::unnamed_composite(params.per_byte_fees.iter().map(|(sm, fee)| {
			Value::unnamed_composite(vec![state_machine_to_value(sm), Value::u128(*fee)])
		}));

	vec![
		("default_per_byte_fee".to_string(), Value::u128(params.default_per_byte_fee)),
		("per_byte_fees".to_string(), per_byte_fees_value),
		("asset_registration_fee".to_string(), Value::u128(params.asset_registration_fee)),
	]
}

fn evm_host_param_to_composite(param: &EvmHostParam) -> Composite<()> {
	let per_byte_fees_value =
		Value::unnamed_composite(param.per_byte_fees.iter().map(per_byte_fee_to_value));
	let state_machines_value =
		Value::unnamed_composite(param.state_machines.iter().map(|id| Value::u128((*id).into())));
	let hyperbridge_value = Value::from_bytes(param.hyperbridge.as_slice());

	Composite::named(vec![
		("default_timeout".to_string(), Value::u128(param.default_timeout)),
		(
			"default_per_byte_fee".to_string(),
			Value::from_bytes(param.default_per_byte_fee.encode()),
		),
		(
			"state_commitment_fee".to_string(),
			Value::from_bytes(param.state_commitment_fee.encode()),
		),
		("fee_token".to_string(), Value::from_bytes(param.fee_token.0.to_vec())),
		("admin".to_string(), Value::from_bytes(param.admin.0.to_vec())),
		("handler".to_string(), Value::from_bytes(param.handler.0.to_vec())),
		("host_manager".to_string(), Value::from_bytes(param.host_manager.0.to_vec())),
		("uniswap_v2".to_string(), Value::from_bytes(param.uniswap_v2.0.to_vec())),
		("un_staking_period".to_string(), Value::u128(param.un_staking_period)),
		("challenge_period".to_string(), Value::u128(param.challenge_period)),
		("consensus_client".to_string(), Value::from_bytes(param.consensus_client.0.to_vec())),
		("state_machines".to_string(), state_machines_value),
		("per_byte_fees".to_string(), per_byte_fees_value),
		("hyperbridge".to_string(), hyperbridge_value),
	])
}

fn per_byte_fee_to_value(fee: &PerByteFee) -> Value<()> {
	Value::named_composite(vec![
		("state_id".to_string(), Value::from_bytes(fee.state_id.as_bytes().to_vec())),
		("per_byte_fee".to_string(), Value::from_bytes(fee.per_byte_fee.encode())),
	])
}

pub fn withdrawal_proof_to_value(proof: &WithdrawalProof) -> Value<()> {
	let commitments_value = Value::unnamed_composite(proof.commitments.iter().map(key_to_value));

	let beneficiary_details_value = match &proof.beneficiary_details {
		Some((address, signature)) => {
			let inner_value = Value::unnamed_composite(vec![
				Value::from_bytes(address.clone()),
				signature_to_value(signature),
			]);
			Value::variant("Some", Composite::unnamed(vec![inner_value]))
		},
		None => Value::variant("None", Composite::unnamed(vec![])),
	};

	Value::named_composite(vec![
		("commitments".to_string(), commitments_value),
		("source_proof".to_string(), proof_to_value(&proof.source_proof)),
		("dest_proof".to_string(), proof_to_value(&proof.dest_proof)),
		("beneficiary_details".to_string(), beneficiary_details_value),
	])
}

fn key_to_value(key: &Key) -> Value<()> {
	match key {
		Key::Request(commitment) => {
			let inner_value = Value::from_bytes(commitment.as_bytes().to_vec());
			Value::variant("Request", Composite::unnamed(vec![inner_value]))
		},
		Key::Response { request_commitment, response_commitment } => {
			let composite = Composite::named(vec![
				(
					"request_commitment".to_string(),
					Value::from_bytes(request_commitment.as_bytes().to_vec()),
				),
				(
					"response_commitment".to_string(),
					Value::from_bytes(response_commitment.as_bytes().to_vec()),
				),
			]);
			Value::variant("Response", composite)
		},
	}
}

fn proof_to_value(proof: &Proof) -> Value<()> {
	Value::named_composite(vec![
		("height".to_string(), state_machine_height_to_value(&proof.height)),
		("proof".to_string(), Value::from_bytes(proof.proof.clone())),
	])
}

fn signature_to_value(sig: &Signature) -> Value<()> {
	match sig {
		Signature::Evm { address, signature } => {
			let composite = Composite::named(vec![
				("address".to_string(), Value::from_bytes(address.clone())),
				("signature".to_string(), Value::from_bytes(signature.clone())),
			]);
			Value::variant("Evm", composite)
		},
		Signature::Sr25519 { public_key, signature } => {
			let composite = Composite::named(vec![
				("public_key".to_string(), Value::from_bytes(public_key.clone())),
				("signature".to_string(), Value::from_bytes(signature.clone())),
			]);
			Value::variant("Sr25519", composite)
		},
		Signature::Ed25519 { public_key, signature } => {
			let composite = Composite::named(vec![
				("public_key".to_string(), Value::from_bytes(public_key.clone())),
				("signature".to_string(), Value::from_bytes(signature.clone())),
			]);
			Value::variant("Ed25519", composite)
		},
	}
}

pub fn withdrawal_input_data_to_value(data: &WithdrawalInputData) -> Value<()> {
	Value::named_composite(vec![
		("signature".to_string(), signature_to_value(&data.signature)),
		("dest_chain".to_string(), state_machine_to_value(&data.dest_chain)),
	])
}

pub fn get_requests_with_proof_to_value(data: &GetRequestsWithProof) -> Value<()> {
	Value::named_composite(vec![
		(
			"requests".to_string(),
			Value::unnamed_composite(data.requests.iter().map(get_request_to_value)),
		),
		("source".to_string(), proof_to_value(&data.source)),
		("response".to_string(), proof_to_value(&data.response)),
		("address".to_string(), Value::from_bytes(data.address.clone())),
	])
}

pub fn transfer_params_to_value<C: Config>(
	params: &TransferParams<C::AccountId, u128>,
) -> Value<()> {
	Value::named_composite(vec![
		("to".to_string(), Value::from_bytes(params.to.encode())),
		("amount".to_string(), Value::u128(params.amount)),
		("para_id".to_string(), Value::u128(params.para_id.into())),
		("timeout".to_string(), Value::u128(params.timeout.into())),
	])
}

pub fn evm_params_to_value(params: &EvmParams) -> Value<()> {
	Value::named_composite(vec![
		("module".to_string(), Value::from_bytes(params.module.0.to_vec())),
		("destination".to_string(), Value::u128(params.destination.into())),
		("timeout".to_string(), Value::u128(params.timeout.into())),
		("count".to_string(), Value::u128(params.count.into())),
	])
}

pub fn get_request_ismp_demo_to_value(params: &GetRequestIsmpDemo) -> Value<()> {
	let keys_value =
		Value::unnamed_composite(params.keys.iter().map(|key| Value::from_bytes(key.clone())));

	Value::named_composite(vec![
		("para_id".to_string(), Value::u128(params.para_id.into())),
		("height".to_string(), Value::u128(params.height.into())),
		("timeout".to_string(), Value::u128(params.timeout.into())),
		("keys".to_string(), keys_value),
	])
}

pub fn account_vec_to_value<C: Config>(accounts: &Vec<C::AccountId>) -> Value<()> {
	Value::unnamed_composite(accounts.iter().map(|account| Value::from_bytes(account.encode())))
}

pub fn evm_hosts_btreemap_to_value(evm_hosts: &BTreeMap<StateMachine, H160>) -> Value<()> {
	Value::unnamed_composite(evm_hosts.iter().map(|(state_machine, address)| {
		Value::unnamed_composite(vec![
			state_machine_to_value(state_machine),
			Value::from_bytes(address.0.to_vec()),
		])
	}))
}

pub fn compact_u32_to_value(compact_int: Compact<u32>) -> Value<()> {
	Value::from_bytes(compact_int.encode())
}

pub fn host_param_tuple_to_value(
	state_machine: &StateMachine,
	host_param: &HostParam<u128>,
) -> Value<()> {
	let state_machine_value = state_machine_to_value(state_machine);
	let host_param_value = host_param_to_value(host_param);

	Value::unnamed_composite(vec![state_machine_value, host_param_value])
}

fn host_param_to_value(param: &HostParam<u128>) -> Value<()> {
	match param {
		HostParam::SubstrateHostParam(p) => {
			let versioned_host_params_value = match p {
				VersionedHostParams::V1(params_v1) => {
					let per_byte_fees_value = Value::unnamed_composite(
						params_v1.per_byte_fees.iter().map(|(sm, fee)| {
							Value::unnamed_composite(vec![
								state_machine_to_value(sm),
								Value::u128(*fee),
							])
						}),
					);

					let substrate_host_params_value = Value::named_composite(vec![
						(
							"default_per_byte_fee".to_string(),
							Value::u128(params_v1.default_per_byte_fee),
						),
						("per_byte_fees".to_string(), per_byte_fees_value),
						(
							"asset_registration_fee".to_string(),
							Value::u128(params_v1.asset_registration_fee),
						),
					]);

					Value::variant("V1", Composite::unnamed(vec![substrate_host_params_value]))
				},
			};
			Value::variant(
				"SubstrateHostParam",
				Composite::unnamed(vec![versioned_host_params_value]),
			)
		},
		HostParam::EvmHostParam(p) => {
			let evm_host_param_value = Value::named_composite(vec![
				("default_timeout".to_string(), Value::u128(p.default_timeout)),
				(
					"default_per_byte_fee".to_string(),
					Value::from_bytes(p.default_per_byte_fee.encode()),
				),
				(
					"state_commitment_fee".to_string(),
					Value::from_bytes(p.state_commitment_fee.encode()),
				),
				("fee_token".to_string(), Value::from_bytes(p.fee_token.0.to_vec())),
				("admin".to_string(), Value::from_bytes(p.admin.0.to_vec())),
				("handler".to_string(), Value::from_bytes(p.handler.0.to_vec())),
				("host_manager".to_string(), Value::from_bytes(p.host_manager.0.to_vec())),
				("uniswap_v2".to_string(), Value::from_bytes(p.uniswap_v2.0.to_vec())),
				("un_staking_period".to_string(), Value::u128(p.un_staking_period)),
				("challenge_period".to_string(), Value::u128(p.challenge_period)),
				("consensus_client".to_string(), Value::from_bytes(p.consensus_client.0.to_vec())),
				(
					"state_machines".to_string(),
					Value::unnamed_composite(
						p.state_machines.iter().map(|id| Value::u128((*id).into())),
					),
				),
				(
					"per_byte_fees".to_string(),
					Value::unnamed_composite(p.per_byte_fees.iter().map(per_byte_fee_to_value)),
				),
				("hyperbridge".to_string(), Value::from_bytes(p.hyperbridge.clone())),
			]);
			Value::variant("EvmHostParam", Composite::unnamed(vec![evm_host_param_value]))
		},
	}
}

pub fn host_params_btreemap_to_value(
	params: &BTreeMap<StateMachine, HostParam<u128>>,
) -> Value<()> {
	let value_pairs: Vec<Value<()>> = params
		.iter()
		.map(|(state_machine, host_param)| {
			Value::unnamed_composite(vec![
				state_machine_to_value(state_machine),
				host_param_to_value(host_param),
			])
		})
		.collect();

	Value::unnamed_composite(value_pairs)
}

pub fn storage_kv_list_to_value(kv_list: &Vec<(Vec<u8>, Vec<u8>)>) -> Value<()> {
	let value_pairs: Vec<Value<()>> = kv_list
		.iter()
		.map(|(key, value)| {
			Value::unnamed_composite(vec![
				Value::from_bytes(key.clone()),
				Value::from_bytes(value.clone()),
			])
		})
		.collect();

	Value::unnamed_composite(value_pairs)
}

pub fn parachain_data_to_value(data: &ParachainData) -> Value<()> {
	Value::named_composite(vec![
		("id".to_string(), Value::u128(data.id.into())),
		("slot_duration".to_string(), Value::u128(data.slot_duration.into())),
	])
}
