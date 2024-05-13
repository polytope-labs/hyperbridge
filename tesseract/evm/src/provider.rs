use crate::{
	abi::{beefy::BeefyConsensusState, EvmHost},
	tx::submit_messages,
	EvmClient,
};
use anyhow::{anyhow, Context, Error};
use beefy_verifier_primitives::ConsensusState;
use codec::Encode;
use ethers::{
	abi::AbiDecode,
	providers::Middleware,
	types::{CallFrame, GethDebugTracingCallOptions, GethTrace, GethTraceFrame},
};
use evm_common::types::EvmStateProof;
use ismp::{
	consensus::{ConsensusStateId, StateMachineId},
	events::Event,
	messaging::{hash_request, hash_response, Message},
};
use ismp_solidity_abi::evm_host::{PostRequestHandledFilter, PostResponseHandledFilter};
use pallet_ismp_host_executive::{EvmHostParam, HostParam};

use crate::{
	gas_oracle::{get_current_gas_cost_in_usd, get_l2_data_cost},
	tx::{generate_contract_calls, get_chain_gas_limit},
};
use ethereum_triedb::StorageProof;
use ethers::{
	contract::parse_log,
	types::{
		CallConfig, GethDebugBuiltInTracerConfig, GethDebugBuiltInTracerType,
		GethDebugTracerConfig, GethDebugTracerType, GethDebugTracingOptions, Log,
	},
};
use futures::stream::{self, FuturesOrdered};
use ismp::{
	consensus::{StateCommitment, StateMachineHeight},
	host::{Ethereum, StateMachine},
	messaging::{CreateConsensusState, ResponseMessage},
	router::{Request, RequestResponse},
};
use primitive_types::U256;
use sp_core::{H160, H256};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tesseract_primitives::{
	BoxStream, EstimateGasReturnParams, Hasher, IsmpProvider, Query, Signature,
	StateMachineUpdated, StateProofQueryType, TxReceipt,
};
use tokio::time;

