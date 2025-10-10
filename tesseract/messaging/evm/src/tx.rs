use crate::{
	gas_oracle::{
		ARBITRUM_CHAIN_ID, ARBITRUM_SEPOLIA_CHAIN_ID, CHIADO_CHAIN_ID, CRONOS_CHAIN_ID,
		CRONOS_TESTNET_CHAIN_ID, GNOSIS_CHAIN_ID, INJECTIVE_CHAIN_ID, INJECTIVE_TESTNET_CHAIN_ID,
		SEI_CHAIN_ID, SEI_TESTNET_CHAIN_ID,
	},
	EvmClient,
};
use anyhow::anyhow;
use codec::Decode;
use ethers::{
	abi::Detokenize,
	contract::{parse_log, FunctionCall},
	core::k256::ecdsa::{self, SigningKey},
	middleware::SignerMiddleware,
	prelude::{
		signer::SignerMiddlewareError, transaction::eip2718::TypedTransaction, ContractError, Log,
		NameOrAddress, Provider, ProviderError, Wallet,
	},
	providers::{Http, Middleware, PendingTransaction},
	types::{TransactionReceipt, TransactionRequest},
};
use geth_primitives::{new_u256, old_u256};
use ismp::{
	host::StateMachine,
	messaging::{hash_request, hash_response, Message, ResponseMessage},
	router::{Request, RequestResponse, Response},
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
use pallet_ismp::offchain::{LeafIndexAndPos, Proof as MmrProof};
use polkadot_sdk::sp_mmr_primitives::utils::NodesUtils;
use primitive_types::{H256, U256};
use std::{collections::BTreeSet, sync::Arc, time::Duration};
use tesseract_primitives::{Hasher, Query, TxReceipt, TxResult};

use crate::gas_oracle::get_current_gas_cost_in_usd;

/// Type alias
type SolidityFunctionCall<T> = FunctionCall<
	Arc<SignerMiddleware<Provider<Http>, Wallet<ecdsa::SigningKey>>>,
	SignerMiddleware<Provider<Http>, Wallet<ecdsa::SigningKey>>,
	T,
>;

#[async_recursion::async_recursion]
pub async fn submit_messages(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<(BTreeSet<H256>, Vec<Message>)> {
	let calls = generate_contract_calls(client, messages.clone(), false).await?;
	let mut events = BTreeSet::new();
	let mut cancelled: Vec<Message> = vec![];
	for (index, call) in calls.into_iter().enumerate() {
		// Encode and Decode needed because of ether-rs and polkadot-sdk incompatibility
		let gas_price = call.tx.gas_price().map(|price| new_u256(price));
		match call.clone().send().await {
			Ok(progress) => {
				let retry = if matches!(messages[index], Message::Consensus(_)) {
					Some(call)
				} else {
					None
				};
				let evs = wait_for_success(
					&client.config.state_machine,
					&client.config.etherscan_api_key,
					client.client.clone(),
					client.signer.clone(),
					progress,
					gas_price,
					retry,
					matches!(messages[index], Message::Consensus(_)),
				)
				.await?;
				if matches!(messages[index], Message::Request(_) | Message::Response(_)) &&
					evs.is_empty()
				{
					cancelled.push(messages[index].clone())
				}
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

	Ok((events, cancelled))
}

#[async_recursion::async_recursion]
pub async fn wait_for_success<'a, T>(
	state_machine: &StateMachine,
	etherscan_api_key: &String,
	provider: Arc<Provider<Http>>,
	signer: Arc<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>,
	tx: PendingTransaction<'a, Http>,
	gas_price: Option<U256>,
	retry: Option<SolidityFunctionCall<T>>,
	is_consensus: bool,
) -> Result<BTreeSet<H256>, anyhow::Error>
where
	'a: 'async_recursion,
	T: Detokenize + Send + Sync,
{
	let log_receipt = |receipt: TransactionReceipt, cancelled: bool| -> Result<(), anyhow::Error> {
		let prelude = if cancelled { "Cancellation Tx" } else { "Tx" };
		if matches!(receipt.status.as_ref().map(|f| f.low_u64()), Some(1)) {
			log::info!("{prelude} for {:?} succeeded", state_machine);
		} else {
			log::info!(
				"{prelude} for {:?} with hash {:?} reverted",
				state_machine,
				receipt.transaction_hash
			);
			Err(anyhow!("Transaction reverted"))?
		}

		Ok(())
	};

	let client_clone = provider.clone();
	let signer_clone = signer.clone();
	let state_machine_clone = state_machine.clone();
	let etherscan_api_key_clone = etherscan_api_key.clone();

	let handle_failed_tx = move || async move {
		log::info!("No receipt for transaction on {:?}", state_machine_clone);

		if let Some(call) = retry {
			// lets retry
			let gas_price: U256 = get_current_gas_cost_in_usd(
				state_machine_clone,
				&etherscan_api_key_clone,
				client_clone.clone(),
			)
			.await?
			.gas_price * 2; // for good measure
			log::info!(
				"Retrying consensus message on {:?} with gas {}",
				state_machine_clone,
				ethers::utils::format_units(
					// Conversion needed because of ether-rs and polkadot-sdk incompatibility
					old_u256(gas_price),
					"gwei"
				)?
			);
			// Conversion needed because of ether-rs and polkadot-sdk incompatibility
			let call = call.gas_price(old_u256(gas_price));
			let pending = call.send().await?;

			// don't retry in the next callstack
			wait_for_success::<()>(
				&state_machine_clone,
				&etherscan_api_key_clone,
				client_clone.clone(),
				signer_clone.clone(),
				pending,
				Some(gas_price),
				None,
				is_consensus,
			)
			.await
		} else {
			// cancel the transaction here
			let pending = signer_clone
				.send_transaction(
					TypedTransaction::Legacy(TransactionRequest {
						to: Some(NameOrAddress::Address(signer_clone.address())),
						value: Some(Default::default()),
						gas_price: gas_price.map(|price| {
							let new_price: U256 = price * 10;
							// Conversion needed because of ether-rs and polkadot-sdk
							// incompatibility
							old_u256(new_price)
						}), // experiment with higher?
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

			// Throw an error only when consensus messages are cancelled
			// Consensus relayer expects an error when consensus messages fail to submit so they can
			// be retried
			if is_consensus {
				Err(anyhow!("Transaction to {:?} was cancelled!", state_machine_clone))?
			}

			log::error!("Transaction to {:?} was cancelled!", state_machine_clone);
			Ok(Default::default())
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
					log::error!("Error broadcasting transaction to {:?}: {err:?}", state_machine);
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
) -> anyhow::Result<Vec<SolidityFunctionCall<()>>> {
	let handler = client.handler().await?;
	let contract = IsmpHandler::new(handler.0, client.signer.clone());
	let ismp_host = client.config.ismp_host;
	let mut calls = Vec::new();
	// If debug trace is false or the client type is erigon, then the gas price must be set
	// Geth does not require gas price to be set when debug tracing, but the erigon implementation
	// does https://github.com/ledgerwatch/erigon/blob/cfb55a3cd44736ac092003be41659cc89061d1be/core/state_transition.go#L246
	// Erigon does not support block overrides when tracing so we don't have the option of omiting
	// the gas price by overriding the base fee
	let set_gas_price = || !debug_trace || client.client_type.erigon();
	let mut gas_price = if set_gas_price() {
		get_current_gas_cost_in_usd(
			client.state_machine,
			&client.config.etherscan_api_key.clone(),
			client.client.clone(),
		)
		.await?
		.gas_price
	} else {
		Default::default()
	};

	// Only use gas price buffer when submitting transactions
	if !debug_trace && client.config.gas_price_buffer.is_some() {
		let buffer = (U256::from(client.config.gas_price_buffer.unwrap_or_default()) * gas_price) /
			U256::from(100u32);
		gas_price = gas_price + buffer
	}

	for message in messages {
		match message {
			Message::Consensus(msg) => {
				let call =
					contract.handle_consensus(ismp_host.0.into(), msg.consensus_proof.into());
				let estimated_gas = call
					.estimate_gas()
					.await
					.unwrap_or(get_chain_gas_limit(client.state_machine).into());
				let gas_limit = estimated_gas + ((estimated_gas * 5) / 100); // 5% buffer
																 // U256 Conversion needed because of ether-rs and polkadot-sdk incompatibility
				let call = call.gas_price(old_u256(gas_price)).gas(gas_limit);

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

				let call = contract.handle_post_requests(ismp_host.0.into(), post_message);
				let estimated_gas = call
					.estimate_gas()
					.await
					.unwrap_or(get_chain_gas_limit(client.state_machine).into());
				let gas_limit = estimated_gas + ((estimated_gas * 5) / 100); // 5% buffer

				// U256 Conversion needed because of ether-rs and polkadot-sdk incompatibility
				let call = if set_gas_price() {
					call.gas_price(old_u256(gas_price)).gas(gas_limit)
				} else {
					call.gas(gas_limit)
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

						let call = contract.handle_post_responses(ismp_host.0.into(), message);
						let estimated_gas = call
							.estimate_gas()
							.await
							.unwrap_or(get_chain_gas_limit(client.state_machine).into());
						let gas_limit = estimated_gas + ((estimated_gas * 5) / 100); // 5% buffer

						if set_gas_price() {
							// U256 Conversion needed because of ether-rs and polkadot-sdk
							// incompatibility
							call.gas_price(old_u256(gas_price)).gas(gas_limit)
						} else {
							call.gas(gas_limit)
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
		StateMachine::Evm(ARBITRUM_CHAIN_ID) | StateMachine::Evm(ARBITRUM_SEPOLIA_CHAIN_ID) =>
			32_000_000,
		StateMachine::Evm(GNOSIS_CHAIN_ID) | StateMachine::Evm(CHIADO_CHAIN_ID) => 16_000_000,
		// Gas limit is 10_000_000, we set our transaction gas limit to 40% of that
		StateMachine::Evm(SEI_CHAIN_ID) | StateMachine::Evm(SEI_TESTNET_CHAIN_ID) => 4_000_000,
		// Gas limit is 60_000_000, we set our transaction gas limit to 30% of that
		StateMachine::Evm(CRONOS_CHAIN_ID) | StateMachine::Evm(CRONOS_TESTNET_CHAIN_ID) =>
			18_000_000,
		// Gas limit is 50_000_000, we set our transaction gas limit to 30% of that
		StateMachine::Evm(INJECTIVE_CHAIN_ID) | StateMachine::Evm(INJECTIVE_TESTNET_CHAIN_ID) =>
			15_000_000,
		// Ethereum L1 max's gas limit per transaction will be reduced to 16m soon.
		StateMachine::Evm(_) => 16_000_000,
		_ => Default::default(),
	}
}

pub async fn handle_message_submission(
	client: &EvmClient,
	messages: Vec<Message>,
) -> Result<TxResult, anyhow::Error> {
	let (receipts, cancelled) = submit_messages(client, messages.clone()).await?;
	let height = client.client.get_block_number().await?.low_u64();
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

	Ok(TxResult { receipts: results, unsuccessful: cancelled })
}
