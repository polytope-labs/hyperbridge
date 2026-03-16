use crate::{
	gas_oracle::{
		ARBITRUM_CHAIN_ID, ARBITRUM_SEPOLIA_CHAIN_ID, CHIADO_CHAIN_ID, CRONOS_CHAIN_ID,
		CRONOS_TESTNET_CHAIN_ID, GNOSIS_CHAIN_ID, INJECTIVE_CHAIN_ID, INJECTIVE_TESTNET_CHAIN_ID,
		SEI_CHAIN_ID, SEI_TESTNET_CHAIN_ID,
	},
	AlloyProvider, EvmClient,
};
use alloy::{
	consensus::{Eip658Value, TxReceipt as AlloyTxReceipt},
	primitives::{Address, Bytes, B256, U256 as AlloyU256},
	providers::Provider,
	rpc::types::{TransactionReceipt, TransactionRequest},
	transports::TransportError,
};
use alloy_sol_types::SolEvent;
use anyhow::anyhow;
use codec::Decode;
use ismp::{
	host::StateMachine,
	messaging::{hash_request, hash_response, Message, ResponseMessage},
	router::{Request, RequestResponse, Response},
};
use ismp_solidity_abi::{
	evm_host::{PostRequestHandled, PostResponseHandled},
	handler::{
		HandlerInstance, PostRequestLeaf, PostRequestMessage, PostResponseLeaf,
		PostResponseMessage, Proof, StateMachineHeight,
	},
};
use mmr_primitives::mmr_position_to_k_index;
use pallet_ismp::offchain::{LeafIndexAndPos, Proof as MmrProof};
use polkadot_sdk::sp_mmr_primitives::utils::NodesUtils;
use primitive_types::{H256, U256};
use std::{collections::BTreeSet, sync::Arc, time::Duration};
use tesseract_primitives::{Hasher, Query, TxReceipt, TxResult};

use crate::gas_oracle::get_current_gas_cost_in_usd;

// ---------------------------------------------------------------------------
// Helpers to eliminate duplication
// ---------------------------------------------------------------------------

/// Build a `TransactionRequest` with optional gas price.
fn build_tx_request(
	from: Address,
	to: Address,
	calldata: Bytes,
	gas_price_u128: u128,
	gas_limit: u64,
	use_gas_price: bool,
) -> TransactionRequest {
	let tx = TransactionRequest::default()
		.from(from)
		.to(to)
		.input(calldata.into())
		.gas_limit(gas_limit);

	if use_gas_price {
		tx.gas_price(gas_price_u128)
	} else {
		tx
	}
}

/// Apply a 5% buffer to a gas estimate, falling back to a chain-specific limit on error.
fn gas_with_buffer(
	estimated: Result<u64, impl std::fmt::Display>,
	state_machine: StateMachine,
) -> u64 {
	let est = estimated.unwrap_or_else(|e| {
		let fallback = get_chain_gas_limit(state_machine) / 4;
		tracing::warn!("Gas estimation failed: {e}, using fallback: {fallback}");
		fallback
	});
	est + ((est * 5) / 100)
}

/// Decode an MMR proof and compute the (k_index, leaf_index) pairs.
fn decode_mmr_proof_with_indices(
	raw_proof: &[u8],
) -> anyhow::Result<(MmrProof<H256>, Vec<(u64, u64)>)> {
	let proof = MmrProof::<H256>::decode(&mut &*raw_proof)?;
	let mmr_size = NodesUtils::new(proof.leaf_count).size();

	let k_and_leaf_indices = proof
		.leaf_indices_and_pos
		.iter()
		.map(|LeafIndexAndPos { pos, leaf_index }| {
			let k_index = mmr_position_to_k_index(vec![*pos], mmr_size)[0].1;
			(k_index, *leaf_index)
		})
		.collect();

	Ok((proof, k_and_leaf_indices))
}

