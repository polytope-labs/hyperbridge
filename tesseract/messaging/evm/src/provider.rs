use crate::{
	abi::{erc_20::Erc20Instance, EvmHostInstance},
	state_comitment_key, EvmClient,
};
use anyhow::{anyhow, Error};
use beefy_verifier_primitives::ConsensusState;
use codec::Encode;
use alloy::{
	primitives::{Address, B256, U256 as AlloyU256},
	providers::Provider,
	sol_types::SolType,
};
use alloy_sol_types::SolEvent;

use evm_state_machine::types::EvmStateProof;

fn alloy_u256_to_primitive(val: AlloyU256) -> primitive_types::U256 {
	primitive_types::U256::from_little_endian(&val.to_le_bytes::<32>())
}
use ismp::{
	consensus::{ConsensusStateId, StateMachineId},
	events::{Event, StateCommitmentVetoed},
	messaging::{Message, StateCommitmentHeight},
};
use ismp_solidity_abi::evm_host::{PostRequestHandled, PostResponseHandled};
use pallet_ismp_host_executive::{EvmHostParam, HostParam, PerByteFee};

use crate::{
	gas_oracle::{get_current_gas_cost_in_usd, get_l2_data_cost},
	tx::get_chain_gas_limit,
};
use ethereum_triedb::StorageProof;
use futures::{stream::FuturesOrdered, FutureExt};
use ismp::{
	consensus::{StateCommitment, StateMachineHeight},
	host::StateMachine,
	messaging::CreateConsensusState,
};
use primitive_types::U256;
use sp_core::{H160, H256};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tesseract_primitives::{
	wait_for_challenge_period, BoxStream, EstimateGasReturnParams, IsmpProvider, Query, Signature,
	StateMachineUpdated, StateProofQueryType, TxResult,
};

use ismp_solidity_abi::beefy::BeefyConsensusState;