#[async_trait::async_trait]
impl IsmpProvider for EvmClient {
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		_: ConsensusStateId,
	) -> Result<Vec<u8>, Error> {
		let contract = EvmHost::new(self.config.ismp_host, self.client.clone());
		let value = {
			let call = if let Some(block) = at {
				contract.consensus_state().block(block)
			} else {
				contract.consensus_state()
			};
			call.call().await?
		};

		// Convert these bytes into BeefyConsensusState for rust and scale encode
		let consensus_state: ConsensusState = BeefyConsensusState::decode(&value.0)?.into();
		Ok(consensus_state.encode())
	}

	async fn query_latest_height(&self, id: StateMachineId) -> Result<u32, Error> {
		let id = match id.state_id {
			StateMachine::Polkadot(para_id) => para_id,
			StateMachine::Kusama(para_id) => para_id,
			_ => Err(anyhow!("Unexpected state machine"))?,
		};
		let contract = EvmHost::new(self.config.ismp_host, self.client.clone());
		let value = contract.latest_state_machine_height(id.into()).call().await?;
		Ok(value.low_u64() as u32)
	}

	async fn query_finalized_height(&self) -> Result<u64, Error> {
		let value = self.client.get_block_number().await?;
		Ok(value.low_u64())
	}

	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		let contract = EvmHost::new(self.config.ismp_host, self.client.clone());
		let id = match height.id.state_id {
			StateMachine::Polkadot(para_id) => para_id,
			StateMachine::Kusama(para_id) => para_id,
			_ => Err(anyhow!(
				"Unknown State Machine: {:?} Expected polkadot or kusama state machine",
				height.id.state_id
			))?,
		};
		let state_machine_height = ismp_solidity_abi::shared_types::StateMachineHeight {
			state_machine_id: id.into(),
			height: height.height.into(),
		};
		let commitment = contract.state_machine_commitment(state_machine_height).call().await?;
		Ok(StateCommitment {
			timestamp: commitment.timestamp.low_u64(),
			overlay_root: Some(commitment.overlay_root.into()),
			state_root: commitment.state_root.into(),
		})
	}

	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, Error> {
		let contract = EvmHost::new(self.config.ismp_host, self.client.clone());
		let value =
			contract.state_machine_commitment_update_time(height.try_into()?).call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_challenge_period(&self, _id: ConsensusStateId) -> Result<Duration, Error> {
		let contract = EvmHost::new(self.config.ismp_host, self.client.clone());
		let value = contract.challenge_period().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		let client = Arc::new(self.client.clone());
		let contract = EvmHost::new(self.config.ismp_host, client);
		let value = contract.timestamp().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_requests_proof(&self, at: u64, keys: Vec<Query>) -> Result<Vec<u8>, Error> {
		let keys = keys
			.into_iter()
			.map(|query| self.request_commitment_key(query.commitment).1)
			.collect();

		let proof = self.client.get_proof(self.config.ismp_host, keys, Some(at.into())).await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
			storage_proof: {
				let storage_proofs = proof.storage_proof.into_iter().map(|proof| {
					StorageProof::new(proof.proof.into_iter().map(|bytes| bytes.0.into()))
				});
				let merged_proofs = StorageProof::merge(storage_proofs);
				vec![(
					self.config.ismp_host.0.to_vec(),
					merged_proofs.into_nodes().into_iter().collect(),
				)]
				.into_iter()
				.collect()
			},
		};
		Ok(proof.encode())
	}

	async fn query_responses_proof(&self, at: u64, keys: Vec<Query>) -> Result<Vec<u8>, Error> {
		let keys = keys
			.into_iter()
			.map(|query| self.response_commitment_key(query.commitment).1)
			.collect();
		let proof = self.client.get_proof(self.config.ismp_host, keys, Some(at.into())).await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
			storage_proof: {
				let storage_proofs = proof.storage_proof.into_iter().map(|proof| {
					StorageProof::new(proof.proof.into_iter().map(|bytes| bytes.0.into()))
				});
				let merged_proofs = StorageProof::merge(storage_proofs);
				vec![(
					self.config.ismp_host.0.to_vec(),
					merged_proofs.into_nodes().into_iter().collect(),
				)]
				.into_iter()
				.collect()
			},
		};
		Ok(proof.encode())
	}

	async fn query_state_proof(
		&self,
		at: u64,
		keys: StateProofQueryType,
	) -> Result<Vec<u8>, Error> {
		let state_proof = match keys {
			StateProofQueryType::Ismp(keys) => {
				let mut map: BTreeMap<Vec<u8>, Vec<Vec<u8>>> = BTreeMap::new();
				let locations = keys.iter().map(|key| H256::from_slice(key)).collect();
				let proof = self
					.client
					.get_proof(self.config.ismp_host, locations, Some(at.into()))
					.await?;
				let mut storage_proofs = vec![];
				for proof in proof.storage_proof {
					storage_proofs.push(StorageProof::new(
						proof.proof.into_iter().map(|bytes| bytes.0.into()),
					));
				}

				let storage_proof = StorageProof::merge(storage_proofs);
				map.insert(
					self.config.ismp_host.0.to_vec(),
					storage_proof.into_nodes().into_iter().collect(),
				);

				let state_proof = EvmStateProof {
					contract_proof: proof
						.account_proof
						.into_iter()
						.map(|bytes| bytes.0.into())
						.collect(),
					storage_proof: map,
				};
				state_proof.encode()
			},
			StateProofQueryType::Arbitrary(keys) => {
				let mut contract_proofs: Vec<_> = vec![];
				let mut map: BTreeMap<Vec<u8>, Vec<Vec<u8>>> = BTreeMap::new();
				let mut contract_address_to_proofs = BTreeMap::new();
				for key in keys {
					if key.len() != 52 {
						Err(anyhow!("All arbitrary keys must have a length of 52 when querying state proofs, founf key with length {}", key.len()))?
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

					let entry = contract_address_to_proofs
						.entry(contract_address.0.to_vec())
						.or_insert(vec![]);
					entry.push(StorageProof::new(
						proof
							.storage_proof
							.get(0)
							.cloned()
							.ok_or_else(|| {
								anyhow!(
									"Invalid key supplied, storage proof could not be retrieved"
								)
							})?
							.proof
							.into_iter()
							.map(|bytes| bytes.0.into()),
					));
				}

				for (address, storage_proofs) in contract_address_to_proofs {
					map.insert(
						address,
						StorageProof::merge(storage_proofs).into_nodes().into_iter().collect(),
					);
				}

				let contract_proof = StorageProof::merge(contract_proofs);

				let state_proof = EvmStateProof {
					contract_proof: contract_proof.into_nodes().into_iter().collect(),
					storage_proof: map,
				};
				state_proof.encode()
			},
		};
		Ok(state_proof)
	}

	async fn query_ismp_events(
		&self,
		previous_height: u64,
		event: StateMachineUpdated,
	) -> Result<Vec<Event>, Error> {
		let full_range = (previous_height + 1)..=event.latest_height;
		if full_range.is_empty() {
			return Ok(Default::default());
		}

		let mut events = vec![];
		let chunk_size = self.config.query_batch_size.unwrap_or(1_000_000_000);
		let chunks = full_range.end().saturating_sub(*full_range.start()) / chunk_size;
		for i in 0..=chunks {
			let start = (i * chunk_size) + *full_range.start();
			let end = if i == chunks { *full_range.end() } else { start + chunk_size - 1 };
			let result = self.events(start, end).await;
			match result {
				Ok(batch) => events.extend(batch),
				Err(err) => {
					log::error!(
						"Error while querying events in range {}..{} from {:?}: {err:?}",
						start,
						end,
						self.state_machine
					);
				},
			}
		}

		Ok(events)
	}

	async fn query_request_receipt(&self, hash: H256) -> Result<H160, anyhow::Error> {
		let host_contract = EvmHost::new(self.config.ismp_host, self.signer.clone());
		let address = host_contract.request_receipts(hash.into()).call().await?;
		Ok(address)
	}

	fn name(&self) -> String {
		format!("{:?}", self.state_machine)
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
		use tokio_stream::StreamExt;
		let messages = _msg.clone();

		let debug_trace_call_options = GethDebugTracingCallOptions {
			tracing_options: GethDebugTracingOptions {
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
			},
			..GethDebugTracingCallOptions::default()
		};

		let calls = generate_contract_calls(self, messages).await?;
		let gas_breakdown = get_current_gas_cost_in_usd(
			self.chain_id,
			self.state_machine,
			&self.config.etherscan_api_key.clone(),
			self.client.clone(),
			self.config.gas_price_buffer,
		)
		.await?;
		let mut gas_estimates = vec![];
		let batch_size = self.config.tracing_batch_size.unwrap_or(10);
		for (calls, msgs) in calls.chunks(batch_size).zip(_msg.chunks(batch_size)) {
			let processes = calls
				.into_iter()
				.zip(msgs)
				.map(|(call, _msg)| {
					let client = self.clone();
					let debug_trace_call_options = debug_trace_call_options.clone();
					let mut call = call.clone();
					let _msg = _msg.clone();
					tokio::spawn(async move {
						let address = H160::from_slice(client.address().as_slice());
						call.tx.set_from(address);

						let call_debug = client
							.client
							.debug_trace_call(
								call.tx.clone(),
								call.block.clone(),
								debug_trace_call_options,
							)
							.await?;
						let mut gas_to_be_used = U256::zero();
						let mut successful_execution = false;

						match call_debug {
							GethTrace::Known(GethTraceFrame::CallTracer(call_frame)) => {
								match _msg {
									Message::Request(_) => {
										successful_execution = check_trace_for_event(
											call_frame.clone(),
											CheckTraceForEventParams::Request,
										);
										if !successful_execution {
											log::trace!(
												"debug_traceCall request message failed on {:?}",
												client.state_machine
											);
										}
									},
									Message::Response(_) => {
										successful_execution = check_trace_for_event(
											call_frame.clone(),
											CheckTraceForEventParams::Response,
										);
										if !successful_execution {
											log::trace!(
												"debug_traceCall response message failed on {:?}",
												client.state_machine
											);
										}
									},
									_ => {
										unreachable!("Only request/responses are estimated");
									},
								};

								if successful_execution &&
									client.state_machine ==
										StateMachine::Ethereum(Ethereum::Arbitrum)
								{
									gas_to_be_used =
										client.client.estimate_gas(&call.tx, call.block).await?;
								} else {
									gas_to_be_used = call_frame.gas_used;
								}
							},
							trace => {
								log::error!("an unknown geth trace was reached {trace:?}")
							},
						};

						let gas_cost_for_data_in_usd = match client.state_machine {
							StateMachine::Ethereum(_) =>
								get_l2_data_cost(
									call.tx.rlp(),
									client.state_machine,
									client.client.clone(),
									gas_breakdown.unit_wei_cost,
								)
								.await?,
							_ => U256::zero().into(),
						};

						let execution_cost = (gas_breakdown.gas_price_cost * gas_to_be_used) +
							gas_cost_for_data_in_usd;
						Ok::<_, Error>(EstimateGasReturnParams {
							execution_cost,
							successful_execution,
						})
					})
				})
				.collect::<FuturesOrdered<_>>();

			let estimates = processes
				.collect::<Result<Vec<_>, _>>()
				.await?
				.into_iter()
				.collect::<Result<Vec<_>, _>>()?;

			gas_estimates.extend(estimates);
		}

		Ok(gas_estimates)
	}

	async fn query_request_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		let host_contract = EvmHost::new(self.config.ismp_host, self.signer.clone());
		let fee_metadata = host_contract.request_commitments(hash.into()).call().await?;
		// erc20 tokens are formatted in 18 decimals
		return Ok(fee_metadata.fee);
	}

	async fn query_response_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		let host_contract = EvmHost::new(self.config.ismp_host, self.signer.clone());
		let fee_metadata = host_contract.response_commitments(hash.into()).call().await?;
		// erc20 tokens are formatted in 18 decimals
		return Ok(fee_metadata.fee);
	}

	async fn state_machine_update_notification(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, Error> {
		use futures::StreamExt;
		let interval = time::interval(Duration::from_secs(self.config.poll_interval.unwrap_or(10)));
		let initial_height = self.client.get_block_number().await?.low_u64();
		let stream = stream::unfold(
			(initial_height, interval, self.clone()),
			move |(latest_height, mut interval, client)| async move {
				let state_machine = client.state_machine;
				interval.tick().await;

				// wait for an update with a greater height
				let block_number = match client.client.get_block_number().await {
					Ok(number) => number.low_u64(),
					Err(err) =>
						return Some((
							Err(anyhow!(
								"Error fetching latest block height on {state_machine:?} {err:?}"
							)),
							(latest_height, interval, client),
						)),
				};

				if block_number <= latest_height {
					return Some((Ok(None), (latest_height, interval, client)))
				}

				let contract = EvmHost::new(client.config.ismp_host, client.client.clone());
				let results = match contract
						.events()
						.address(client.config.ismp_host.into())
						.from_block(latest_height)
						.to_block(block_number)
						.query()
						.await
					{
						Ok(events) => events,
						Err(err) =>
							// If the query failed we still advance the latest known height
							return Some((
								Err(err).context(format!(
									"Failed to query state machine updates in range {latest_height:?}..{block_number:?} on {state_machine:?}"
								)),
								(block_number, interval, client),
							)),
					};
				let event = results
					.into_iter()
					.filter_map(|ev| match Event::try_from(ev) {
						Ok(Event::StateMachineUpdated(update)) => Some(update),
						_ => None
					})
					.max_by(|a, b| a.latest_height.cmp(&b.latest_height));

				if let Some(event) = event {
					return Some((
						Ok(Some(event.clone())),
						(block_number + 1, interval, client),
					))
				} else {
					return Some((Ok(None), (block_number + 1, interval, client)))
				}
			},
		).filter_map(|res| async move {
			match res {
				Ok(Some(update)) => Some(Ok(update)),
				Ok(None) => None,
				Err(err) => Some(Err(err)),
			}
		});

		Ok(Box::pin(stream))
	}

	async fn submit(&self, messages: Vec<Message>) -> Result<Vec<TxReceipt>, Error> {
		let receipts = submit_messages(&self, messages.clone()).await?;
		let height = self.client.get_block_number().await?.low_u64();
		let mut results = vec![];
		for msg in messages {
			match msg {
				Message::Request(req_msg) =>
					for post in req_msg.requests {
						let req = Request::Post(post);
						let commitment = hash_request::<Hasher>(&req);
						if receipts.contains(&commitment) {
							let tx_receipt = TxReceipt::Request {
								query: Query {
									source_chain: req.source_chain(),
									dest_chain: req.dest_chain(),
									nonce: req.nonce(),
									commitment,
								},
								height,
							};

							results.push(tx_receipt);
						}
					},
				Message::Response(ResponseMessage {
					datagram: RequestResponse::Response(resp),
					..
				}) =>
					for res in resp {
						let commitment = hash_response::<Hasher>(&res);
						let request_commitment = hash_request::<Hasher>(&res.request());
						if receipts.contains(&commitment) {
							let tx_receipt = TxReceipt::Response {
								query: Query {
									source_chain: res.source_chain(),
									dest_chain: res.dest_chain(),
									nonce: res.nonce(),
									commitment,
								},
								request_commitment,
								height,
							};

							results.push(tx_receipt);
						}
					},
				_ => {},
			}
		}

		Ok(results)
	}

	fn request_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		let key_1 = self.request_commitment_key(commitment).0 .0.to_vec();
		let key_2 = self.request_commitment_key(commitment).1 .0.to_vec();
		vec![key_1, key_2]
	}

	fn request_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		vec![self.request_receipt_key(commitment).0.to_vec()]
	}

	fn response_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		let key_1 = self.response_commitment_key(commitment).0 .0.to_vec();
		let key_2 = self.response_commitment_key(commitment).1 .0.to_vec();
		vec![key_1, key_2]
	}

	fn response_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.response_receipt_key(commitment)
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

	async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		self.set_latest_finalized_height(counterparty).await
	}

	async fn set_initial_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), Error> {
		self.set_consensus_state(message.consensus_state).await?;
		Ok(())
	}

	async fn veto_state_commitment(&self, height: StateMachineHeight) -> Result<(), Error> {
		let contract = EvmHost::new(self.config.ismp_host, self.client.clone());
		if let Some(_) = contract
			.veto_state_commitment(ismp_solidity_abi::beefy::StateMachineHeight {
				state_machine_id: match height.id.state_id {
					StateMachine::Kusama(id) | StateMachine::Polkadot(id) => id.into(),
					_ => Err(anyhow!("Unexpected State machine"))?,
				},
				height: height.height.into(),
			})
			.send()
			.await?
			.await?
		{
			log::info!("Frozen consensus client on {:?}", self.state_machine);
		}
		Ok(())
	}

	async fn query_host_params(
		&self,
		_state_machine: StateMachine,
	) -> Result<HostParam<u128>, anyhow::Error> {
		let contract = EvmHost::new(self.config.ismp_host, self.client.clone());
		let params: ismp_solidity_abi::evm_host::HostParams = contract.host_params().call().await?;
		let evm_params = EvmHostParam {
			default_timeout: params.default_timeout.low_u128(),
			per_byte_fee: params.per_byte_fee.low_u128(),
			fee_token: params.fee_token,
			admin: params.admin,
			handler: params.handler,
			host_manager: params.host_manager,
			un_staking_period: params.un_staking_period.low_u128(),
			challenge_period: params.challenge_period.low_u128(),
			consensus_client: params.consensus_client,
			consensus_state: params
				.consensus_state
				.0
				.to_vec()
				.try_into()
				.map_err(|_| anyhow!("Failed to convert bounded vec"))?,
			consensus_update_timestamp: params.consensus_update_timestamp.low_u128(),
			state_machine_whitelist: params
				.state_machine_whitelist
				.into_iter()
				.map(|id| id.low_u32())
				.collect::<Vec<_>>()
				.try_into()
				.map_err(|_| anyhow!("Failed to convert bounded vec"))?,
			fishermen: params
				.fishermen
				.try_into()
				.map_err(|_| anyhow!("Failed to convert bounded vec"))?,
			hyperbridge: params
				.hyperbridge
				.0
				.to_vec()
				.try_into()
				.map_err(|_| anyhow!("Failed to convert bounded vec"))?,
		};
		Ok(HostParam::EvmHostParam(evm_params))
	}

	fn max_concurrent_queries(&self) -> usize {
		self.config.tracing_batch_size.unwrap_or(10)
	}
}