/// Extract the numeric state machine ID, expecting Polkadot or Kusama.
fn extract_state_machine_id(state_id: &StateMachine) -> anyhow::Result<AlloyU256> {
	match state_id {
		StateMachine::Polkadot(id) | StateMachine::Kusama(id) => Ok(AlloyU256::from(*id)),
		other => Err(anyhow!("Expected Polkadot or Kusama state machine, got: {other:?}")),
	}
}

/// Build a solidity `Proof` from an MMR proof and height.
fn build_solidity_proof(
	mmr_proof: MmrProof<H256>,
	height: &ismp::consensus::StateMachineHeight,
) -> anyhow::Result<Proof> {
	Ok(Proof {
		height: StateMachineHeight {
			stateMachineId: extract_state_machine_id(&height.id.state_id)?,
			height: AlloyU256::from(height.height),
		},
		multiproof: mmr_proof
			.items
			.into_iter()
			.map(|node| B256::from_slice(&node.0))
			.collect(),
		leafCount: AlloyU256::from(mmr_proof.leaf_count),
	})
}

/// Extract handled-event commitments from a transaction receipt.
fn extract_event_commitments(receipt: &TransactionReceipt) -> BTreeSet<H256> {
	receipt
		.inner
		.logs()
		.iter()
		.filter_map(|log| {
			PostRequestHandled::decode_log(&log.inner)
				.map(|ev| H256::from_slice(ev.commitment.as_slice()))
				.ok()
				.or_else(|| {
					PostResponseHandled::decode_log(&log.inner)
						.map(|ev| H256::from_slice(ev.commitment.as_slice()))
						.ok()
				})
		})
		.collect()
}

/// Check if an error is a rate limit (429) or other retryable RPC error.
fn is_rate_limit_error(err: &anyhow::Error) -> bool {
	if let Some(transport_err) = err.downcast_ref::<TransportError>() {
		match transport_err {
			TransportError::Transport(kind) => kind.is_retry_err(),
			TransportError::ErrorResp(payload) => payload.is_retry_err(),
			_ => false,
		}
	} else {
		format!("{err:?}").contains("429")
	}
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Submit ISMP messages as EVM transactions, returning handled commitments and cancelled messages.
#[tracing::instrument(skip_all, fields(chain = ?client.state_machine))]
#[async_recursion::async_recursion]
pub async fn submit_messages(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<(BTreeSet<H256>, Vec<Message>)> {
	let (tx_requests, gas_price) = generate_contract_calls(client, messages.clone(), false).await?;

	let mut events = BTreeSet::new();
	let mut cancelled: Vec<Message> = vec![];

	for (index, tx) in tx_requests.into_iter().enumerate() {
		let pending = match client.signer.send_transaction(tx).await {
			Ok(pending) => pending,
			Err(err) => {
				let err = anyhow::Error::from(err);
				if is_rate_limit_error(&err) {
					tracing::info!("Rate-limited, retrying tx submission");
					return submit_messages(client, messages).await;
				}
				return Err(err);
			},
		};

		let tx_hash = H256::from_slice(pending.tx_hash().as_slice());
		let is_consensus = matches!(messages[index], Message::Consensus(_));
		let retry_message = if is_consensus { Some(messages[index].clone()) } else { None };

		let evs = wait_for_success(client, tx_hash, gas_price, retry_message, is_consensus).await?;

		if matches!(messages[index], Message::Request(_) | Message::Response(_)) && evs.is_empty() {
			cancelled.push(messages[index].clone());
		}
		events.extend(evs);
	}

	if !events.is_empty() {
		tracing::trace!("Got {} receipts from {:?}", events.len(), client.state_machine);
	}

	Ok((events, cancelled))
}

/// Poll for a transaction receipt with a 7-second interval, timing out after 5 minutes.
pub async fn wait_for_transaction_receipt(
	tx_hash: H256,
	provider: Arc<AlloyProvider>,
) -> Result<Option<TransactionReceipt>, anyhow::Error> {
	let poll_interval = Duration::from_secs(7);
	let deadline = tokio::time::Instant::now() + Duration::from_secs(5 * 60);

	loop {
		if tokio::time::Instant::now() >= deadline {
			tracing::error!("No receipt after 5 minutes for tx {tx_hash:?}");
			return Ok(None);
		}

		match provider.get_transaction_receipt(B256::from_slice(&tx_hash.0)).await {
			Ok(Some(receipt)) => return Ok(Some(receipt)),
			Ok(None) => {},
			Err(err) => tracing::warn!("Error querying receipt for {tx_hash:?}: {err:?}"),
		}

		tokio::time::sleep(poll_interval).await;
	}
}

/// Wait for a transaction to be mined and succeed. Handles receipt polling, consensus-message
/// retries (with doubled gas price), and stuck-transaction cancellation.
#[tracing::instrument(skip_all, fields(chain = ?client.state_machine, ?tx_hash))]
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
			let events = extract_event_commitments(&receipt);

			if receipt.inner.status_or_post_state() == Eip658Value::Eip658(true) {
				tracing::info!("Tx for {state_machine:?} succeeded");
			} else {
				tracing::info!(
					"Tx for {state_machine:?} with hash {:?} reverted",
					receipt.transaction_hash
				);
				return Err(anyhow!("Transaction reverted"));
			}

			Ok(events)
		},
		None => handle_missing_receipt(client, gas_price, retry_message, is_consensus).await,
	}
}

