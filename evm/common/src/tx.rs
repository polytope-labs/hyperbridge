use crate::EvmClient;
use anyhow::{anyhow, Error};
use codec::Decode;
use ethers::{
	contract::{parse_log, FunctionCall},
	core::k256::ecdsa,
	middleware::SignerMiddleware,
	prelude::{
		signer::SignerMiddlewareError, transaction::eip2718::TypedTransaction, ContractError, Log,
		NameOrAddress, Provider, ProviderError, Wallet,
	},
	providers::{Http, Middleware, PendingTransaction},
	types::{TransactionReceipt, TransactionRequest},
};
use ismp::{
	host::{Ethereum, StateMachine},
	messaging::{Message, ResponseMessage, TimeoutMessage},
	router::{Request, RequestResponse, Response},
};
use ismp_rpc::MmrProof;
use ismp_solidity_abi::{
	beefy::{GetRequest, StateMachineHeight},
	evm_host::{PostRequestHandledFilter, PostResponseHandledFilter},
	handler::{
		GetResponseMessage, GetTimeoutMessage, Handler as IsmpHandler, PostRequestLeaf,
		PostRequestMessage, PostRequestTimeoutMessage, PostResponseLeaf, PostResponseMessage,
		PostResponseTimeoutMessage, Proof,
	},
};
use mmr_utils::mmr_position_to_k_index;
use pallet_ismp::{
	primitives::{LeafIndexAndPos, SubstrateStateProof},
	NodesUtils,
};
use primitive_types::{H160, H256, U256};
use std::{collections::BTreeSet, sync::Arc, time::Duration};

use crate::gas_oracle::get_current_gas_cost_in_usd;
use tesseract_primitives::IsmpHost;

/// Type alias
type SolidityFunctionCall = FunctionCall<
	Arc<SignerMiddleware<Provider<Http>, Wallet<ecdsa::SigningKey>>>,
	SignerMiddleware<Provider<Http>, Wallet<ecdsa::SigningKey>>,
	(),
>;

#[async_recursion::async_recursion]
pub async fn submit_messages<I: IsmpHost>(
	client: &EvmClient<I>,
	messages: Vec<Message>,
) -> anyhow::Result<BTreeSet<H256>> {
	let calls = generate_contract_calls(client, messages.clone()).await?;
	let mut events = BTreeSet::new();
	for (index, call) in calls.into_iter().enumerate() {
		let gas_price = call.tx.gas_price();
		match call.clone().send().await {
			Ok(progress) => {
				let retry = if matches!(messages[index], Message::Consensus(_)) {
					Some(call)
				} else {
					None
				};
				let evs = wait_for_success(client, progress, gas_price, retry).await?;
				events.extend(evs);
			},
			Err(err) => {
				match err {
					ContractError::MiddlewareError {
						e:
							SignerMiddlewareError::MiddlewareError(ProviderError::JsonRpcClientError(
								ref error,
							)),
					} => {
						if let Some(err) = error.as_error_response() {
							// https://docs.alchemy.com/reference/error-reference#http-status-codes
							if err.code == 429 {
								// we should retry.
								log::info!("Retrying tx submission, got error: {err:?}");
								return submit_messages(&client, messages).await
							}
						}
					},
					_ => {},
				}

				log::error!(
					"Error broadcasting transaction to {}:  {err:?}",
					client.config.state_machine
				);
			},
		}
	}

	if !events.is_empty() {
		log::trace!("Got {} receipts from executing on {:?}", events.len(), client.state_machine);
	}

	Ok(events)
}