pub enum CheckTraceForEventParams {
	Request,
	Response,
}

pub fn check_trace_for_event(call_frame: CallFrame, event_in: CheckTraceForEventParams) -> bool {
	if let Some(error) = call_frame.revert_reason {
		log::error!("Error in main call frame: {error}");
	}

	if let Some((logs, frame)) = call_frame
		.calls
		.map(|inner| inner.last().cloned())
		.flatten()
		.map(|last_call_frame| last_call_frame.logs.clone().map(|logs| (logs, last_call_frame)))
		.flatten()
	{
		if let Some(error) = frame.error {
			log::error!("Error in inner call frame: {error}");
		}
		for log in logs {
			let log = Log {
				topics: log.clone().topics.unwrap_or_default(),
				data: log.clone().data.unwrap_or_default(),
				..Default::default()
			};

			match event_in {
				CheckTraceForEventParams::Request => {
					let event = parse_log::<PostRequestHandledFilter>(log.clone());
					match event {
						Ok(_) => return true,
						Err(err) => {
							log::error!("Failed to parse {:?} trace log: {err:?}", frame.to)
						},
					}
				},
				CheckTraceForEventParams::Response => {
					let event = parse_log::<PostResponseHandledFilter>(log.clone());

					match event {
						Ok(_) => return true,
						Err(err) => {
							log::error!("Failed to parse {:?} trace log: {err:?}", frame.to)
						},
					}
				},
			};
		}
	}

	log::error!("Debug trace frame not found!");

	false
}
