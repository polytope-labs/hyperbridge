use crate::{
	abi::{beefy::BeefyConsensusState, erc_20::Erc20, EvmHost},
	gas_oracle::is_orbit_chain,
	state_comitment_key, EvmClient,
};
use anyhow::{anyhow, Error};
use beefy_verifier_primitives::ConsensusState;
use codec::Encode;
use ethers::{
	abi::AbiDecode,
	providers::Middleware,
	types::{CallFrame, GethDebugTracingCallOptions, GethTrace, GethTraceFrame},
};

use evm_state_machine::types::EvmStateProof;
use geth_primitives::new_u256;
use ismp::{
	consensus::{ConsensusStateId, StateMachineId},
	events::{Event, StateCommitmentVetoed},
	messaging::{Message, StateCommitmentHeight},
};
use ismp_solidity_abi::evm_host::{PostRequestHandledFilter, PostResponseHandledFilter};
use pallet_ismp_host_executive::{EvmHostParam, HostParam, PerByteFee};

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

#[async_trait::async_trait]
impl IsmpProvider for EvmClient {
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		_: ConsensusStateId,
	) -> Result<Vec<u8>, Error> {
		let contract = EvmHost::new(self.config.ismp_host.0, self.client.clone());
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
		let contract = EvmHost::new(self.config.ismp_host.0, self.client.clone());
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
		let timestamp = {
			let timestamp = self
				.client
				.get_storage_at(
					ethers::types::H160(self.config.ismp_host.0),
					timestamp_key.0.into(),
					None,
				)
				.await?;
			U256::from_big_endian(timestamp.as_bytes()).low_u64()
		};
		let overlay_root = self
			.client
			.get_storage_at(
				ethers::types::H160(self.config.ismp_host.0),
				overlay_key.0.into(),
				None,
			)
			.await?
			.0
			.into();
		let state_root = self
			.client
			.get_storage_at(
				ethers::types::H160(self.config.ismp_host.0),
				state_root_key.0.into(),
				None,
			)
			.await?
			.0
			.into();
		Ok(StateCommitment { timestamp, overlay_root: Some(overlay_root), state_root })
	}

	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, Error> {
		let contract = EvmHost::new(self.config.ismp_host.0, self.client.clone());
		let value =
			contract.state_machine_commitment_update_time(height.try_into()?).call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_challenge_period(&self, _id: StateMachineId) -> Result<Duration, Error> {
		let contract = EvmHost::new(self.config.ismp_host.0, self.client.clone());
		let value = contract.challenge_period().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		let client = Arc::new(self.client.clone());
		let contract = EvmHost::new(self.config.ismp_host.0, client);
		let value = contract.timestamp().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let keys = keys
			.into_iter()
			.map(|query| self.request_commitment_key(query.commitment).1 .0.into())
			.collect();

		let proof = self
			.client
			.get_proof(ethers::types::H160(self.config.ismp_host.0), keys, Some(at.into()))
			.await?;
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

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let keys = keys
			.into_iter()
			.map(|query| self.response_commitment_key(query.commitment).1 .0.into())
			.collect();
		let proof = self
			.client
			.get_proof(ethers::types::H160(self.config.ismp_host.0), keys, Some(at.into()))
			.await?;
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
				let locations = keys.iter().map(|key| H256::from_slice(key).0.into()).collect();
				let proof = self
					.client
					.get_proof(
						ethers::types::H160(self.config.ismp_host.0),
						locations,
						Some(at.into()),
					)
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
					let proof = self
						.client
						.get_proof(
							ethers::types::H160(contract_address.0),
							slot_hashes.into_iter().map(|slot| slot.0.into()).collect(),
							Some(at.into()),
						)
						.await?;
					contract_proofs.push(StorageProof::new(
						proof.account_proof.into_iter().map(|node| node.0.into()),
					));

					if !proof.storage_proof.is_empty() {
						let storage_proofs = proof.storage_proof.into_iter().map(|storage_proof| {
							StorageProof::new(
								storage_proof.proof.into_iter().map(|bytes| bytes.0.into()),
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
		let host_contract = EvmHost::new(self.config.ismp_host.0, self.signer.clone());
		let address = host_contract.request_receipts(hash.into()).call().await?;
		Ok(address.0.to_vec())
	}

	async fn query_response_receipt(&self, hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		let host_contract = EvmHost::new(self.config.ismp_host.0, self.signer.clone());
		let address = host_contract.response_receipts(hash.into()).call().await?.relayer;
		Ok(address.0.to_vec())
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

		// The clients we support(erigon and geth) both use Geth style tracing
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

		let calls = generate_contract_calls(self, messages, true).await?;
		let gas_breakdown = get_current_gas_cost_in_usd(
			self.state_machine,
			&self.config.etherscan_api_key.clone(),
			self.client.clone(),
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
						call.tx.set_from(address.0.into());

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

								if successful_execution && is_orbit_chain(client.chain_id as u32) {
									let _temp_gas =
										client.client.estimate_gas(&call.tx, call.block).await?;
									gas_to_be_used = new_u256(_temp_gas);
								} else {
									gas_to_be_used = new_u256(call_frame.gas_used);
								}
							},
							trace => {
								log::error!("an unknown geth trace was reached {trace:?}")
							},
						};

						let gas_cost_for_data_in_usd = match client.state_machine {
							StateMachine::Evm(_) =>
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
		let host_contract = EvmHost::new(self.config.ismp_host.0, self.signer.clone());
		let fee_metadata = host_contract.request_commitments(hash.into()).call().await?;
		// erc20 tokens are formatted in 18 decimals
		return Ok(new_u256(fee_metadata.fee));
	}

	async fn query_response_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		let host_contract = EvmHost::new(self.config.ismp_host.0, self.signer.clone());
		let fee_metadata = host_contract.response_commitments(hash.into()).call().await?;
		// erc20 tokens are formatted in 18 decimals
		return Ok(new_u256(fee_metadata.fee));
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
					Ok(number) => number.low_u64(),
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
			let initial_height = self.client.get_block_number().await?.low_u64();
			let client = self.clone();
			let poll_interval = self.config.poll_interval.unwrap_or(10);

			tokio::spawn(async move {
				let mut latest_height = initial_height;
				let state_machine = client.state_machine;
				loop {
					tokio::time::sleep(Duration::from_secs(poll_interval)).await;
					// wait for an update with a greater height
					let block_number = match client.client.get_block_number().await {
						Ok(number) => number.low_u64(),
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
			.ok_or_else(|| anyhow!("Trasnsaction submission pipeline was not initialized"))?
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
		let signature = self
			.signer
			.signer()
			.sign_hash(H256::from_slice(msg).0.into())
			.expect("Infallible")
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
		let contract = EvmHost::new(self.config.ismp_host.0, self.client.clone());
		let params: ismp_solidity_abi::evm_host::HostParams = contract.host_params().call().await?;
		let evm_params = EvmHostParam {
			default_timeout: params.default_timeout.low_u128(),
			default_per_byte_fee: new_u256(params.default_per_byte_fee),
			state_commitment_fee: new_u256(params.state_commitment_fee),
			fee_token: params.fee_token.0.into(),
			admin: params.admin.0.into(),
			handler: params.handler.0.into(),
			host_manager: params.host_manager.0.into(),
			uniswap_v2: params.uniswap_v2.0.into(),
			un_staking_period: params.un_staking_period.low_u128(),
			challenge_period: params.challenge_period.low_u128(),
			consensus_client: params.consensus_client.0.into(),
			state_machines: params
				.state_machines
				.into_iter()
				.map(|id| id.low_u32())
				.collect::<Vec<_>>()
				.try_into()
				.map_err(|_| anyhow!("Failed to convert bounded vec"))?,
			per_byte_fees: params
				.per_byte_fees
				.into_iter()
				.map(|p| PerByteFee {
					per_byte_fee: new_u256(p.per_byte_fee),
					state_id: p.state_id_hash.into(),
				})
				.collect::<Vec<_>>()
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

	async fn fee_token_decimals(&self) -> Result<u8, anyhow::Error> {
		let fee_token = match self.query_host_params(self.state_machine).await? {
			HostParam::EvmHostParam(params) => params.fee_token,
			_ => Err(anyhow!("Unexpected host params"))?,
		};

		let contract = Erc20::new(fee_token.0, self.client.clone());

		let decimals = contract.decimals().call().await?;

		Ok(decimals)
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
	} else {
		log::error!("Debug trace frame not found!");
	}

	false
}
