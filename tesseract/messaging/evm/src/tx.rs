use crate::{
	gas_oracle::{
		ARBITRUM_CHAIN_ID, ARBITRUM_SEPOLIA_CHAIN_ID, CHIADO_CHAIN_ID, CRONOS_CHAIN_ID,
		CRONOS_TESTNET_CHAIN_ID, GNOSIS_CHAIN_ID, INJECTIVE_CHAIN_ID, INJECTIVE_TESTNET_CHAIN_ID,
		SEI_CHAIN_ID, SEI_TESTNET_CHAIN_ID,
	},
	AlloyProvider, EvmClient,
};
use anyhow::anyhow;
use codec::Decode;
use alloy::{
	primitives::{Address, Bytes, B256, U256 as AlloyU256},
	providers::Provider,
	rpc::types::{TransactionReceipt, TransactionRequest},
	transports::TransportError,
};
use ismp::{
	host::StateMachine,
	messaging::{hash_request, hash_response, Message, ResponseMessage},
	router::{Request, RequestResponse, Response},
};
use alloy_sol_types::SolEvent;
use ismp_solidity_abi::{
	evm_host::{PostRequestHandled, PostResponseHandled},
	handler::{
		HandlerInstance, PostRequest as SolPostRequest, PostRequestLeaf, PostRequestMessage,
		PostResponse as SolPostResponse, PostResponseLeaf, PostResponseMessage, Proof,
		StateMachineHeight,
	},
};
use mmr_primitives::mmr_position_to_k_index;
use pallet_ismp::offchain::{LeafIndexAndPos, Proof as MmrProof};
use polkadot_sdk::sp_mmr_primitives::utils::NodesUtils;
use primitive_types::{H256, U256};
use std::{collections::BTreeSet, sync::Arc, time::Duration};
use tesseract_primitives::{Hasher, Query, TxReceipt, TxResult};

use crate::gas_oracle::get_current_gas_cost_in_usd;
use ismp::router::{PostRequest, PostResponse};

fn convert_post_request(post: PostRequest) -> SolPostRequest {
	SolPostRequest {
		source: Bytes::from(post.source.to_string().as_bytes().to_vec()),
		dest: Bytes::from(post.dest.to_string().as_bytes().to_vec()),
		nonce: post.nonce,
		from: Bytes::from(post.from.clone()),
		to: Bytes::from(post.to.clone()),
		timeoutTimestamp: post.timeout_timestamp,
		body: Bytes::from(post.body.clone()),
	}
}


fn convert_post_response(res: PostResponse) -> SolPostResponse {
	SolPostResponse {
		request: convert_post_request(res.post),
		response: Bytes::from(res.response.clone()),
		timeoutTimestamp: res.timeout_timestamp,
	}
}

/// Check if an error is a rate limit (429) or other retryable RPC error using alloy's
/// structured error types. This covers HTTP 429, Alchemy -32016, Infura -32005, QuickNode
/// -32007/-32012, and other provider-specific rate limit codes.
fn is_rate_limit_error(err: &anyhow::Error) -> bool {
	if let Some(transport_err) = err.downcast_ref::<TransportError>() {
		match transport_err {
			TransportError::Transport(kind) => kind.is_retry_err(),
			TransportError::ErrorResp(payload) => payload.is_retry_err(),
			_ => false,
		}
	} else {
		// Fallback: check string representation for "429" in case the error was wrapped
		let err_str = format!("{:?}", err);
		err_str.contains("429")
	}
}