#[async_trait::async_trait]
impl IsmpProvider for EvmClient {
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		_: ConsensusStateId,
	) -> Result<Vec<u8>, Error> {
		use alloy::eips::BlockId;
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let contract = EvmHostInstance::new(host_addr, self.client.clone());
		let value = {
			let call = contract.consensusState();
			if let Some(block) = at {
				call.block(BlockId::number(block)).call().await?
			} else {
				call.call().await?
			}
		};

		// Convert these bytes into BeefyConsensusState for rust and scale encode
		let consensus_state: ConsensusState = BeefyConsensusState::abi_decode(&value)?.into();
		Ok(consensus_state.encode())
	}

	async fn query_latest_height(&self, id: StateMachineId) -> Result<u32, Error> {
		let id = match id.state_id {
			StateMachine::Polkadot(para_id) => para_id,
			StateMachine::Kusama(para_id) => para_id,
			_ => Err(anyhow!("Unexpected state machine"))?,
		};
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let contract = EvmHostInstance::new(host_addr, self.client.clone());
		let value = contract.latestStateMachineHeight(AlloyU256::from(id)).call().await?;
		Ok(value.try_into().unwrap_or(0))
	}

	async fn query_finalized_height(&self) -> Result<u64, Error> {
		let value = self.client.get_block_number().await?;
		Ok(value)
	}

	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		let id = match height.id.state_id {
			StateMachine::Polkadot(para_id) => para_id,
			StateMachine::Kusama(para_id) => para_id,
			_ => Err(anyhow!(
				"Unknown State Machine: {:?} Expected polkadot or kusama state machine",
				height.id.state_id
			))?,
		};
		let (timestamp_key, overlay_key, state_root_key) =
			state_comitment_key(id.into(), height.height.into());
		let host_addr = Address::from_slice(&self.config.ismp_host.0);

		let timestamp = {
			let timestamp = self
				.client
				.get_storage_at(host_addr, B256::from_slice(&timestamp_key.0).into())
				.await?;
			U256::from_big_endian(&timestamp.to_be_bytes::<32>()).low_u64()
		};
		let overlay_root = self
			.client
			.get_storage_at(host_addr, B256::from_slice(&overlay_key.0).into())
			.await?;
		let state_root = self
			.client
			.get_storage_at(host_addr, B256::from_slice(&state_root_key.0).into())
			.await?;
		Ok(StateCommitment {
			timestamp,
			overlay_root: Some(H256::from_slice(&overlay_root.to_be_bytes::<32>())),
			state_root: H256::from_slice(&state_root.to_be_bytes::<32>()),
		})
	}

	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, Error> {
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let contract = EvmHostInstance::new(host_addr, self.client.clone());
		let value =
			contract.stateMachineCommitmentUpdateTime(height.try_into()?).call().await?;
		Ok(Duration::from_secs(value.try_into().unwrap_or(0)))
	}

	async fn query_challenge_period(&self, _id: StateMachineId) -> Result<Duration, Error> {
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let contract = EvmHostInstance::new(host_addr, self.client.clone());
		let value = contract.challengePeriod().call().await?;
		Ok(Duration::from_secs(value.try_into().unwrap_or(0)))
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let contract = EvmHostInstance::new(host_addr, self.client.clone());
		let value = contract.timestamp().call().await?;
		Ok(Duration::from_secs(value.try_into().unwrap_or(0)))
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let keys: Vec<B256> = keys
			.into_iter()
			.map(|query| B256::from_slice(&self.request_commitment_key(query.commitment).1.0))
			.collect();

		let proof = self
			.client
			.get_proof(host_addr, keys)
			.block_id(at.into())
			.await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.to_vec()).collect(),
			storage_proof: {
				let storage_proofs = proof.storage_proof.into_iter().map(|proof| {
					StorageProof::new(proof.proof.into_iter().map(|bytes| bytes.to_vec()))
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

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let keys: Vec<B256> = keys
			.into_iter()
			.map(|query| B256::from_slice(&self.response_commitment_key(query.commitment).1.0))
			.collect();

		let proof = self
			.client
			.get_proof(host_addr, keys)
			.block_id(at.into())
			.await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.to_vec()).collect(),
			storage_proof: {
				let storage_proofs = proof.storage_proof.into_iter().map(|proof| {
					StorageProof::new(proof.proof.into_iter().map(|bytes| bytes.to_vec()))
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
				let host_addr = Address::from_slice(&self.config.ismp_host.0);
				let locations: Vec<B256> = keys.iter().map(|key| B256::from_slice(key)).collect();
				let proof = self
					.client
					.get_proof(host_addr, locations)
					.block_id(at.into())
					.await?;
				let mut storage_proofs = vec![];
				for proof in proof.storage_proof {
					storage_proofs.push(StorageProof::new(
						proof.proof.into_iter().map(|bytes| bytes.to_vec()),
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
						.map(|bytes| bytes.to_vec())
						.collect(),
					storage_proof: map,
				};
				state_proof.encode()
			},
			StateProofQueryType::Arbitrary(keys) => {
				let mut contract_proofs: Vec<_> = vec![];
				let mut map: BTreeMap<Vec<u8>, Vec<Vec<u8>>> = BTreeMap::new();
				let mut contract_addresses_to_keys = BTreeMap::new();

				for key in keys {
					if key.len() != 20 && key.len() != 52 {
						Err(anyhow!("All arbitrary keys must have a length of 52 bytes or 20 bytes when querying state proofs, found key with length {}", key.len()))?
					}

					let contract_address = H160::from_slice(&key[..20]);
					let entry =
						contract_addresses_to_keys.entry(contract_address).or_insert(vec![]);

					if key.len() == 52 {
						let slot_hash = H256::from_slice(&key[20..]);
						entry.push(slot_hash)
					}
				}

				for (contract_address, slot_hashes) in contract_addresses_to_keys {
					let addr = Address::from_slice(&contract_address.0);
					let slots: Vec<B256> = slot_hashes.into_iter().map(|slot| B256::from_slice(&slot.0)).collect();
					let proof = self
						.client
						.get_proof(addr, slots)
						.block_id(at.into())
						.await?;
					contract_proofs.push(StorageProof::new(
						proof.account_proof.into_iter().map(|node| node.to_vec()),
					));

					if !proof.storage_proof.is_empty() {
						let storage_proofs = proof.storage_proof.into_iter().map(|storage_proof| {
							StorageProof::new(
								storage_proof.proof.into_iter().map(|bytes| bytes.to_vec()),
							)
						});

						map.insert(
							contract_address.0.to_vec(),
							StorageProof::merge(storage_proofs).into_nodes().into_iter().collect(),
						);
					}
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

	async fn query_request_receipt(&self, hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let host_contract = EvmHostInstance::new(host_addr, self.signer.clone());
		let address = host_contract.requestReceipts(B256::from_slice(&hash.0)).call().await?;
		Ok(address.to_vec())
	}

	async fn query_response_receipt(&self, hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let host_contract = EvmHostInstance::new(host_addr, self.signer.clone());
		let address = host_contract.responseReceipts(B256::from_slice(&hash.0)).call().await?.relayer;
		Ok(address.to_vec())
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

	/// Returns gas estimate for message execution and it's value in USD.
	/// Uses debug_traceCall to verify that the message would actually be handled successfully.
	async fn estimate_gas(
		&self,
		msg: Vec<Message>,
	) -> Result<Vec<EstimateGasReturnParams>, Error> {
		use crate::tx::estimate_gas_for_messages;
		use alloy::{
			eips::BlockId,
			providers::ext::DebugApi,
			rpc::types::{
				trace::geth::{
					CallConfig, GethDebugBuiltInTracerType, GethDebugTracerConfig,
					GethDebugTracerType, GethDebugTracingCallOptions, GethDebugTracingOptions,
					GethDefaultTracingOptions, GethTrace,
				},
				TransactionRequest,
			},
		};
		use crate::gas_oracle::is_orbit_chain;

		let estimates = estimate_gas_for_messages(self, msg.clone()).await?;

		// Setup debug trace call options with the call tracer
		let call_config = CallConfig { only_top_call: Some(false), with_log: Some(true) };
		let debug_trace_call_options = GethDebugTracingCallOptions {
			tracing_options: GethDebugTracingOptions {
				config: GethDefaultTracingOptions {
					disable_storage: Some(true),
					enable_memory: Some(false),
					..Default::default()
				},
				tracer: Some(GethDebugTracerType::BuiltInTracer(
					GethDebugBuiltInTracerType::CallTracer,
				)),
				tracer_config: GethDebugTracerConfig(serde_json::to_value(
					call_config,
				)?),
				..Default::default()
			},
			..Default::default()
		};

		let handler = self.handler().await?;
		let handler_addr = Address::from_slice(&handler.0);
		let from_address = Address::from_slice(&self.address);

		// For erigon clients, we need to set gas price even when tracing
		let trace_gas_price = if self.client_type.erigon() {
			Some(
				get_current_gas_cost_in_usd(
					self.state_machine,
					self.config.ismp_host.0.into(),
					self.client.clone(),
				)
				.await?
				.gas_price
				.low_u128(),
			)
		} else {
			None
		};

		let gas_breakdown = get_current_gas_cost_in_usd(
			self.state_machine,
			self.config.ismp_host.0.into(),
			self.client.clone(),
		)
		.await?;
		let mut gas_estimates = vec![];
		let batch_size = self.config.tracing_batch_size.unwrap_or(10);

		for (estimates_batch, msgs) in estimates.chunks(batch_size).zip(msg.chunks(batch_size)) {
			let processes = estimates_batch
				.iter()
				.zip(msgs)
				.map(|(estimate, _msg)| {
					let client = self.clone();
					let debug_trace_call_options = debug_trace_call_options.clone();
					let gas_breakdown_unit_wei_cost = gas_breakdown.unit_wei_cost;
					let gas_breakdown_gas_price_cost = gas_breakdown.gas_price_cost;
					let calldata = estimate.calldata.clone();
					let _msg = _msg.clone();
					tokio::spawn(async move {
						let mut tx = TransactionRequest::default()
							.from(from_address)
							.to(handler_addr)
							.input(alloy::primitives::Bytes::from(calldata.clone()).into());

						if let Some(gas_price) = trace_gas_price {
							tx = tx.gas_price(gas_price);
						}

						let call_debug = client
							.client
							.debug_trace_call(
								tx,
								BlockId::latest(),
								debug_trace_call_options,
							)
							.await;

						let mut gas_to_be_used = U256::zero();
						let mut successful_execution = false;

						match call_debug {
							Ok(GethTrace::CallTracer(call_frame)) => {
								match _msg {
									Message::Request(_) => {
										successful_execution = check_trace_for_event(
											&call_frame,
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
											&call_frame,
											CheckTraceForEventParams::Response,
										);
										if !successful_execution {
											log::trace!(
												"debug_traceCall response message failed on {:?}",
												client.state_machine
											);
										}
									},
									_ => unreachable!("Only request/responses are estimated"),
								};

								if successful_execution &&
									is_orbit_chain(client.chain_id as u32)
								{
									let estimate_tx = TransactionRequest::default()
										.from(from_address)
										.to(handler_addr)
										.input(
											alloy::primitives::Bytes::from(calldata.clone())
												.into(),
										);
									let estimated_gas =
										client.client.estimate_gas(estimate_tx).await?;
									gas_to_be_used = U256::from(estimated_gas);
								} else {
									gas_to_be_used = alloy_u256_to_primitive(call_frame.gas_used);
								}
							},
							Ok(trace) => {
								log::error!(
									"Unexpected geth trace variant: {trace:?}"
								);
							},
							Err(err) => {
								log::error!(
									"debug_traceCall failed on {:?}: {err:?}",
									client.state_machine
								);
							},
						};

						let gas_cost_for_data_in_usd = match client.state_machine {
							StateMachine::Evm(_) => {
								use alloy::consensus::TxLegacy;
								use alloy::primitives::TxKind;
								use alloy_rlp::Encodable;

								let unsigned_tx = TxLegacy {
									to: TxKind::Call(handler_addr),
									input: alloy::primitives::Bytes::from(calldata),
									..Default::default()
								};
								let mut rlp_buf = Vec::new();
								unsigned_tx.encode(&mut rlp_buf);

								get_l2_data_cost(
									rlp_buf.into(),
									client.state_machine,
									client.client.clone(),
									gas_breakdown_unit_wei_cost,
								)
								.await?
							},
							_ => U256::zero().into(),
						};

						let execution_cost = (gas_breakdown_gas_price_cost * gas_to_be_used) +
							gas_cost_for_data_in_usd;
						Ok::<_, Error>(EstimateGasReturnParams {
							execution_cost,
							successful_execution,
						})
					})
				})
				.collect::<FuturesOrdered<_>>();

			use futures::StreamExt;
			let estimates_result: Vec<_> = processes.collect().await;
			let estimates_result = estimates_result
				.into_iter()
				.map(|r| r.map_err(|e| anyhow!("Join error: {e:?}"))?)
				.collect::<Result<Vec<_>, Error>>()?;

			gas_estimates.extend(estimates_result);
		}

		Ok(gas_estimates)
	}

	async fn query_request_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let host_contract = EvmHostInstance::new(host_addr, self.signer.clone());
		let fee_metadata = host_contract.requestCommitments(B256::from_slice(&hash.0)).call().await?;
		// erc20 tokens are formatted in 18 decimals
		return Ok(alloy_u256_to_primitive(fee_metadata.fee));
	}

	async fn query_response_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let host_contract = EvmHostInstance::new(host_addr, self.signer.clone());
		let fee_metadata = host_contract.responseCommitments(B256::from_slice(&hash.0)).call().await?;
		// erc20 tokens are formatted in 18 decimals
		return Ok(alloy_u256_to_primitive(fee_metadata.fee));
	}

	async fn state_commitment_vetoed_notification(
		&self,
		from: u64,
		update_height: StateMachineHeight,
	) -> BoxStream<StateCommitmentVetoed> {
		let (tx, recv) = tokio::sync::mpsc::channel(32);
		let client = self.clone();
		let poll_interval = 10;
		tokio::spawn(async move {
			let mut latest_height = from;
			let state_machine = client.state_machine;
			loop {
				// If receiver has been dropped kill the task
				if tx.is_closed() {
					return
				}
				tokio::time::sleep(Duration::from_secs(poll_interval)).await;
				// wait for an update with a greater height
				let block_number = match client.client.get_block_number().await {
					Ok(number) => number,
					Err(err) => {
						if let Err(err) = tx
							.send(Err(anyhow!(
								"Error fetching latest block height on {state_machine:?} {err:?}"
							).into()))
							.await
						{
							log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
							return
						}
						continue;
					},
				};

				if block_number <= latest_height {
					continue;
				}

				let event = StateMachineUpdated {
					state_machine_id: client.state_machine_id(),
					latest_height: block_number,
				};

				let events = match client.query_ismp_events(latest_height, event).await {
					Ok(events) => events,
					Err(err) => {
						if let Err(err) = tx
							.send(Err(anyhow!(
								"Error encountered while querying ismp events {err:?}"
							).into()))
							.await
						{
							log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
							return
						}
						latest_height = block_number;
						continue;
					},
				};

				let event = events
					.into_iter()
					.find_map(|ev| match ev {
						Event::StateCommitmentVetoed(update) if update.height == update_height => Some(update),
						_ => None,
					});

				if let Some(event) = event {
					if let Err(err) = tx.send(Ok(event.clone())).await {
						log::trace!(target: "tesseract", "Failed to send state commitment vetoed event over channel on {state_machine:?}->{:?} \n {err:?}", update_height.id.state_id);
						return
					};
				}
				latest_height = block_number;
			}
		}.boxed());

		Box::pin(tokio_stream::wrappers::ReceiverStream::new(recv))
	}

	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, Error> {
		use futures::StreamExt;
		let mut mutex = self.state_machine_update_sender.lock().await;
		let is_empty = mutex.is_none();
		let (tx, recv) = if is_empty {
			let (tx_og, recv) = tokio::sync::broadcast::channel(512);
			*mutex = Some(tx_og.clone());
			(tx_og, recv)
		} else {
			let tx = mutex.as_ref().expect("Not empty").clone();
			let recv = tx.subscribe();
			(tx, recv)
		};

		if is_empty {
			let initial_height = self.client.get_block_number().await?;
			let client = self.clone();
			let poll_interval = self.config.poll_interval.unwrap_or(10);

			tokio::spawn(async move {
				let mut latest_height = initial_height;
				let state_machine = client.state_machine;
				loop {
					tokio::time::sleep(Duration::from_secs(poll_interval)).await;
					// wait for an update with a greater height
					let block_number = match client.client.get_block_number().await {
						Ok(number) => number,
						Err(err) => {
							if let Err(err) = tx
								.send(Err(anyhow!(
									"Error fetching latest block height on {state_machine:?} {err:?}"
								).into()))
							{
								log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
								return
							}
							continue;
						},
					};

					if block_number <= latest_height {
						continue;
					}

					let event = StateMachineUpdated {
						state_machine_id: client.state_machine_id(),
						latest_height: block_number,
					};

					let events = match client.query_ismp_events(latest_height, event).await {
						Ok(events) => events,
						Err(err) => {
							if let Err(err) = tx
								.send(Err(anyhow!(
									"Error encountered while querying ismp events {err:?}"
								).into()))
							{
								log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
								return
							}
							latest_height = block_number;
							continue;
						},
					};

					let event = events
						.into_iter()
						.filter_map(|ev| match ev {
							Event::StateMachineUpdated(update) => Some(update),
							_ => None,
						})
						.max_by(|a, b| a.latest_height.cmp(&b.latest_height));

					if let Some(event) = event {
						// We wait for the challenge period and see if the update will be vetoed before yielding
						let commitment_height = StateMachineHeight { id: counterparty_state_id, height: event.latest_height };
						let state_machine_update_time = match client.query_state_machine_update_time(commitment_height).await {
							Ok(val) => val,
							Err(err) => {
								if let Err(err) = tx
									.send(Err(anyhow!(
										"Error encountered while querying state_machine_update_time {err:?}"
									).into()))
								{
									log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
									return
								}
								latest_height = block_number;
								continue;
							}
						};

						let mut state_commitment_vetoed_stream = client.state_commitment_vetoed_notification(latest_height, commitment_height).await;
						let provider = Arc::new(client.clone());
						// Yield if the challenge period elapses and the state commitment is not vetoed
						tokio::select! {
							_res = wait_for_challenge_period(provider, state_machine_update_time, counterparty_state_id) => {
								match _res {
									Ok(_) => {
										if let Err(err) = tx.send(Ok(event.clone())) {
											log::trace!(target: "tesseract", "Failed to send state machine update over channel on {state_machine:?} - {:?} \n {err:?}", counterparty_state_id.state_id);
											return
										};
									}
									Err(err) => {
										log::error!(target: "tesseract", "Error waiting for challenge period in {state_machine:?} - {:?} update stream \n {err:?}", counterparty_state_id.state_id);
									}
								}
							}

							_res = state_commitment_vetoed_stream.next() => {
								match _res {
									Some(Ok(_)) => {
										log::error!(target: "tesseract", "State Commitment for {event:?} was vetoed on {state_machine}");
									}

									_ => {
										log::error!(target: "tesseract", "Error in state machine vetoed stream {state_machine:?} - {:?}", counterparty_state_id.state_id);
									}
								}
							}
						};
					}
					latest_height = block_number;
				}
			}.boxed());
		}

		let stream = tokio_stream::wrappers::BroadcastStream::new(recv).filter_map(|res| async {
			match res {
				Ok(res) => Some(res),
				Err(err) => Some(Err(anyhow!("{err:?}").into())),
			}
		});

		Ok(Box::pin(stream))
	}

	async fn submit(
		&self,
		messages: Vec<Message>,
		_coprocessor: StateMachine,
	) -> Result<TxResult, Error> {
		let queue = self
			.queue
			.as_ref()
			.ok_or_else(|| anyhow!("Transaction submission pipeline was not initialized"))?
			.clone();
		queue.send(messages).await?
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
		use alloy::signers::SignerSync;
		let hash = B256::from_slice(msg);
		let signature = self
			.private_key_signer
			.sign_hash_sync(&hash)
			.expect("Infallible")
			.as_bytes()
			.to_vec();
		Signature::Evm { address: self.address.clone(), signature }
	}

	async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		self.set_latest_finalized_height(counterparty).await
	}

	async fn set_initial_consensus_state(
		&self,
		mut message: CreateConsensusState,
	) -> Result<(), Error> {
		let (id, StateCommitmentHeight { commitment, height }) =
			message.state_machine_commitments.remove(0);
		let height = StateMachineHeight { id, height };
		self.set_consensus_state(message.consensus_state, height.try_into()?, commitment.into())
			.await?;
		Ok(())
	}

	async fn veto_state_commitment(&self, _height: StateMachineHeight) -> Result<(), Error> {
		// let contract = EvmHost::new(self.config.ismp_host, self.client.clone());
		// if let Some(_) = contract
		// 	.veto_state_commitment(ismp_solidity_abi::beefy::StateMachineHeight {
		// 		state_machine_id: match height.id.state_id {
		// 			StateMachine::Kusama(id) | StateMachine::Polkadot(id) => id.into(),
		// 			_ => Err(anyhow!("Unexpected State machine"))?,
		// 		},
		// 		height: height.height.into(),
		// 	})
		// 	.send()
		// 	.await?
		// 	.await?
		// {
		// 	log::info!("Frozen consensus client on {:?}", self.state_machine);
		// }
		Ok(())
	}

	async fn query_host_params(
		&self,
		_state_machine: StateMachine,
	) -> Result<HostParam<u128>, anyhow::Error> {
		let host_addr = Address::from_slice(&self.config.ismp_host.0);
		let contract = EvmHostInstance::new(host_addr, self.client.clone());
		let params = contract.hostParams().call().await?;
		let evm_params = EvmHostParam {
			default_timeout: params.defaultTimeout.try_into().unwrap_or(0),
			default_per_byte_fee: alloy_u256_to_primitive(params.defaultPerByteFee),
			state_commitment_fee: alloy_u256_to_primitive(params.stateCommitmentFee),
			fee_token: H160::from_slice(params.feeToken.as_slice()),
			admin: H160::from_slice(params.admin.as_slice()),
			handler: H160::from_slice(params.handler.as_slice()),
			host_manager: H160::from_slice(params.hostManager.as_slice()),
			uniswap_v2: H160::from_slice(params.uniswapV2.as_slice()),
			un_staking_period: params.unStakingPeriod.try_into().unwrap_or(0),
			challenge_period: params.challengePeriod.try_into().unwrap_or(0),
			consensus_client: H160::from_slice(params.consensusClient.as_slice()),
			state_machines: params
				.stateMachines
				.into_iter()
				.map(|id| id.try_into().unwrap_or(0))
				.collect::<Vec<_>>()
				.try_into()
				.map_err(|_| anyhow!("Failed to convert bounded vec"))?,
			per_byte_fees: params
				.perByteFees
				.into_iter()
				.map(|p| PerByteFee {
					per_byte_fee: alloy_u256_to_primitive(p.perByteFee),
					state_id: H256::from_slice(p.stateIdHash.as_slice()),
				})
				.collect::<Vec<_>>()
				.try_into()
				.map_err(|_| anyhow!("Failed to convert bounded vec"))?,
			hyperbridge: params
				.hyperbridge
				.to_vec()
				.try_into()
				.map_err(|_| anyhow!("Failed to convert bounded vec"))?,
		};
		Ok(HostParam::EvmHostParam(evm_params))
	}

	fn max_concurrent_queries(&self) -> usize {
		self.config.tracing_batch_size.unwrap_or(10)
	}

	async fn fee_token_decimals(&self) -> Result<u8, anyhow::Error> {
		let fee_token = match self.query_host_params(self.state_machine).await? {
			HostParam::EvmHostParam(params) => params.fee_token,
			_ => Err(anyhow!("Unexpected host params"))?,
		};

		let fee_token_addr = Address::from_slice(&fee_token.0);
		let contract = Erc20Instance::new(fee_token_addr, self.client.clone());

		let decimals = contract.decimals().call().await?;

		Ok(decimals)
	}
}

pub enum CheckTraceForEventParams {
	Request,
	Response,
}

pub fn check_trace_for_event(
	call_frame: &alloy::rpc::types::trace::geth::CallFrame,
	event_in: CheckTraceForEventParams,
) -> bool {
	use alloy::primitives::LogData;

	if let Some(ref error) = call_frame.revert_reason {
		log::error!("Error in main call frame: {error}");
	}

	// Check the last inner call frame's logs for the expected event
	if let Some(last_call_frame) = call_frame.calls.last() {
		if let Some(ref error) = last_call_frame.error {
			log::error!("Error in inner call frame: {error}");
		}

		for log in &last_call_frame.logs {
			let topics = log.topics.clone().unwrap_or_default();
			let data = log.data.clone().unwrap_or_default();
			if let Some(log_data) = LogData::new(topics, data) {
				let prim_log = alloy::primitives::Log {
					address: log.address.unwrap_or_default(),
					data: log_data,
				};

				match event_in {
					CheckTraceForEventParams::Request => {
						match PostRequestHandled::decode_log(&prim_log) {
							Ok(_) => return true,
							Err(err) => {
								log::error!(
									"Failed to parse {:?} trace log: {err:?}",
									last_call_frame.to
								);
							},
						}
					},
					CheckTraceForEventParams::Response => {
						match PostResponseHandled::decode_log(&prim_log) {
							Ok(_) => return true,
							Err(err) => {
								log::error!(
									"Failed to parse {:?} trace log: {err:?}",
									last_call_frame.to
								);
							},
						}
					},
				}
			}
		}
	} else {
		log::error!("Debug trace frame not found!");
	}

	false
}