#[async_recursion::async_recursion]
async fn wait_for_success<'a, I>(
	client: &EvmClient<I>,
	tx: PendingTransaction<'a, Http>,
	gas_price: Option<U256>,
	retry: Option<SolidityFunctionCall>,
) -> Result<BTreeSet<H256>, anyhow::Error>
where
	'a: 'async_recursion,
	I: IsmpHost,
{
	let log_receipt = |receipt: TransactionReceipt, cancelled: bool| {
		let prelude = if cancelled { "Cancellation Tx" } else { "Tx" };
		if matches!(receipt.status.as_ref().map(|f| f.low_u64()), Some(1)) {
			log::info!("{prelude} for {:?} succeeded", client.state_machine);
		} else {
			log::info!(
				"{prelude} for {:?} with hash {:?} reverted",
				client.state_machine,
				receipt.transaction_hash
			);
		}
	};

	let client_clone = client.clone();

	let handle_failed_tx = move || async move {
		log::info!("No receipt for transaction on {:?}", client_clone.state_machine);

		if let Some(call) = retry {
			// lets retry
			let gas_price = get_current_gas_cost_in_usd(
				client.chain_id,
				client.state_machine,
				&client.config.etherscan_api_key.clone(),
				client.client.clone(),
			)
			.await?
			.gas_price * 2; // for good measure
			log::info!(
				"Retrying consensus message on {:?} with gas {}",
				client_clone.state_machine,
				ethers::utils::format_units(gas_price, "gwei")?
			);
			let call = call.gas_price(gas_price);
			let pending = call.send().await?;

			// don't retry in the next callstack
			wait_for_success(client, pending, Some(gas_price), None).await
		} else {
			// cancel the transaction here
			let pending = client_clone
				.signer
				.send_transaction(
					TypedTransaction::Legacy(TransactionRequest {
						to: Some(NameOrAddress::Address(H160::from_slice(&client_clone.address))),
						value: Some(Default::default()),
						gas_price: gas_price.map(|price| price * 10), // experiment with higher?
						..Default::default()
					}),
					None,
				)
				.await;

			if let Ok(pending) = pending {
				if let Ok(Some(receipt)) = pending.await {
					log_receipt(receipt, true);
				}
			}

			Err(anyhow!("Transaction to {:?} was cancelled!", client.state_machine))?
		}
	};

	// Race transaction submission by a five minute timer
	let sleep = tokio::time::sleep(Duration::from_secs(5 * 60));

	tokio::select! {
		_ = sleep => {
			return handle_failed_tx().await;
		},
		result = tx => {
			match result {
				Ok(Some(receipt)) => {
					let events =  receipt.logs.iter().filter_map(|l| {
						let log = Log {
							topics: l.clone().topics,
							data: l.clone().data,
							..Default::default()
						};
						if let Some(ev) = parse_log::<PostRequestHandledFilter>(log.clone()).ok() {
							return Some(ev.commitment.into())
						}
						if let Some(ev) = parse_log::<PostResponseHandledFilter>(log.clone()).ok() {
							return Some(ev.commitment.into())
						}
						None
					}).collect();
					log_receipt(receipt, false);
					Ok(events)
				},
				Ok(None) => {
					return handle_failed_tx().await;
				},
				Err(err) => {
					log::error!("Error broadcasting transaction to {:?}: {err:?}", client.state_machine);
					Err(err)?
				},
			}
		}
	}
}

