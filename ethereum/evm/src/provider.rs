use crate::{
	abi::{beefy::BeefyConsensusState, EvmHost, StateMachineUpdatedFilter},
	tx::submit_messages,
	EvmClient,
};
use anyhow::{anyhow, Error};
use beefy_verifier_primitives::{BeefyNextAuthoritySet, ConsensusState};
use codec::Encode;
use ethers::{
	abi::AbiDecode,
	providers::Middleware,
	types::{CallFrame, GethDebugTracingCallOptions, GethTrace, GethTraceFrame},
};
use futures::stream::StreamExt;
use ismp::{
	consensus::{ConsensusStateId, StateMachineId},
	events::Event,
	messaging::Message,
};
use ismp_solidity_abi::evm_host::{PostRequestHandledFilter, PostResponseHandledFilter};
use ismp_sync_committee::types::EvmStateProof;
use jsonrpsee::{
	core::{params::ObjectParams, traits::ToRpcParams},
	rpc_params,
};

use crate::{
	abi::to_state_machine_updated,
	tx::{
		generate_contract_calls, get_chain_gas_limit, get_current_gas_cost_in_usd, get_l2_data_cost,
	},
};
use ethereum_trie::StorageProof;
use ethers::{
	contract::parse_log,
	middleware::MiddlewareBuilder,
	types::{
		CallConfig, GethDebugBuiltInTracerConfig, GethDebugBuiltInTracerType,
		GethDebugTracerConfig, GethDebugTracerType, GethDebugTracingOptions, Log,
	},
};
use ismp::{
	consensus::{StateCommitment, StateMachineHeight},
	host::StateMachine,
	messaging::CreateConsensusState,
};
use primitive_types::U256;
use sp_core::{H160, H256};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tesseract_primitives::{
	BoxStream, EstimateGasReturnParams, IsmpHost, IsmpProvider, NonceProvider, Query, Signature,
	StateMachineUpdated,
};