#[async_recursion::async_recursion]
pub async fn submit_messages(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<(BTreeSet<H256>, Vec<Message>)> {
	let (tx_requests, gas_price) = match generate_contract_calls(client, messages.clone()).await {
		Ok(result) => result,
		Err(err) => {
			if is_rate_limit_error(&err) {
				log::info!("Retrying tx submission, got rate limit error");
				return submit_messages(&client, messages).await;
			}
			return Err(err);
		},
	};

	let mut events = BTreeSet::new();
	let mut cancelled: Vec<Message> = vec![];

	for (index, tx) in tx_requests.into_iter().enumerate() {
		let pending = match client.signer.send_transaction(tx).await {
			Ok(pending) => pending,
			Err(err) => {
				let err = anyhow::Error::from(err);
				if is_rate_limit_error(&err) {
					log::info!("Retrying tx submission, got rate limit error");
					return submit_messages(&client, messages).await;
				}
				return Err(err);
			},
		};

		let tx_hash = *pending.tx_hash();
		let is_consensus = matches!(messages[index], Message::Consensus(_));
		let retry_message =
			if is_consensus { Some(messages[index].clone()) } else { None };
		let evs = wait_for_success(
			client,
			H256::from_slice(tx_hash.as_slice()),
			gas_price,
			retry_message,
			is_consensus,
		)
		.await?;
		if matches!(messages[index], Message::Request(_) | Message::Response(_)) &&
			evs.is_empty()
		{
			cancelled.push(messages[index].clone())
		}
		events.extend(evs);
	}

	if !events.is_empty() {
		log::trace!("Got {} receipts from executing on {:?}", events.len(), client.state_machine);
	}

	Ok((events, cancelled))
}

/// Waits for a transaction receipt by polling with a 7-second interval for up to 5 minutes.
pub async fn wait_for_transaction_receipt(
	tx_hash: H256,
	provider: Arc<AlloyProvider>,
) -> Result<Option<TransactionReceipt>, anyhow::Error> {
	let poll_interval = Duration::from_secs(7);
	let max_duration = Duration::from_secs(5 * 60);
	let start_time = tokio::time::Instant::now();

	loop {
		if start_time.elapsed() >= max_duration {
			log::error!("Transaction receipt not found after 5 minutes for tx: {:?}", tx_hash);
			return Ok(None);
		}

		match provider.get_transaction_receipt(B256::from_slice(&tx_hash.0)).await {
			Ok(Some(receipt)) => {
				log::trace!("Transaction receipt found for tx: {:?}", tx_hash);
				return Ok(Some(receipt));
			},
			Ok(None) => {
				log::trace!(
					"Transaction receipt not yet available for tx: {:?}, will retry in 7 seconds",
					tx_hash
				);
			},
			Err(err) => {
				log::warn!("Error querying transaction receipt for tx: {:?}: {err:?}", tx_hash);
			},
		}

		tokio::time::sleep(poll_interval).await;
	}
}

#[async_recursion::async_recursion]
pub async fn wait_for_success(
	client: &EvmClient,
	tx_hash: H256,
	gas_price: U256,
	retry_message: Option<Message>,
	is_consensus: bool,
) -> Result<BTreeSet<H256>, anyhow::Error> {
	let state_machine = &client.config.state_machine;

	match wait_for_transaction_receipt(tx_hash, client.client.clone()).await? {
		Some(receipt) => {
			let events = receipt
				.inner
				.logs()
				.iter()
				.filter_map(|l| {
					if let Ok(ev) = PostRequestHandled::decode_log(&l.inner) {
						return Some(H256::from_slice(ev.commitment.as_slice()))
					}
					if let Ok(ev) = PostResponseHandled::decode_log(&l.inner) {
						return Some(H256::from_slice(ev.commitment.as_slice()))
					}
					None
				})
				.collect();
			if receipt.status() {
				log::info!("Tx for {:?} succeeded", state_machine);
			} else {
				log::info!(
					"Tx for {:?} with hash {:?} reverted",
					state_machine,
					receipt.transaction_hash
				);
				Err(anyhow!("Transaction reverted"))?
			}
			Ok(events)
		},
		None => {
			// Transaction timed out - no receipt after 5 minutes
			log::info!("No receipt for transaction on {:?}", state_machine);

			if let Some(msg) = retry_message {
				// Retry consensus messages with 2x gas price
				let new_gas_price: U256 = get_current_gas_cost_in_usd(
					client.state_machine,
					client.config.ismp_host.0.into(),
					client.client.clone(),
				)
				.await?
				.gas_price * 2;

				let gas_gwei = new_gas_price.low_u128() as f64 / 1e9;
				log::info!(
					"Retrying consensus message on {:?} with gas {:.4} gwei",
					state_machine,
					gas_gwei,
				);

				let handler = client.handler().await?;
				let handler_addr = Address::from_slice(&handler.0);
				let contract = HandlerInstance::new(handler_addr, client.signer.clone());
				let ismp_host = Address::from_slice(&client.config.ismp_host.0);

				match msg {
					Message::Consensus(consensus_msg) => {
						let call = contract.handleConsensus(
							ismp_host,
							Bytes::from(consensus_msg.consensus_proof),
						);
						let estimated_gas = call
							.estimate_gas()
							.await
							.unwrap_or(get_chain_gas_limit(client.state_machine));
						let gas_limit = estimated_gas + ((estimated_gas * 5) / 100);
						let call = call.gas_price(new_gas_price.low_u128()).gas(gas_limit);
						let pending = call.send().await?;
						let new_tx_hash = H256::from_slice(pending.tx_hash().as_slice());
						// Don't retry again in the recursive call
						wait_for_success(client, new_tx_hash, new_gas_price, None, is_consensus)
							.await
					},
					_ => Err(anyhow!("Only consensus messages can be retried")),
				}
			} else {
				// Cancel the stuck transaction with a self-transfer at higher gas price
				let from_address = Address::from_slice(&client.address);
				let cancel_gas_price: U256 = gas_price * U256::from(10);
				let tx = TransactionRequest::default()
					.to(from_address)
					.value(AlloyU256::ZERO)
					.gas_price(cancel_gas_price.low_u128());

				if let Ok(pending) = client.signer.send_transaction(tx).await {
					let cancel_hash = H256::from_slice(pending.tx_hash().as_slice());
					if let Ok(Some(receipt)) =
						wait_for_transaction_receipt(cancel_hash, client.client.clone()).await
					{
						let prelude = "Cancellation Tx";
						if receipt.status() {
							log::info!("{prelude} for {:?} succeeded", state_machine);
						} else {
							log::info!("{prelude} for {:?} reverted", state_machine);
						}
					}
				}

				if is_consensus {
					Err(anyhow!("Transaction to {:?} was cancelled!", state_machine))?
				}

				log::error!("Transaction to {:?} was cancelled!", state_machine);
				Ok(Default::default())
			}
		},
	}
}

/// Result of estimating gas for a message
#[derive(Clone)]
pub struct MessageGasEstimate {
	/// Estimated gas for execution
	pub gas_estimate: u64,
	/// Calldata bytes for L2 data cost calculation
	pub calldata: Vec<u8>,
}

/// Function estimates gas for messages without sending them
pub async fn estimate_gas_for_messages(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<Vec<MessageGasEstimate>> {
	let handler = client.handler().await?;
	let handler_addr = Address::from_slice(&handler.0);
	let contract = HandlerInstance::new(handler_addr, client.signer.clone());
	let ismp_host = Address::from_slice(&client.config.ismp_host.0);
	let mut estimates = Vec::new();

	for message in messages {
		match message {
			Message::Consensus(msg) => {
				let call = contract.handleConsensus(ismp_host, Bytes::from(msg.consensus_proof));
				let gas_estimate = call.estimate_gas().await.unwrap_or(0);
				let calldata = call.calldata().to_vec();
				estimates.push(MessageGasEstimate { gas_estimate, calldata });
			},
			Message::Request(msg) => {
				let membership_proof = MmrProof::<H256>::decode(&mut msg.proof.proof.as_slice())?;
				let mmr_size = NodesUtils::new(membership_proof.leaf_count).size();
				let k_and_leaf_indices = membership_proof
					.leaf_indices_and_pos
					.iter()
					.map(|LeafIndexAndPos { pos, leaf_index }| {
						let k_index = mmr_position_to_k_index(vec![*pos], mmr_size)[0].1;
						(k_index, *leaf_index)
					})
					.collect::<Vec<_>>();

				let mut leaves = msg
					.requests
					.iter()
					.cloned()
					.zip(k_and_leaf_indices)
					.map(|(post, (k_index, leaf_index))| PostRequestLeaf {
						request: convert_post_request(post),
						index: AlloyU256::from(leaf_index),
						kIndex: AlloyU256::from(k_index),
					})
					.collect::<Vec<_>>();
				leaves.sort_by(|a, b| a.index.cmp(&b.index));

				let post_message = PostRequestMessage {
					proof: Proof {
						height: StateMachineHeight {
							stateMachineId: {
								match msg.proof.height.id.state_id {
									StateMachine::Polkadot(id) | StateMachine::Kusama(id) =>
										AlloyU256::from(id),
									_ => continue,
								}
							},
							height: AlloyU256::from(msg.proof.height.height),
						},
						multiproof: membership_proof.items.iter().map(|node| B256::from_slice(&node.0)).collect(),
						leafCount: AlloyU256::from(membership_proof.leaf_count),
					},
					requests: leaves,
				};

				let call = contract.handlePostRequests(ismp_host, post_message);
				let gas_estimate = call.estimate_gas().await.unwrap_or(0);
				let calldata = call.calldata().to_vec();
				estimates.push(MessageGasEstimate { gas_estimate, calldata });
			},
			Message::Response(ResponseMessage { datagram, proof, .. }) => {
				let membership_proof = MmrProof::<H256>::decode(&mut proof.proof.as_slice())?;
				let mmr_size = NodesUtils::new(membership_proof.leaf_count).size();
				let k_and_leaf_indices = membership_proof
					.leaf_indices_and_pos
					.iter()
					.map(|LeafIndexAndPos { pos, leaf_index }| {
						let k_index = mmr_position_to_k_index(vec![*pos], mmr_size)[0].1;
						(k_index, *leaf_index)
					})
					.collect::<Vec<_>>();

				match datagram {
					RequestResponse::Response(responses) => {
						let mut leaves = responses
							.iter()
							.cloned()
							.zip(k_and_leaf_indices)
							.filter_map(|(res, (k_index, leaf_index))| match res {
								Response::Post(res) => Some(PostResponseLeaf {
									response: convert_post_response(res),
									index: AlloyU256::from(leaf_index),
									kIndex: AlloyU256::from(k_index),
								}),
								_ => None,
							})
							.collect::<Vec<_>>();
						leaves.sort_by(|a, b| a.index.cmp(&b.index));

						let message = PostResponseMessage {
							proof: Proof {
								height: StateMachineHeight {
									stateMachineId: {
										match proof.height.id.state_id {
											StateMachine::Polkadot(id) |
											StateMachine::Kusama(id) => AlloyU256::from(id),
											_ => continue,
										}
									},
									height: AlloyU256::from(proof.height.height),
								},
								multiproof: membership_proof
									.items
									.iter()
									.map(|node| B256::from_slice(&node.0))
									.collect(),
								leafCount: AlloyU256::from(membership_proof.leaf_count),
							},
							responses: leaves,
						};

						let call = contract.handlePostResponses(ismp_host, message);
						let gas_estimate = call.estimate_gas().await.unwrap_or(0);
						let calldata = call.calldata().to_vec();
						estimates.push(MessageGasEstimate { gas_estimate, calldata });
					},
					RequestResponse::Request(..) => continue,
				}
			},
			Message::Timeout(_) | Message::FraudProof(_) => continue,
		}
	}

	Ok(estimates)
}

/// Function generates contract calls from batches of messages without sending them.
/// Returns the unsent transaction requests along with the gas price used.
pub async fn generate_contract_calls(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<(Vec<TransactionRequest>, U256)> {
	let handler = client.handler().await?;
	let handler_addr = Address::from_slice(&handler.0);
	let contract = HandlerInstance::new(handler_addr, client.signer.clone());
	let ismp_host = Address::from_slice(&client.config.ismp_host.0);
	let mut tx_requests = Vec::new();

	let mut gas_price = get_current_gas_cost_in_usd(client.state_machine, client.config.ismp_host.0.into(), client.client.clone())
		.await?
		.gas_price;

	// Apply gas price buffer
	if client.config.gas_price_buffer.is_some() {
		let buffer = (U256::from(client.config.gas_price_buffer.unwrap_or_default()) * gas_price) /
			U256::from(100u32);
		gas_price = gas_price + buffer
	}

	// Convert U256 to u128 for gas_price parameter
	let gas_price_u128 = gas_price.low_u128();

	for message in messages {
		match message {
			Message::Consensus(msg) => {
				let call = contract.handleConsensus(ismp_host, Bytes::from(msg.consensus_proof));
				let estimated_gas = call
					.estimate_gas()
					.await
					.unwrap_or(get_chain_gas_limit(client.state_machine));
				let gas_limit = estimated_gas + ((estimated_gas * 5) / 100); // 5% buffer
				let calldata = call.calldata().clone();

				let tx = TransactionRequest::default()
					.to(handler_addr)
					.input(calldata.into())
					.gas_price(gas_price_u128)
					.gas_limit(gas_limit);
				tx_requests.push(tx);
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
						request: convert_post_request(post),
						index: AlloyU256::from(leaf_index),
						kIndex: AlloyU256::from(k_index),
					})
					.collect::<Vec<_>>();
				leaves.sort_by(|a, b| a.index.cmp(&b.index));

				let post_message = PostRequestMessage {
					proof: Proof {
						height: StateMachineHeight {
							stateMachineId: {
								match msg.proof.height.id.state_id {
									StateMachine::Polkadot(id) | StateMachine::Kusama(id) =>
										AlloyU256::from(id),
									_ => {
										panic!("Expected polkadot or kusama state machines");
									},
								}
							},
							height: AlloyU256::from(msg.proof.height.height),
						},
						multiproof: membership_proof.items.into_iter().map(|node| B256::from_slice(&node.0)).collect(),
						leafCount: AlloyU256::from(membership_proof.leaf_count),
					},
					requests: leaves,
				};

				let call = contract.handlePostRequests(ismp_host, post_message);
				let estimated_gas = call
					.estimate_gas()
					.await
					.unwrap_or(get_chain_gas_limit(client.state_machine));
				let gas_limit = estimated_gas + ((estimated_gas * 5) / 100); // 5% buffer
				let calldata = call.calldata().clone();

				let tx = TransactionRequest::default()
					.to(handler_addr)
					.input(calldata.into())
					.gas_price(gas_price_u128)
					.gas_limit(gas_limit);
				tx_requests.push(tx);
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

				match datagram {
					RequestResponse::Response(responses) => {
						let mut leaves = responses
							.into_iter()
							.zip(k_and_leaf_indices)
							.filter_map(|(res, (k_index, leaf_index))| match res {
								Response::Post(res) => Some(PostResponseLeaf {
									response: convert_post_response(res),
									index: AlloyU256::from(leaf_index),
									kIndex: AlloyU256::from(k_index),
								}),
								_ => None,
							})
							.collect::<Vec<_>>();
						leaves.sort_by(|a, b| a.index.cmp(&b.index));

						let message = PostResponseMessage {
							proof: Proof {
								height: StateMachineHeight {
									stateMachineId: {
										match proof.height.id.state_id {
											StateMachine::Polkadot(id) |
											StateMachine::Kusama(id) => AlloyU256::from(id),
											_ => {
												log::error!("Expected polkadot or kusama state machines");
												continue;
											},
										}
									},
									height: AlloyU256::from(proof.height.height),
								},
								multiproof: membership_proof
									.items
									.into_iter()
									.map(|node| B256::from_slice(&node.0))
									.collect(),
								leafCount: AlloyU256::from(membership_proof.leaf_count),
							},
							responses: leaves,
						};

						let call = contract.handlePostResponses(ismp_host, message);
						let estimated_gas = call
							.estimate_gas()
							.await
							.unwrap_or(get_chain_gas_limit(client.state_machine));
						let gas_limit = estimated_gas + ((estimated_gas * 5) / 100); // 5% buffer
						let calldata = call.calldata().clone();

						let tx = TransactionRequest::default()
							.to(handler_addr)
							.input(calldata.into())
							.gas_price(gas_price_u128)
							.gas_limit(gas_limit);
						tx_requests.push(tx);
					},
					RequestResponse::Request(..) =>
						Err(anyhow!("Get requests are not supported by relayer"))?,
				};
			},
			Message::Timeout(_) => Err(anyhow!("Timeout messages not supported by relayer"))?,
			Message::FraudProof(_) => Err(anyhow!("Unexpected fraud proof message"))?,
		}
	}

	Ok((tx_requests, gas_price))
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
	let height = client.client.get_block_number().await?;
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

#[cfg(test)]
mod tests {
	use super::*;
	use alloy::providers::{Provider, RootProvider};

	#[tokio::test]
	#[ignore] // Requires local RPC node
	async fn test_wait_for_transaction_receipt() {
		let _ = env_logger::builder().is_test(true).try_init();

		let provider = Arc::new(RootProvider::new_http("http://localhost:8545".parse().unwrap()));

		let tx_hash: H256 = "0xf43c2f2910bb84fdd9f4bd94378469195d4e0b401802c6fb8d3d74a20abef3da"
			.parse()
			.expect("Failed to parse transaction hash");

		match wait_for_transaction_receipt(tx_hash, provider).await {
			Ok(Some(receipt)) => {
				println!("✅ Transaction receipt found!");
				println!("Transaction hash: {:?}", receipt.transaction_hash);
				println!("Block number: {:?}", receipt.block_number);
				println!("Gas used: {:?}", receipt.gas_used);
				println!("Status: {:?}", receipt.status());
			},
			Ok(None) => {
				println!("❌ Transaction receipt not found after 5 minutes");
			},
			Err(err) => {
				println!("❌ Error fetching transaction receipt: {err:?}");
			},
		}
	}

	#[tokio::test]
	#[ignore] // Requires local RPC node
	async fn test_get_block() {
		// Initialize logger
		let _ = env_logger::builder().is_test(true).try_init();

		let provider = Arc::new(RootProvider::new_http("http://localhost:8545".parse().unwrap()));

		// Block number to test
		let block_number: u64 = 4726213;

		println!("Fetching block {block_number}...");

		// Get block by number
		match provider.get_block(block_number.into()).await {
			Ok(Some(block)) => {
				println!("Block found!");
				println!("Block number: {:?}", block.header.number);
				println!("Block hash: {:?}", block.header.hash);
				println!("Parent hash: {:?}", block.header.parent_hash);
				println!("Timestamp: {:?}", block.header.timestamp);
				println!("Gas used: {:?}", block.header.gas_used);
				println!("Gas limit: {:?}", block.header.gas_limit);
				println!("Miner: {:?}", block.header.beneficiary);
				println!("Number of transactions: {}", block.transactions.len());
				println!("State root: {:?}", block.header.state_root);
			},
			Ok(None) => {
				println!("❌ Block not found");
				panic!("Block {block_number} should exist");
			},
			Err(err) => {
				println!("❌ Error fetching block: {err:?}");
				panic!("Failed to fetch block: {err:?}");
			},
		}
	}
}