/// Handle the case where no receipt was found within the timeout window.
async fn handle_missing_receipt(
	client: &EvmClient,
	gas_price: U256,
	retry_message: Option<Message>,
	is_consensus: bool,
) -> Result<BTreeSet<H256>, anyhow::Error> {
	let state_machine = &client.config.state_machine;
	tracing::info!("No receipt for transaction on {state_machine:?}");

	if let Some(msg) = retry_message {
		return retry_consensus_message(client, msg, is_consensus).await;
	}

	// Cancel the stuck transaction with a self-transfer at 10x gas price
	cancel_stuck_transaction(client, gas_price).await?;

	if is_consensus {
		return Err(anyhow!("Transaction to {state_machine:?} was cancelled!"));
	}

	tracing::error!("Transaction to {state_machine:?} was cancelled!");
	Ok(Default::default())
}

/// Retry a consensus message with doubled gas price.
async fn retry_consensus_message(
	client: &EvmClient,
	msg: Message,
	is_consensus: bool,
) -> Result<BTreeSet<H256>, anyhow::Error> {
	let Message::Consensus(consensus_msg) = msg else {
		return Err(anyhow!("Only consensus messages can be retried"));
	};

	let new_gas_price: U256 = get_current_gas_cost_in_usd(
		client.state_machine,
		client.config.ismp_host.0.into(),
		client.client.clone(),
	)
	.await?
	.gas_price * 2;

	tracing::info!(
		"Retrying consensus on {:?} at {:.4} gwei",
		client.config.state_machine,
		new_gas_price.low_u128() as f64 / 1e9,
	);

	let handler = client.handler().await?;
	let handler_addr = Address::from_slice(&handler.0);
	let contract = HandlerInstance::new(handler_addr, client.signer.clone());
	let ismp_host = Address::from_slice(&client.config.ismp_host.0);

	let call =
		contract.handleConsensus(ismp_host, Bytes::from(consensus_msg.consensus_proof));
	let gas_limit = {
		let est = call
			.estimate_gas()
			.await
			.unwrap_or(get_chain_gas_limit(client.state_machine) / 4);
		est + ((est * 5) / 100)
	};
	let pending =
		call.gas_price(new_gas_price.low_u128()).gas(gas_limit).send().await?;
	let new_tx_hash = H256::from_slice(pending.tx_hash().as_slice());

	// Don't retry again in the recursive call
	wait_for_success(client, new_tx_hash, new_gas_price, None, is_consensus).await
}