/// Function generates FunctionCall(s) from a batchs of messages
pub async fn generate_contract_calls<I: IsmpHost>(
	client: &EvmClient<I>,
	messages: Vec<Message>,
) -> anyhow::Result<Vec<SolidityFunctionCall>> {
	let contract = IsmpHandler::new(client.config.handler, client.signer.clone());
	let ismp_host = client.config.ismp_host;
	let mut calls = Vec::new();
	let gas_price = get_current_gas_cost_in_usd(
		client.chain_id,
		client.state_machine,
		&client.config.etherscan_api_key.clone(),
		client.client.clone(),
	)
	.await?
	.gas_price;

	for message in messages {
		match message {
			Message::Consensus(msg) => {
				let call = contract.handle_consensus(ismp_host, msg.consensus_proof.into());
				let gas_limit = call
					.estimate_gas()
					.await
					.unwrap_or(get_chain_gas_limit(client.state_machine).into());
				let call = call.gas_price(gas_price).gas(gas_limit);

				calls.push(call);
			},
			Message::Request(msg) => {
				let membership_proof = MmrProof::<H256>::decode(&mut msg.proof.proof.as_slice())?;
				let mmr_size = NodesUtils::new(membership_proof.leaf_count).size();
				let k_and_leaf_indices = membership_proof
					.leaf_positions_and_indices
					.into_iter()
					.map(|LeafIndexAndPos { pos, leaf_index }| {
						let k_index = mmr_position_to_k_index(vec![pos], mmr_size)[0].1;
						(k_index, leaf_index)
					})
					.collect::<Vec<_>>();

				let mut leaves = msg
					.requests
					.into_iter()
					.zip(k_and_leaf_indices)
					.map(|(post, (k_index, leaf_index))| PostRequestLeaf {
						request: post.into(),
						index: leaf_index.into(),
						k_index: k_index.into(),
					})
					.collect::<Vec<_>>();
				leaves.sort_by_key(|leaf| leaf.index);
				let gas_limit = get_chain_gas_limit(client.state_machine);
				let post_message = PostRequestMessage {
					proof: Proof {
						height: StateMachineHeight {
							state_machine_id: {
								match msg.proof.height.id.state_id {
									StateMachine::Polkadot(id) | StateMachine::Kusama(id) =>
										id.into(),
									_ => {
										panic!("Expected polkadot or kusama state machines");
									},
								}
							},
							height: msg.proof.height.height.into(),
						},
						multiproof: membership_proof.items.into_iter().map(|node| node.0).collect(),
						leaf_count: membership_proof.leaf_count.into(),
					},
					requests: leaves,
				};

				let call = contract
					.handle_post_requests(ismp_host, post_message)
					.gas_price(gas_price)
					.gas(gas_limit);

				calls.push(call);
			},
			Message::Response(ResponseMessage { datagram, proof, .. }) => {
				let membership_proof = MmrProof::<H256>::decode(&mut proof.proof.as_slice())?;
				let mmr_size = NodesUtils::new(membership_proof.leaf_count).size();
				let k_and_leaf_indices = membership_proof
					.leaf_positions_and_indices
					.into_iter()
					.map(|LeafIndexAndPos { pos, leaf_index }| {
						let k_index = mmr_position_to_k_index(vec![pos], mmr_size)[0].1;
						(k_index, leaf_index)
					})
					.collect::<Vec<_>>();

				let call = match datagram {
					RequestResponse::Response(responses) => {
						let mut leaves = responses
							.into_iter()
							.zip(k_and_leaf_indices)
							.filter_map(|(res, (k_index, leaf_index))| match res {
								Response::Post(res) => Some(PostResponseLeaf {
									response: res.into(),
									index: leaf_index.into(),
									k_index: k_index.into(),
								}),
								_ => None,
							})
							.collect::<Vec<_>>();
						leaves.sort_by_key(|leaf| leaf.index);
						let gas_limit = get_chain_gas_limit(client.state_machine);
						let message =
							PostResponseMessage {
								proof: Proof {
									height: StateMachineHeight {
										state_machine_id: {
											match proof.height.id.state_id {
												StateMachine::Polkadot(id) |
												StateMachine::Kusama(id) => id.into(),
												_ => {
													log::error!("Expected polkadot or kusama state machines");
													continue
												},
											}
										},
										height: proof.height.height.into(),
									},
									multiproof: membership_proof
										.items
										.into_iter()
										.map(|node| node.0)
										.collect(),
									leaf_count: membership_proof.leaf_count.into(),
								},
								responses: leaves,
							};

						contract
							.handle_post_responses(ismp_host, message)
							.gas_price(gas_price)
							.gas(gas_limit)
					},
					RequestResponse::Request(requests) => {
						let requests = match requests
							.into_iter()
							.map(|req| {
								let get = req
									.get_request()
									.map_err(|_| anyhow!("Expected get request"))?;
								Ok(GetRequest {
									source: get.source.to_string().as_bytes().to_vec().into(),
									dest: get.dest.to_string().as_bytes().to_vec().into(),
									nonce: get.nonce,
									from: get.from.into(),
									keys: get.keys.into_iter().map(|key| key.into()).collect(),
									timeout_timestamp: get.timeout_timestamp,
									gaslimit: get.gas_limit.into(),
									height: get.height.into(),
								})
							})
							.collect::<Result<Vec<_>, Error>>()
						{
							Ok(reqs) => reqs,
							Err(err) => {
								log::error!("Failed to error {err:?}");
								continue
							},
						};

						let gas_limit = get_chain_gas_limit(client.state_machine);

						let state_proof: SubstrateStateProof =
							match codec::Decode::decode(&mut proof.proof.as_slice()) {
								Ok(proof) => proof,
								_ => {
									log::error!("Failed to decode membership proof");
									continue
								},
							};
						let message = GetResponseMessage {
							proof: state_proof
								.storage_proof
								.into_iter()
								.map(|key| key.into())
								.collect(),
							height: StateMachineHeight {
								state_machine_id: {
									match proof.height.id.state_id {
										StateMachine::Polkadot(id) | StateMachine::Kusama(id) =>
											id.into(),
										_ => {
											log::error!(
												"Expected polkadot or kusama state machines"
											);
											continue
										},
									}
								},
								height: proof.height.height.into(),
							},
							requests,
						};

						contract
							.handle_get_responses(ismp_host, message)
							.gas_price(gas_price)
							.gas(gas_limit)
					},
				};

				calls.push(call);
			},
			Message::Timeout(TimeoutMessage::Post { timeout_proof, requests }) => {
				let post_requests = requests
					.into_iter()
					.filter_map(|req| match req {
						Request::Post(post) => Some(post.into()),
						Request::Get(_) => None,
					})
					.collect();

				let state_proof: SubstrateStateProof =
					match codec::Decode::decode(&mut timeout_proof.proof.as_slice()) {
						Ok(proof) => proof,
						_ => {
							log::error!("Failed to decode membership proof");
							continue
						},
					};
				let message = PostRequestTimeoutMessage {
					timeouts: post_requests,
					height: StateMachineHeight {
						state_machine_id: {
							match timeout_proof.height.id.state_id {
								StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
								_ => {
									log::error!("Expected polkadot or kusama state machines");
									continue
								},
							}
						},
						height: timeout_proof.height.height.into(),
					},
					proof: state_proof.storage_proof.into_iter().map(|key| key.into()).collect(),
				};
				let gas_limit = get_chain_gas_limit(client.state_machine);
				let call = contract
					.handle_post_request_timeouts(ismp_host, message)
					.gas_price(gas_price)
					.gas(gas_limit);

				calls.push(call);
			},
			Message::Timeout(TimeoutMessage::PostResponse { timeout_proof, responses }) => {
				let post_responses = responses.into_iter().map(|res| res.into()).collect();

				let state_proof: SubstrateStateProof =
					match codec::Decode::decode(&mut timeout_proof.proof.as_slice()) {
						Ok(proof) => proof,
						_ => {
							log::error!("Failed to decode membership proof");
							continue
						},
					};
				let message = PostResponseTimeoutMessage {
					timeouts: post_responses,
					height: StateMachineHeight {
						state_machine_id: {
							match timeout_proof.height.id.state_id {
								StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
								_ => {
									log::error!("Expected polkadot or kusama state machines");
									continue
								},
							}
						},
						height: timeout_proof.height.height.into(),
					},
					proof: state_proof.storage_proof.into_iter().map(|key| key.into()).collect(),
				};
				let gas_limit = get_chain_gas_limit(client.state_machine);
				let call = contract
					.handle_post_response_timeouts(ismp_host, message)
					.gas_price(gas_price)
					.gas(gas_limit);

				calls.push(call);
			},
			Message::Timeout(TimeoutMessage::Get { requests }) => {
				let get_requests = requests
					.into_iter()
					.filter_map(|req| match req {
						Request::Get(get) => Some(GetRequest {
							source: get.source.to_string().as_bytes().to_vec().into(),
							dest: get.dest.to_string().as_bytes().to_vec().into(),
							nonce: get.nonce,
							from: get.from.into(),
							keys: get.keys.into_iter().map(|key| key.into()).collect(),
							timeout_timestamp: get.timeout_timestamp,
							gaslimit: get.gas_limit.into(),
							height: get.height.into(),
						}),
						_ => None,
					})
					.collect();

				let message = GetTimeoutMessage { timeouts: get_requests };
				let gas_limit = get_chain_gas_limit(client.state_machine);
				let call = contract
					.handle_get_request_timeouts(ismp_host, message)
					.gas_price(gas_price)
					.gas(gas_limit);

				calls.push(call);
			},
			Message::FraudProof(_) => {
				log::warn!(target: "tesseract", "Unexpected fraud proof message")
			},
		}
	}

	Ok(calls)
}

pub fn get_chain_gas_limit(state_machine: StateMachine) -> u64 {
	match state_machine {
		StateMachine::Ethereum(Ethereum::ExecutionLayer) => 20_000_000,
		StateMachine::Ethereum(Ethereum::Arbitrum) => 32_000_000,
		StateMachine::Ethereum(Ethereum::Optimism) => 20_000_000,
		StateMachine::Ethereum(Ethereum::Base) => 20_000_000,
		StateMachine::Polygon => 20_000_000,
		StateMachine::Bsc => 20_000_000,
		_ => Default::default(),
	}
}
