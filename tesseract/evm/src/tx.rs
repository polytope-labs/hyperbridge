use crate::EvmClient;
use anyhow::anyhow;
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
	messaging::{Message, ResponseMessage},
	router::{RequestResponse, Response},
};
use ismp_solidity_abi::{
	beefy::StateMachineHeight,
	evm_host::{PostRequestHandledFilter, PostResponseHandledFilter},
	handler::{
		Handler as IsmpHandler, PostRequestLeaf, PostRequestMessage, PostResponseLeaf,
		PostResponseMessage, Proof,
	},
};
use mmr_primitives::mmr_position_to_k_index;
use pallet_ismp::mmr::{LeafIndexAndPos, Proof as MmrProof};
use primitive_types::{H160, H256, U256};
use sp_mmr_primitives::utils::NodesUtils;
use std::{collections::BTreeSet, sync::Arc, time::Duration};

use crate::gas_oracle::get_current_gas_cost_in_usd;

/// Type alias
type SolidityFunctionCall = FunctionCall<
	Arc<SignerMiddleware<Provider<Http>, Wallet<ecdsa::SigningKey>>>,
	SignerMiddleware<Provider<Http>, Wallet<ecdsa::SigningKey>>,
	(),
>;

#[async_recursion::async_recursion]
pub async fn submit_messages(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<BTreeSet<H256>> {
	let calls = generate_contract_calls(client, messages.clone(), false).await?;
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
								return submit_messages(&client, messages).await;
							}
						}
					},
					_ => {},
				}

				Err(err)?
			},
		}
	}

	if !events.is_empty() {
		log::trace!("Got {} receipts from executing on {:?}", events.len(), client.state_machine);
	}

	Ok(events)
}

#[async_recursion::async_recursion]
async fn wait_for_success<'a>(
	client: &EvmClient,
	tx: PendingTransaction<'a, Http>,
	gas_price: Option<U256>,
	retry: Option<SolidityFunctionCall>,
) -> Result<BTreeSet<H256>, anyhow::Error>
where
	'a: 'async_recursion,
{
	let log_receipt = |receipt: TransactionReceipt, cancelled: bool| -> Result<(), anyhow::Error> {
		let prelude = if cancelled { "Cancellation Tx" } else { "Tx" };
		if matches!(receipt.status.as_ref().map(|f| f.low_u64()), Some(1)) {
			log::info!("{prelude} for {:?} succeeded", client.state_machine);
		} else {
			log::info!(
				"{prelude} for {:?} with hash {:?} reverted",
				client.state_machine,
				receipt.transaction_hash
			);
			Err(anyhow!("Transaction reverted"))?
		}

		Ok(())
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
				client.config.gas_price_buffer,
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
					// we're going to error anyways
					let _ = log_receipt(receipt, true);
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
					log_receipt(receipt, false)?;
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
/// If `debug_trace` is true then the gas_price will not be set on the generated call
pub async fn generate_contract_calls(
	client: &EvmClient,
	messages: Vec<Message>,
	debug_trace: bool,
) -> anyhow::Result<Vec<SolidityFunctionCall>> {
	let handler = client.handler().await?;
	let contract = IsmpHandler::new(handler, client.signer.clone());
	let ismp_host = client.config.ismp_host;
	let mut calls = Vec::new();
	// If debug trace is false or the client type is erigon, then the gas price must be set
	// Geth does not require gas price to be set when debug tracing, but the erigon implementation
	// does https://github.com/ledgerwatch/erigon/blob/cfb55a3cd44736ac092003be41659cc89061d1be/core/state_transition.go#L246
	// Erigon does not support block overrides when tracing so we don't have the option of omiting
	// the gas price by overriding the base fee
	let set_gas_price = || !debug_trace || client.client_type.erigon();
	let gas_price = if set_gas_price() {
		get_current_gas_cost_in_usd(
			client.chain_id,
			client.state_machine,
			&client.config.etherscan_api_key.clone(),
			client.client.clone(),
			client.config.gas_price_buffer,
		)
		.await?
		.gas_price
	} else {
		Default::default()
	};

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
					.leaf_indices_and_pos
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

				let call = if set_gas_price() {
					contract
						.handle_post_requests(ismp_host, post_message)
						.gas_price(gas_price)
						.gas(gas_limit)
				} else {
					contract.handle_post_requests(ismp_host, post_message).gas(gas_limit)
				};
				calls.push(call)
			},
			Message::Response(ResponseMessage { datagram, proof, .. }) => {
				let membership_proof = MmrProof::<H256>::decode(&mut proof.proof.as_slice())?;
				let mmr_size = NodesUtils::new(membership_proof.leaf_count).size();
				let k_and_leaf_indices = membership_proof
					.leaf_indices_and_pos
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
													continue;
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

						if set_gas_price() {
							contract
								.handle_post_responses(ismp_host, message)
								.gas_price(gas_price)
								.gas(gas_limit)
						} else {
							contract.handle_post_responses(ismp_host, message).gas(gas_limit)
						}
					},
					RequestResponse::Request(..) =>
						Err(anyhow!("Get requests are not supported by relayer"))?,
				};

				calls.push(call);
			},
			Message::Timeout(_) => Err(anyhow!("Timeout messages not supported by relayer"))?,
			Message::FraudProof(_) => Err(anyhow!("Unexpected fraud proof message"))?,
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