#[async_trait::async_trait]
impl<I: IsmpHost> IsmpProvider for EvmClient<I>
where
	I: Send + Sync,
{
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		_: ConsensusStateId,
	) -> Result<Vec<u8>, Error> {
		let contract = EvmHost::new(self.ismp_host, self.client.clone());
		let value = {
			let call = if let Some(block) = at {
				contract.consensus_state().block(block)
			} else {
				contract.consensus_state()
			};
			call.call().await?
		};

		let beefy_consensus_state = BeefyConsensusState::decode(&value.0)?;
		// Convert this bytes into BeefyConsensusState for rust and scale encode
		let consensus_state = ConsensusState {
			latest_beefy_height: beefy_consensus_state.latest_height.as_u32(),
			mmr_root_hash: Default::default(),
			beefy_activation_block: beefy_consensus_state.beefy_activation_block.as_u32(),
			current_authorities: BeefyNextAuthoritySet {
				id: beefy_consensus_state.current_authority_set.id.as_u64(),
				len: beefy_consensus_state.current_authority_set.len.as_u32(),
				keyset_commitment: H256::from_slice(
					beefy_consensus_state.current_authority_set.root.as_slice(),
				),
			},
			next_authorities: BeefyNextAuthoritySet {
				id: beefy_consensus_state.next_authority_set.id.as_u64(),
				len: beefy_consensus_state.next_authority_set.len.as_u32(),
				keyset_commitment: H256::from_slice(
					beefy_consensus_state.next_authority_set.root.as_slice(),
				),
			},
		};
		Ok(consensus_state.encode())
	}

	async fn query_latest_height(&self, _id: StateMachineId) -> Result<u32, Error> {
		let contract = EvmHost::new(self.ismp_host, self.client.clone());
		let value = contract.latest_state_machine_height().call().await?;
		Ok(value.low_u64() as u32)
	}

	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		let contract = EvmHost::new(self.ismp_host, self.client.clone());
		let state_machine_height = ismp_solidity_abi::shared_types::StateMachineHeight {
			state_machine_id: Default::default(),
			height: height.height.into(),
		};
		let commitment = contract.state_machine_commitment(state_machine_height).call().await?;
		Ok(StateCommitment {
			timestamp: commitment.timestamp.low_u64(),
			overlay_root: Some(commitment.overlay_root.into()),
			state_root: commitment.state_root.into(),
		})
	}

	async fn query_consensus_update_time(&self, _id: ConsensusStateId) -> Result<Duration, Error> {
		let contract = EvmHost::new(self.ismp_host, self.client.clone());
		let value = contract.consensus_update_time().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_challenge_period(&self, _id: ConsensusStateId) -> Result<Duration, Error> {
		let contract = EvmHost::new(self.ismp_host, self.client.clone());
		let value = contract.challenge_period().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		let client = Arc::new(self.client.clone());
		let contract = EvmHost::new(self.ismp_host, client);
		let value = contract.timestamp().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_requests_proof(&self, at: u64, keys: Vec<Query>) -> Result<Vec<u8>, Error> {
		let keys = keys
			.into_iter()
			.map(|query| self.request_commitment_key(query.commitment))
			.collect();

		let proof = self.client.get_proof(self.ismp_host, keys, Some(at.into())).await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
			storage_proof: proof
				.storage_proof
				.into_iter()
				.map(|proof| {
					(
						sp_core::keccak_256(&proof.key.0).to_vec(),
						proof.proof.into_iter().map(|bytes| bytes.0.into()).collect(),
					)
				})
				.collect(),
		};
		Ok(proof.encode())
	}

	async fn query_responses_proof(&self, at: u64, keys: Vec<Query>) -> Result<Vec<u8>, Error> {
		let keys = keys
			.into_iter()
			.map(|query| self.response_commitment_key(query.commitment))
			.collect();
		let proof = self.client.get_proof(self.ismp_host, keys, Some(at.into())).await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
			storage_proof: proof
				.storage_proof
				.into_iter()
				.map(|proof| {
					(
						sp_core::keccak_256(&proof.key.0).to_vec(),
						proof.proof.into_iter().map(|bytes| bytes.0.into()).collect(),
					)
				})
				.collect(),
		};
		Ok(proof.encode())
	}

	async fn query_state_proof(&self, at: u64, keys: Vec<Vec<u8>>) -> Result<Vec<u8>, Error> {
		let ismp_proof = keys.iter().all(|key| key.len() == 32);
		let state_proof = if ismp_proof {
			let mut map: BTreeMap<Vec<u8>, Vec<Vec<u8>>> = BTreeMap::new();
			let locations = keys.iter().map(|key| H256::from_slice(key)).collect();
			let proof = self.client.get_proof(self.ismp_host, locations, Some(at.into())).await?;
			for (index, key) in keys.into_iter().enumerate() {
				map.insert(
					key,
					proof
						.storage_proof
						.get(index)
						.cloned()
						.ok_or_else(|| {
							anyhow!("Invalid key supplied, storage proof could not be retrieved")
						})?
						.proof
						.into_iter()
						.map(|bytes| bytes.0.into())
						.collect(),
				);
			}

			let state_proof = EvmStateProof {
				contract_proof: proof
					.account_proof
					.into_iter()
					.map(|bytes| bytes.0.into())
					.collect(),
				storage_proof: map,
			};
			state_proof.encode()
		} else {
			let mut contract_proofs: Vec<_> = vec![];
			let mut map: BTreeMap<Vec<u8>, Vec<Vec<u8>>> = BTreeMap::new();
			for key in keys {
				if key.len() != 52 {
					continue
				}

				let contract_address = H160::from_slice(&key[..20]);
				let slot_hash = H256::from_slice(&key[20..]);
				let proof = self
					.client
					.get_proof(contract_address, vec![slot_hash], Some(at.into()))
					.await?;
				contract_proofs.push(StorageProof::new(
					proof.account_proof.into_iter().map(|node| node.0.into()),
				));
				map.insert(
					key,
					proof
						.storage_proof
						.get(0)
						.cloned()
						.ok_or_else(|| {
							anyhow!("Invalid key supplied, storage proof could not be retrieved")
						})?
						.proof
						.into_iter()
						.map(|bytes| bytes.0.into())
						.collect(),
				);
			}
			let contract_proof = StorageProof::merge(contract_proofs);

			let state_proof = EvmStateProof {
				contract_proof: contract_proof.into_nodes().into_iter().collect(),
				storage_proof: map,
			};
			state_proof.encode()
		};

		Ok(state_proof)
	}

	async fn query_ismp_events(
		&self,
		previous_height: u64,
		event: StateMachineUpdated,
	) -> Result<Vec<Event>, Error> {
		let range = (previous_height + 1)..=event.latest_height;
		if range.is_empty() {
			return Ok(Default::default())
		}
		let events = self.events(previous_height + 1, event.latest_height).await?;
		log::info!("querying: {range:?}");
		Ok(events)
	}

	fn name(&self) -> String {
		self.state_machine.to_string()
	}

	fn state_machine_id(&self) -> StateMachineId {
		StateMachineId { state_id: self.state_machine, consensus_state_id: self.consensus_state_id }
	}

	fn block_max_gas(&self) -> u64 {
		get_chain_gas_limit(self.state_machine)
	}

	fn initial_height(&self) -> u64 {
		self.initial_height
	}

	/// Returns gas estimate for message excecution and it value in USD.
	async fn estimate_gas(
		&self,
		_msg: Vec<Message>,
	) -> Result<Vec<EstimateGasReturnParams>, Error> {
		let messages = _msg.clone();

		let debug_trace_option = GethDebugTracingOptions {
			disable_storage: Some(true),
			enable_memory: Some(false),
			tracer: Some(GethDebugTracerType::BuiltInTracer(
				GethDebugBuiltInTracerType::CallTracer,
			)),
			tracer_config: Some(GethDebugTracerConfig::BuiltInTracer(
				GethDebugBuiltInTracerConfig::CallTracer(CallConfig {
					only_top_call: Some(false),
					with_log: Some(true),
				}),
			)),
			..Default::default()
		};

		let debug_trace_call_option = GethDebugTracingCallOptions {
			tracing_options: debug_trace_option,
			..GethDebugTracingCallOptions::default()
		};

		let calls = generate_contract_calls(self, messages, true).await?;
		let mut gas_estimates = Vec::new();
		let (unit_gas_cost, _) = get_current_gas_cost_in_usd(
			self.chain_id,
			self.state_machine,
			&self.etherscan_keys.clone(),
			self.client.clone(),
		)
		.await?;
		for (i_, call) in calls.clone().into_iter().enumerate() {
			let call_debug = self
				.client
				.debug_trace_call(call.tx, call.block, debug_trace_call_option.clone())
				.await?;
			let mut gas_to_be_used = U256::zero();
			let mut successful_execution = false;

			match _msg[i_] {
				Message::Request(_) | Message::Response(_) => match call_debug {
					GethTrace::Known(geth_trace_frame) =>
						match geth_trace_frame {
							GethTraceFrame::CallTracer(call_frame) => {
								match _msg[0] {
									Message::Request(_) => {
										successful_execution = check_trace_for_event(
											call_frame.clone(),
											CheckTraceForEventParams::Request,
										);
										log::debug!("estimate 'request' message success {successful_execution}");
									},
									Message::Response(_) => {
										successful_execution = check_trace_for_event(
											call_frame.clone(),
											CheckTraceForEventParams::Response,
										);
										log::debug!("estimate 'response' message success {successful_execution}");
									},
									_ => {},
								};

								gas_to_be_used = call_frame.gas_used / U256::from(1_000_000_000u64);
							},
							_ => {},
						},
					GethTrace::Unknown(value) => {
						log::error!("an unknown geth trace was reached {value}")
					},
				},
				_ => {
					gas_to_be_used = calls[0].estimate_gas().await? / U256::from(1_000_000_000u64);

					if gas_to_be_used == U256::zero() {
						log::error!("an error occurred while estimating this message submitting")
					}
				},
			}

			let mut gas_cost_for_data_in_gwei = U256::zero();

			match self.state_machine {
				StateMachine::Ethereum(_) => {
					gas_cost_for_data_in_gwei = get_l2_data_cost(
						calls[0].clone().tx.rlp(),
						self.state_machine,
						self.client.clone(),
					)
					.await?;
				},
				_ => {},
			}

			let execution_cost = (gas_to_be_used + gas_cost_for_data_in_gwei) * unit_gas_cost;

			gas_estimates.push(EstimateGasReturnParams { execution_cost, successful_execution });
		}

		Ok(gas_estimates)
	}

	async fn get_message_request_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		let host_contract = EvmHost::new(self.ismp_host, self.signer.clone());
		let fee_metadata = host_contract.request_commitments(hash.into()).call().await?;
		return Ok(fee_metadata.fee);
	}

	async fn query_message_response_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		let host_contract = EvmHost::new(self.ismp_host, self.signer.clone());
		let fee_metadata = host_contract.response_commitments(hash.into()).call().await?;
		return Ok(fee_metadata.fee);
	}

	async fn state_machine_update_notification(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, Error> {
		use ethers::contract::parse_log;
		let mut obj = ObjectParams::new();
		let address = format!("{:?}", self.handler);
		obj.insert("address", address.as_str())
			.expect("handler address should be valid");
		let param = obj.to_rpc_params().ok().flatten().expect("Failed to serialize rpc params");
		let sub = self
			.rpc_client
			.subscribe(
				"eth_subscribe".to_string(),
				rpc_params!("logs", param),
				"eth_unsubscribe".to_string(),
			)
			.await?;
		let stream = sub.filter_map(|log| async move {
			log.ok().and_then(|raw| {
				let log = serde_json::from_str(raw.get()).ok()?;
				parse_log::<StateMachineUpdatedFilter>(log)
					.ok()
					.map(|ev| Ok(to_state_machine_updated(ev)))
			})
		});

		Ok(Box::pin(stream))
	}

	async fn submit(&self, messages: Vec<Message>) -> Result<(), Error> {
		submit_messages(&self, messages).await?;
		Ok(())
	}

	fn request_commitment_full_key(&self, commitment: H256) -> Vec<u8> {
		self.request_commitment_key(commitment).0.to_vec()
	}

	fn request_receipt_full_key(&self, commitment: H256) -> Vec<u8> {
		self.request_receipt_key(commitment).0.to_vec()
	}

	fn response_commitment_full_key(&self, commitment: H256) -> Vec<u8> {
		self.response_commitment_key(commitment).0.to_vec()
	}

	fn response_receipt_full_key(&self, commitment: H256) -> Vec<u8> {
		self.response_receipt_key(commitment).0.to_vec()
	}

	fn address(&self) -> Vec<u8> {
		self.address.clone()
	}

	fn sign(&self, msg: &[u8]) -> Signature {
		let signature = self
			.signer
			.signer()
			.sign_hash(H256::from_slice(msg))
			.expect("Infallible")
			.to_vec();
		Signature::Ethereum { address: self.address.clone(), signature }
	}

	async fn initialize_nonce(&self) -> Result<NonceProvider, Error> {
		let nonce = self
			.client
			.clone()
			.nonce_manager(self.signer.address())
			.initialize_nonce(None)
			.await?
			.as_u64();
		Ok(NonceProvider::new(nonce))
	}

	fn set_nonce_provider(&mut self, nonce_provider: NonceProvider) {
		self.nonce_provider = Some(nonce_provider);
	}

	async fn set_initial_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), Error> {
		self.set_consensus_state(message.consensus_state).await?;
		Ok(())
	}

	async fn freeze_state_machine(&self, _id: StateMachineId) -> Result<(), Error> {
		let contract = EvmHost::new(self.ismp_host, self.client.clone());
		contract.set_frozen_state(true).nonce(self.get_nonce().await?).call().await?;
		Ok(())
	}
}

pub enum CheckTraceForEventParams {
	Request,
	Response,
}

pub fn check_trace_for_event(call_frame: CallFrame, event_in: CheckTraceForEventParams) -> bool {
	if let Some(inner_call) = call_frame.calls {
		if let Some(last_call_frame) = inner_call.last().cloned() {
			if let Some(logs) = last_call_frame.logs {
				for log in logs {
					let log = Log {
						topics: log.clone().topics.unwrap_or_default(),
						data: log.clone().data.unwrap_or_default(),
						..Default::default()
					};

					match event_in {
						CheckTraceForEventParams::Request => {
							let event = parse_log::<PostRequestHandledFilter>(log.clone());

							if let Ok(_) = event {
								return true;
							}
						},
						CheckTraceForEventParams::Response => {
							let event = parse_log::<PostResponseHandledFilter>(log.clone());

							if let Ok(_) = event {
								return true;
							}
						},
					};
				}
			}
		}
	}

	false
}