/// Cancel a stuck transaction by sending a zero-value self-transfer at 10x gas price.
async fn cancel_stuck_transaction(client: &EvmClient, gas_price: U256) -> anyhow::Result<()> {
	let state_machine = &client.config.state_machine;
	let from_address = Address::from_slice(&client.address);
	let cancel_gas_price = gas_price * U256::from(10);

	let tx = TransactionRequest::default()
		.to(from_address)
		.value(AlloyU256::ZERO)
		.gas_price(cancel_gas_price.low_u128());

	if let Ok(pending) = client.signer.send_transaction(tx).await {
		let cancel_hash = H256::from_slice(pending.tx_hash().as_slice());
		if let Ok(Some(receipt)) =
			wait_for_transaction_receipt(cancel_hash, client.client.clone()).await
		{
			let status = if receipt.inner.status_or_post_state() == Eip658Value::Eip658(true) {
				"succeeded"
			} else {
				"reverted"
			};
			tracing::info!("Cancellation tx for {state_machine:?} {status}");
		}
	}

	Ok(())
}

// ---------------------------------------------------------------------------
// Contract call generation
// ---------------------------------------------------------------------------

/// Generate unsigned `TransactionRequest`s from ISMP messages.
/// Returns the requests along with the gas price used.
#[tracing::instrument(skip_all, fields(chain = ?client.state_machine))]
pub async fn generate_contract_calls(
	client: &EvmClient,
	messages: Vec<Message>,
	debug_trace: bool,
) -> anyhow::Result<(Vec<TransactionRequest>, U256)> {
	let handler = client.handler().await?;
	let handler_addr = Address::from_slice(&handler.0);
	let contract = HandlerInstance::new(handler_addr, client.signer.clone());
	let ismp_host = Address::from_slice(&client.config.ismp_host.0);
	let from_address = Address::from_slice(&client.address);

	let gas_price = resolve_gas_price(client, debug_trace).await?;
	let gas_price_u128 = gas_price.low_u128();
	let use_gas_price = gas_price != U256::zero();

	let mut tx_requests = Vec::new();

	for message in messages {
		match message {
			Message::Consensus(msg) => {
				let call =
					contract.handleConsensus(ismp_host, Bytes::from(msg.consensus_proof));
				let gas_limit = gas_with_buffer(call.estimate_gas().await, client.state_machine);
				let calldata = call.calldata().clone();

				tx_requests.push(build_tx_request(
					from_address,
					handler_addr,
					calldata,
					gas_price_u128,
					gas_limit,
					use_gas_price,
				));
			},
			Message::Request(msg) => {
				let tx = build_post_request_tx(
					&contract,
					ismp_host,
					msg,
					client.state_machine,
					from_address,
					handler_addr,
					gas_price_u128,
					use_gas_price,
				)
				.await?;
				tx_requests.push(tx);
			},
			Message::Response(ResponseMessage { datagram, proof, .. }) => {
				let tx = build_post_response_tx(
					&contract,
					ismp_host,
					datagram,
					proof,
					client.state_machine,
					from_address,
					handler_addr,
					gas_price_u128,
					use_gas_price,
				)
				.await?;
				tx_requests.push(tx);
			},
			Message::Timeout(_) => return Err(anyhow!("Timeout messages not supported by relayer")),
			Message::FraudProof(_) =>
				return Err(anyhow!("Unexpected fraud proof message")),
		}
	}

	Ok((tx_requests, gas_price))
}

/// Resolve the gas price, applying an optional buffer from config.
async fn resolve_gas_price(client: &EvmClient, debug_trace: bool) -> anyhow::Result<U256> {
	// Gas price must be set unless we're debug-tracing on a non-Erigon client.
	// Erigon requires gas price even during tracing:
	// https://github.com/ledgerwatch/erigon/blob/cfb55a3/core/state_transition.go#L246
	let needs_gas_price = !debug_trace || client.client_type.erigon();

	if !needs_gas_price {
		return Ok(U256::zero());
	}

	let mut gas_price = get_current_gas_cost_in_usd(
		client.state_machine,
		client.config.ismp_host.0.into(),
		client.client.clone(),
	)
	.await?
	.gas_price;

	if !debug_trace {
		if let Some(buffer_bps) = client.config.gas_price_buffer {
			let buffer = (U256::from(buffer_bps) * gas_price) / U256::from(10_000u32);
			gas_price = gas_price + buffer;
		}
	}

	Ok(gas_price)
}

/// Build a `TransactionRequest` for a batch of post requests.
async fn build_post_request_tx(
	contract: &HandlerInstance<(), Arc<crate::AlloySignerProvider>>,
	ismp_host: Address,
	msg: ismp::messaging::RequestMessage,
	state_machine: StateMachine,
	from: Address,
	to: Address,
	gas_price_u128: u128,
	use_gas_price: bool,
) -> anyhow::Result<TransactionRequest> {
	let (mmr_proof, k_and_leaf_indices) =
		decode_mmr_proof_with_indices(&msg.proof.proof)?;

	let mut leaves: Vec<PostRequestLeaf> = msg
		.requests
		.into_iter()
		.zip(k_and_leaf_indices)
		.map(|(post, (k_index, leaf_index))| PostRequestLeaf {
			request: post.into(),
			index: AlloyU256::from(leaf_index),
			kIndex: AlloyU256::from(k_index),
		})
		.collect();
	leaves.sort_by(|a, b| a.index.cmp(&b.index));

	let proof = build_solidity_proof(mmr_proof, &msg.proof.height)?;
	let call =
		contract.handlePostRequests(ismp_host, PostRequestMessage { proof, requests: leaves });

	let gas_limit = gas_with_buffer(call.estimate_gas().await, state_machine);
	let calldata = call.calldata().clone();

	Ok(build_tx_request(from, to, calldata, gas_price_u128, gas_limit, use_gas_price))
}

/// Build a `TransactionRequest` for a batch of post responses.
async fn build_post_response_tx(
	contract: &HandlerInstance<(), Arc<crate::AlloySignerProvider>>,
	ismp_host: Address,
	datagram: RequestResponse,
	proof_data: ismp::messaging::Proof,
	state_machine: StateMachine,
	from: Address,
	to: Address,
	gas_price_u128: u128,
	use_gas_price: bool,
) -> anyhow::Result<TransactionRequest> {
	let RequestResponse::Response(responses) = datagram else {
		return Err(anyhow!("Get requests are not supported by relayer"));
	};

	let (mmr_proof, k_and_leaf_indices) =
		decode_mmr_proof_with_indices(&proof_data.proof)?;

	let mut leaves: Vec<PostResponseLeaf> = responses
		.into_iter()
		.zip(k_and_leaf_indices)
		.filter_map(|(res, (k_index, leaf_index))| match res {
			Response::Post(res) => Some(PostResponseLeaf {
				response: res.into(),
				index: AlloyU256::from(leaf_index),
				kIndex: AlloyU256::from(k_index),
			}),
			_ => None,
		})
		.collect();
	leaves.sort_by(|a, b| a.index.cmp(&b.index));

	let proof = build_solidity_proof(mmr_proof, &proof_data.height)?;
	let call =
		contract.handlePostResponses(ismp_host, PostResponseMessage { proof, responses: leaves });

	let gas_limit = gas_with_buffer(call.estimate_gas().await, state_machine);
	let calldata = call.calldata().clone();

	Ok(build_tx_request(from, to, calldata, gas_price_u128, gas_limit, use_gas_price))
}

/// Return the chain-specific gas limit for a given state machine.
pub fn get_chain_gas_limit(state_machine: StateMachine) -> u64 {
	match state_machine {
		StateMachine::Evm(ARBITRUM_CHAIN_ID) | StateMachine::Evm(ARBITRUM_SEPOLIA_CHAIN_ID) =>
			32_000_000,
		StateMachine::Evm(GNOSIS_CHAIN_ID) | StateMachine::Evm(CHIADO_CHAIN_ID) => 16_000_000,
		StateMachine::Evm(SEI_CHAIN_ID) | StateMachine::Evm(SEI_TESTNET_CHAIN_ID) => 4_000_000,
		StateMachine::Evm(CRONOS_CHAIN_ID) | StateMachine::Evm(CRONOS_TESTNET_CHAIN_ID) =>
			18_000_000,
		StateMachine::Evm(INJECTIVE_CHAIN_ID) | StateMachine::Evm(INJECTIVE_TESTNET_CHAIN_ID) =>
			15_000_000,
		StateMachine::Evm(_) => 16_000_000,
		_ => Default::default(),
	}
}

// ---------------------------------------------------------------------------
// Message submission pipeline
// ---------------------------------------------------------------------------

/// Entry point: submit messages and convert results into `TxResult`.
#[tracing::instrument(skip_all, fields(chain = ?client.state_machine))]
pub async fn handle_message_submission(
	client: &EvmClient,
	messages: Vec<Message>,
) -> Result<TxResult, anyhow::Error> {
	let (receipts, cancelled) = submit_messages(client, messages.clone()).await?;
	let height = client.client.get_block_number().await?;

	let results = messages
		.into_iter()
		.flat_map(|msg| collect_tx_receipts(msg, &receipts, height))
		.collect();

	Ok(TxResult { receipts: results, unsuccessful: cancelled })
}

/// Match handled commitments against the original messages to produce `TxReceipt`s.
fn collect_tx_receipts(
	msg: Message,
	receipts: &BTreeSet<H256>,
	height: u64,
) -> Vec<TxReceipt> {
	match msg {
		Message::Request(req_msg) => req_msg
			.requests
			.into_iter()
			.filter_map(|post| {
				let req = Request::Post(post);
				let commitment = hash_request::<Hasher>(&req);
				receipts.contains(&commitment).then(|| TxReceipt::Request {
					query: Query {
						source_chain: req.source_chain(),
						dest_chain: req.dest_chain(),
						nonce: req.nonce(),
						commitment,
					},
					height,
				})
			})
			.collect(),
		Message::Response(ResponseMessage {
			datagram: RequestResponse::Response(responses),
			..
		}) => responses
			.into_iter()
			.filter_map(|res| {
				let commitment = hash_response::<Hasher>(&res);
				let request_commitment = hash_request::<Hasher>(&res.request());
				receipts.contains(&commitment).then(|| TxReceipt::Response {
					query: Query {
						source_chain: res.source_chain(),
						dest_chain: res.dest_chain(),
						nonce: res.nonce(),
						commitment,
					},
					request_commitment,
					height,
				})
			})
			.collect(),
		_ => vec![],
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
	use super::*;
	use crate::AlloyProvider;

	#[tokio::test]
	#[ignore] // Requires local RPC node
	async fn test_wait_for_transaction_receipt() {
		let _ = env_logger::builder().is_test(true).try_init();

		let provider = Arc::new(AlloyProvider::new_http("http://localhost:8545".parse().unwrap()));
		let tx_hash: H256 = "0xf43c2f2910bb84fdd9f4bd94378469195d4e0b401802c6fb8d3d74a20abef3da"
			.parse()
			.expect("Failed to parse transaction hash");

		match wait_for_transaction_receipt(tx_hash, provider).await {
			Ok(Some(receipt)) => {
				println!("Transaction receipt found!");
				println!("Transaction hash: {:?}", receipt.transaction_hash);
				println!("Block number: {:?}", receipt.block_number);
				println!("Gas used: {:?}", receipt.gas_used);
				println!("Status: {:?}", receipt.status());
			},
			Ok(None) => println!("Transaction receipt not found after 5 minutes"),
			Err(err) => println!("Error fetching transaction receipt: {err:?}"),
		}
	}

	#[tokio::test]
	#[ignore] // Requires local RPC node
	async fn test_get_block() {
		let _ = env_logger::builder().is_test(true).try_init();

		let provider = Arc::new(AlloyProvider::new_http("http://localhost:8545".parse().unwrap()));
		let block_number: u64 = 4726213;

		let block: Option<alloy::rpc::types::Block> = provider
			.get_block_by_number(block_number.into())
			.full()
			.await
			.expect("Failed to fetch block");

		match block {
			Some(block) => {
				println!("Block {block_number} found!");
				println!("Hash: {:?}", block.header.hash);
				println!("Timestamp: {:?}", block.header.timestamp);
				println!("Gas used: {:?}", block.header.gas_used);
				println!("Transactions: {}", block.transactions.len());
			},
			None => panic!("Block {block_number} should exist"),
		}
	}
}
