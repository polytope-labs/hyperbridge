use crate::{
	gas_oracle::{
		ARBITRUM_CHAIN_ID, ARBITRUM_SEPOLIA_CHAIN_ID, CHIADO_CHAIN_ID, CRONOS_CHAIN_ID,
		CRONOS_TESTNET_CHAIN_ID, GNOSIS_CHAIN_ID, INJECTIVE_CHAIN_ID, INJECTIVE_TESTNET_CHAIN_ID,
		SEI_CHAIN_ID, SEI_TESTNET_CHAIN_ID,
	},
	EvmClient,
};
use alloy::{
	consensus::{Eip658Value, TxReceipt as AlloyTxReceipt},
	primitives::{Address, Bytes, FixedBytes, B256, U256 as AlloyU256},
	providers::Provider,
	rpc::types::{TransactionReceipt, TransactionRequest},
	transports::TransportError,
};
use alloy_sol_types::{SolCall, SolEvent};
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
		handler_v2::{batchCallCall, HandlerV2Instance},
		HandlerInstance, PostRequestLeaf, PostRequestMessage, PostResponseLeaf,
		PostResponseMessage, Proof, StateMachineHeight,
	},
};
use pallet_ismp::offchain::{LeafIndexAndPos, Proof as MmrProof};
use primitive_types::{H256, U256};
use std::{collections::BTreeSet, time::Duration};
use tesseract_primitives::{Hasher, Query, TxReceipt, TxResult};

/// ERC-165 interface id for `IHandlerV2`. Since the interface contains only
/// `batchCall(bytes[])`, the id equals that function's 4-byte selector.
const IHANDLER_V2_INTERFACE_ID: FixedBytes<4> = FixedBytes::new(batchCallCall::SELECTOR);

use crate::gas_oracle::get_current_gas_cost_in_usd;

// ── Pure helpers ──────────────────────────────────────────────────────────────

/// Check if an error is a rate limit (429) or other retryable RPC error.
fn is_rate_limit_error(err: &anyhow::Error) -> bool {
	if let Some(transport_err) = err.downcast_ref::<TransportError>() {
		return match transport_err {
			TransportError::Transport(kind) => kind.is_retry_err(),
			TransportError::ErrorResp(payload) => payload.is_retry_err(),
			_ => false,
		};
	}
	format!("{err:?}").contains("429")
}

/// Extract the numeric state machine ID, expecting Polkadot or Kusama.
fn extract_state_machine_id(state_id: &StateMachine) -> anyhow::Result<AlloyU256> {
	match state_id {
		StateMachine::Polkadot(id) | StateMachine::Kusama(id) => Ok(AlloyU256::from(*id)),
		other => Err(anyhow!("Expected Polkadot or Kusama state machine, got: {other:?}")),
	}
}

/// Decode a raw MMR proof and extract the leaf indices.
fn decode_mmr_proof(raw: &[u8]) -> anyhow::Result<(MmrProof<H256>, Vec<u64>)> {
	let proof = MmrProof::<H256>::decode(&mut &*raw)?;
	let leaf_indices = proof
		.leaf_indices_and_pos
		.iter()
		.map(|LeafIndexAndPos { leaf_index, .. }| *leaf_index)
		.collect();
	Ok((proof, leaf_indices))
}

/// Build the solidity `Proof` struct from an MMR proof and ISMP height.
fn build_solidity_proof(
	mmr_proof: &MmrProof<H256>,
	height: &ismp::consensus::StateMachineHeight,
) -> anyhow::Result<Proof> {
	Ok(Proof {
		height: StateMachineHeight {
			stateMachineId: extract_state_machine_id(&height.id.state_id)?,
			height: AlloyU256::from(height.height),
		},
		multiproof: mmr_proof.items.iter().map(|node| B256::from_slice(&node.0)).collect(),
		leafCount: AlloyU256::from(mmr_proof.leaf_count),
	})
}

/// Extract post-request and post-response handled commitments from a receipt's logs.
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

/// Add a 5% buffer to a gas estimate.
fn gas_with_buffer(estimated: u64) -> u64 {
	estimated + (estimated * 5 / 100)
}

/// Build a `TransactionRequest`, optionally setting gas price.
fn build_tx_request(
	from: Address,
	to: Address,
	calldata: Bytes,
	gas_price: U256,
	gas_limit: u64,
) -> TransactionRequest {
	let base = TransactionRequest::default()
		.from(from)
		.to(to)
		.input(calldata.into())
		.gas_limit(gas_limit);
	if gas_price.is_zero() {
		base
	} else {
		base.gas_price(gas_price.low_u128())
	}
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

// ── Gas price ─────────────────────────────────────────────────────────────────

/// Fetch the current gas price, applying an optional buffer from config.
///
/// In debug-trace mode, gas price is omitted unless the client is Erigon (which requires it even
/// during tracing: https://github.com/ledgerwatch/erigon/blob/cfb55a3/core/state_transition.go#L246).
#[tracing::instrument(skip(client, debug_trace), fields(chain = ?client.state_machine))]
async fn fetch_gas_price(client: &EvmClient, debug_trace: bool) -> anyhow::Result<U256> {
	if debug_trace && !client.client_type.erigon() {
		return Ok(U256::zero());
	}

	let mut price = get_current_gas_cost_in_usd(
		client.state_machine,
		client.config.ismp_host.0.into(),
		client.client.clone(),
	)
	.await?
	.gas_price;

	if !debug_trace {
		if let Some(bps) = client.config.gas_price_buffer {
			price = price + (U256::from(bps) * price) / U256::from(10_000u32);
		}
	}

	Ok(price)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Poll for a transaction receipt, retrying every 7 seconds for up to 5 minutes.
pub async fn wait_for_transaction_receipt(
	tx_hash: H256,
	client: &EvmClient,
) -> anyhow::Result<Option<TransactionReceipt>> {
	let provider = client.client.clone();
	let poll_interval = Duration::from_secs(7);
	let start = tokio::time::Instant::now();
	let deadline = start + Duration::from_secs(5 * 60);

	loop {
		match provider.get_transaction_receipt(B256::from_slice(&tx_hash.0)).await {
			Ok(Some(receipt)) => {
				tracing::trace!(target: "messaging-evm", "Receipt available after {:?}", start.elapsed());
				return Ok(Some(receipt));
			},
			Ok(None) =>
				tracing::trace!(target: "messaging-evm", "Receipt not yet available, retrying in 7s"),
			Err(err) => tracing::warn!(target: "messaging-evm", "Error querying receipt: {err:?}"),
		}

		if tokio::time::Instant::now() >= deadline {
			tracing::error!(target: "messaging-evm", "No receipt after 5 minutes");
			return Ok(None);
		}

		tokio::time::sleep(poll_interval).await;
	}
}

/// Build unsigned `TransactionRequest`s for a batch of ISMP messages.
///
/// Returns the requests and the gas price used (needed for cancellation bumping).
/// Pass `debug_trace = true` to skip gas price (except on Erigon).
pub async fn generate_contract_calls(
	client: &EvmClient,
	messages: &[Message],
	debug_trace: bool,
) -> anyhow::Result<(Vec<TransactionRequest>, U256)> {
	let handler_addr = Address::from_slice(&client.handler().await?.0);
	let contract = HandlerInstance::new(handler_addr, client.signer.clone());
	let ismp_host = Address::from_slice(&client.config.ismp_host.0);
	let from = Address::from_slice(&client.address);
	let gas_price = fetch_gas_price(client, debug_trace).await?;
	let chain_gas_limit = get_chain_gas_limit(client.state_machine);

	let mut txs = Vec::with_capacity(messages.len());

	for msg in messages {
		let (calldata, gas_limit) = match msg {
			Message::Consensus(msg) => {
				let call =
					contract.handleConsensus(ismp_host, Bytes::from(msg.consensus_proof.clone()));
				let gas = call.estimate_gas().await.unwrap_or(chain_gas_limit / 4);
				(call.calldata().clone(), gas_with_buffer(gas))
			},

			Message::Request(msg) => {
				let (mmr_proof, leaf_indices) = decode_mmr_proof(&msg.proof.proof)?;
				let mut leaves: Vec<PostRequestLeaf> = msg
					.requests
					.iter()
					.zip(&leaf_indices)
					.map(|(post, &leaf_index)| PostRequestLeaf {
						request: post.clone().into(),
						index: AlloyU256::from(leaf_index),
					})
					.collect();
				leaves.sort_by_key(|l| l.index);
				let proof = build_solidity_proof(&mmr_proof, &msg.proof.height)?;
				let call = contract
					.handlePostRequests(ismp_host, PostRequestMessage { proof, requests: leaves });
				let gas = call.estimate_gas().await.unwrap_or_else(|_| chain_gas_limit / 4);
				(call.calldata().clone(), gas_with_buffer(gas))
			},

			Message::Response(ResponseMessage {
				datagram: RequestResponse::Response(responses),
				proof,
				..
			}) => {
				let (mmr_proof, leaf_indices) = decode_mmr_proof(&proof.proof)?;
				let mut leaves: Vec<PostResponseLeaf> = responses
					.iter()
					.zip(&leaf_indices)
					.filter_map(|(res, &leaf_index)| match res {
						Response::Post(res) => Some(PostResponseLeaf {
							response: res.clone().into(),
							index: AlloyU256::from(leaf_index),
						}),
						_ => None,
					})
					.collect();
				leaves.sort_by_key(|l| l.index);
				let solidity_proof = build_solidity_proof(&mmr_proof, &proof.height)?;
				let call = contract.handlePostResponses(
					ismp_host,
					PostResponseMessage { proof: solidity_proof, responses: leaves },
				);
				let gas = call.estimate_gas().await.unwrap_or_else(|_| chain_gas_limit / 4);
				(call.calldata().clone(), gas_with_buffer(gas))
			},

			Message::Response(ResponseMessage {
				datagram: RequestResponse::Request(..), ..
			}) => return Err(anyhow!("Get requests are not supported by relayer")),

			Message::Timeout(_) => return Err(anyhow!("Timeout messages not supported by relayer")),

			Message::FraudProof(_) => return Err(anyhow!("Unexpected fraud proof message")),
		};

		txs.push(build_tx_request(from, handler_addr, calldata, gas_price, gas_limit));
	}

	Ok((txs, gas_price))
}

/// Build the per-message inner calldata array for an `IHandlerV2.batchCall`.
///
/// Each entry is an ABI-encoded HandlerV1 call (`handleConsensus`,
/// `handlePostRequests`, `handlePostResponses`) — `batchCall` delegatecalls
/// self so those selectors still dispatch correctly against HandlerV2.
async fn build_batch_inner_calls(
	client: &EvmClient,
	messages: &[Message],
) -> anyhow::Result<Vec<Bytes>> {
	let handler_addr = Address::from_slice(&client.handler().await?.0);
	let contract = HandlerInstance::new(handler_addr, client.signer.clone());
	let ismp_host = Address::from_slice(&client.config.ismp_host.0);

	let mut inner = Vec::with_capacity(messages.len());
	for msg in messages {
		let calldata = match msg {
			Message::Consensus(msg) => contract
				.handleConsensus(ismp_host, Bytes::from(msg.consensus_proof.clone()))
				.calldata()
				.clone(),

			Message::Request(msg) => {
				let (mmr_proof, leaf_indices) = decode_mmr_proof(&msg.proof.proof)?;
				let mut leaves: Vec<PostRequestLeaf> = msg
					.requests
					.iter()
					.zip(&leaf_indices)
					.map(|(post, &leaf_index)| PostRequestLeaf {
						request: post.clone().into(),
						index: AlloyU256::from(leaf_index),
					})
					.collect();
				leaves.sort_by_key(|l| l.index);
				let proof = build_solidity_proof(&mmr_proof, &msg.proof.height)?;
				contract
					.handlePostRequests(ismp_host, PostRequestMessage { proof, requests: leaves })
					.calldata()
					.clone()
			},

			Message::Response(ResponseMessage {
				datagram: RequestResponse::Response(responses),
				proof,
				..
			}) => {
				let (mmr_proof, leaf_indices) = decode_mmr_proof(&proof.proof)?;
				let mut leaves: Vec<PostResponseLeaf> = responses
					.iter()
					.zip(&leaf_indices)
					.filter_map(|(res, &leaf_index)| match res {
						Response::Post(res) => Some(PostResponseLeaf {
							response: res.clone().into(),
							index: AlloyU256::from(leaf_index),
						}),
						_ => None,
					})
					.collect();
				leaves.sort_by_key(|l| l.index);
				let solidity_proof = build_solidity_proof(&mmr_proof, &proof.height)?;
				contract
					.handlePostResponses(
						ismp_host,
						PostResponseMessage { proof: solidity_proof, responses: leaves },
					)
					.calldata()
					.clone()
			},

			Message::Response(ResponseMessage {
				datagram: RequestResponse::Request(..), ..
			}) => return Err(anyhow!("Get requests are not supported by batchCall")),

			Message::Timeout(_) =>
				return Err(anyhow!("Timeout messages are not supported by batchCall")),

			Message::FraudProof(_) =>
				return Err(anyhow!("Unexpected fraud proof message in batchCall")),
		};
		inner.push(calldata);
	}
	Ok(inner)
}

/// Submit a full batch of ISMP messages as a single `IHandlerV2.batchCall` transaction.
///
/// One tx replaces what would otherwise be N separate txs (one per message),
/// cutting gas overhead and nonce management complexity. Atomic: if any
/// inner call reverts, the whole transaction reverts.
pub async fn submit_batch_messages(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<(BTreeSet<H256>, Vec<Message>)> {
	if messages.is_empty() {
		return Ok((BTreeSet::new(), Vec::new()));
	}

	let handler_addr = Address::from_slice(&client.handler().await?.0);
	let from = Address::from_slice(&client.address);
	let gas_price = fetch_gas_price(client, false).await?;
	let chain_gas_limit = get_chain_gas_limit(client.state_machine);

	let inner_calls = build_batch_inner_calls(client, &messages).await?;
	let handler_v2 = HandlerV2Instance::new(handler_addr, client.signer.clone());
	let call = handler_v2.batchCall(inner_calls);
	let gas = call.estimate_gas().await.unwrap_or_else(|_| (chain_gas_limit * 8) / 10);
	let calldata = call.calldata().clone();
	let calldata_len = calldata.len();
	let tx_request =
		build_tx_request(from, handler_addr, calldata, gas_price, gas_with_buffer(gas));

	let nonce = client.signer.get_transaction_count(from).await?;
	let tx = tx_request.nonce(nonce).transaction_type(0);

	tracing::info!(
		target: "messaging-evm", chain = ?client.state_machine,
		msgs = messages.len(),
		calldata_bytes = calldata_len,
		gas_estimate = gas,
		nonce,
		"dispatching HandlerV2.batchCall",
	);

	// Bounded rate-limit retry. If the remote is throttling for longer than
	// MAX_RATE_LIMIT_RETRIES * 1s, give up and let the outbound task retry on
	// the next ProofAccepted event — avoids pinning a task forever.
	const MAX_RATE_LIMIT_RETRIES: u32 = 10;
	let mut attempt = 0u32;
	let pending = loop {
		match client.signer.send_transaction(tx.clone()).await {
			Ok(p) => break p,
			Err(err) => {
				let err = anyhow::Error::from(err);
				if is_rate_limit_error(&err) {
					attempt += 1;
					if attempt > MAX_RATE_LIMIT_RETRIES {
						return Err(anyhow!(
							"batchCall to {:?} exceeded {} rate-limit retries",
							client.state_machine,
							MAX_RATE_LIMIT_RETRIES,
						));
					}
					tracing::info!(
						target: "messaging-evm", chain = ?client.state_machine,
						attempt,
						max = MAX_RATE_LIMIT_RETRIES,
						"rate limited; retrying batchCall in 1s",
					);
					tokio::time::sleep(Duration::from_secs(1)).await;
				} else {
					return Err(err);
				}
			},
		}
	};

	let tx_hash = H256::from_slice(pending.tx_hash().as_slice());
	let events = match wait_for_success(client, tx_hash).await? {
		Some(evs) => evs,
		None => {
			cancel_transaction(client, from, nonce, gas_price, tx_hash).await;
			return Err(anyhow!("batchCall to {:?} was cancelled", client.state_machine));
		},
	};
	tracing::info!(
		target: "messaging-evm", chain = ?client.state_machine,
		?tx_hash,
		events = events.len(),
		"batchCall included",
	);

	// Atomic semantics: if the tx succeeded every inner call did, so no
	// per-message unsuccessful bucket.
	Ok((events, Vec::new()))
}

/// Send a zero-value self-transfer at 10× gas to evict a stuck transaction from the mempool.
#[tracing::instrument(skip_all, fields(chain = ?client.state_machine))]
async fn cancel_transaction(
	client: &EvmClient,
	from: Address,
	nonce: u64,
	gas_price: U256,
	stuck_tx: H256,
) {
	tracing::warn!(target: "messaging-evm", "Cancelling stuck tx {stuck_tx:#?} at nonce {nonce}",);
	let cancel_tx = TransactionRequest::default()
		.to(from)
		.value(AlloyU256::ZERO)
		.gas_price((gas_price * U256::from(10)).low_u128())
		.nonce(nonce)
		.transaction_type(0);

	let Ok(pending) = client.signer.send_transaction(cancel_tx).await else { return };
	let cancel_hash = H256::from_slice(pending.tx_hash().as_slice());

	if let Ok(Some(receipt)) = wait_for_transaction_receipt(cancel_hash, client).await {
		let status = if receipt.inner.status_or_post_state() == Eip658Value::Eip658(true) {
			"succeeded"
		} else {
			"reverted"
		};
		tracing::info!(target: "messaging-evm", "Cancellation tx for {:?} {status}", client.state_machine);
	}
}

/// Submit ISMP messages as EVM transactions.
///
/// Retries individual sends on rate-limit errors. On timeout, consensus messages are retried once
/// with 2× gas on the same nonce; all other messages are cancelled with a zero-value self-transfer
/// on the same nonce so the sequence slot is freed for the next round.
pub async fn submit_messages(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<(BTreeSet<H256>, Vec<Message>)> {
	let (tx_requests, gas_price) = generate_contract_calls(client, &messages, false).await?;

	let mut events = BTreeSet::new();
	let mut unsuccessful = Vec::new();

	let from = Address::from_slice(&client.address);

	for (idx, tx) in tx_requests.into_iter().enumerate() {
		// Fetch the pending nonce upfront so we can reuse it if the tx gets stuck.
		let nonce = client.signer.get_transaction_count(from).await?;
		let tx = tx.nonce(nonce).transaction_type(0);

		// Retry the send on rate limits.
		let pending = loop {
			match client.signer.send_transaction(tx.clone()).await {
				Ok(p) => break p,
				Err(err) => {
					let err = anyhow::Error::from(err);
					if is_rate_limit_error(&err) {
						tracing::info!(target: "messaging-evm", chain = ?client.state_machine, "Rate limited, retrying submission in 1s");
						tokio::time::sleep(Duration::from_secs(1)).await;
					} else {
						return Err(err);
					}
				},
			}
		};

		let tx_hash = H256::from_slice(pending.tx_hash().as_slice());

		let evs = match wait_for_success(client, tx_hash).await? {
			Some(evs) => evs,
			None => {
				cancel_transaction(client, from, nonce, gas_price, tx_hash).await;
				return Err(anyhow!("Transaction to {:?} was cancelled", client.state_machine));
			},
		};

		if matches!(messages[idx], Message::Request(_) | Message::Response(_)) && evs.is_empty() {
			unsuccessful.push(messages[idx].clone());
		}
		events.extend(evs);
	}

	if !events.is_empty() {
		tracing::trace!(
			target: "messaging-evm", chain = ?client.state_machine,
			"Got {} receipts",
			events.len(),
		);
	}

	Ok((events, unsuccessful))
}

/// Wait for a transaction to be mined and verify it succeeded.
///
/// Returns `Some(commitments)` on success, `None` on timeout, and `Err` if the tx reverted.
#[tracing::instrument(skip(client), fields(chain = ?client.state_machine, ?tx_hash))]
pub async fn wait_for_success(
	client: &EvmClient,
	tx_hash: H256,
) -> anyhow::Result<Option<BTreeSet<H256>>> {
	match wait_for_transaction_receipt(tx_hash, client).await? {
		Some(receipt) =>
			if receipt.inner.status_or_post_state() == Eip658Value::Eip658(true) {
				tracing::info!(target: "messaging-evm", "Tx for {:?} succeeded", client.state_machine);
				Ok(Some(extract_event_commitments(&receipt)))
			} else {
				tracing::info!(
					target: "messaging-evm", "Tx {:?} for {:?} reverted",
					receipt.transaction_hash,
					client.state_machine
				);
				Err(anyhow!("Transaction reverted"))
			},
		None => Ok(None),
	}
}

/// Build `TxReceipt`s for every message in `messages` whose commitment
/// appears in the on-chain event set `receipts`. Shared between serial and
/// batched submission paths.
fn build_tx_receipts(
	receipts: BTreeSet<H256>,
	unsuccessful: Vec<Message>,
	messages: Vec<Message>,
	height: u64,
) -> TxResult {
	let mut results = vec![];
	for msg in messages {
		match msg {
			Message::Request(req_msg) =>
				for post in req_msg.requests {
					let req = Request::Post(post);
					let commitment = hash_request::<Hasher>(&req);
					if receipts.contains(&commitment) {
						results.push(TxReceipt::Request {
							query: Query {
								source_chain: req.source_chain(),
								dest_chain: req.dest_chain(),
								nonce: req.nonce(),
								commitment,
							},
							height,
						});
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
						results.push(TxReceipt::Response {
							query: Query {
								source_chain: res.source_chain(),
								dest_chain: res.dest_chain(),
								nonce: res.nonce(),
								commitment,
							},
							request_commitment,
							height,
						});
					}
				},
			_ => {},
		}
	}
	TxResult { receipts: results, unsuccessful }
}

/// Probe (via ERC-165 `supportsInterface`) whether this chain's handler
/// contract implements `IHandlerV2` and therefore supports `batchCall`.
///
/// Returns `false` on any failure (pre-ERC165 handler, RPC error, etc.) —
/// the caller falls back to the serial submit path. Result should be cached
/// on the client; see `EvmClient::supports_batch`.
pub async fn probe_handler_supports_batch(client: &EvmClient) -> bool {
	let handler_addr = match client.handler().await {
		Ok(h) => Address::from_slice(&h.0),
		Err(err) => {
			tracing::debug!(target: "messaging-evm", ?err, "handler address lookup failed during batch probe");
			return false;
		},
	};
	let handler = HandlerInstance::new(handler_addr, client.signer.clone());
	match handler.supportsInterface(IHANDLER_V2_INTERFACE_ID).call().await {
		Ok(supported) => {
			tracing::debug!(
				target: "messaging-evm", chain = ?client.state_machine,
				%handler_addr,
				supported,
				"IHandlerV2 supportsInterface probe",
			);
			supported
		},
		Err(err) => {
			tracing::debug!(
				target: "messaging-evm", chain = ?client.state_machine,
				%handler_addr,
				?err,
				"supportsInterface call failed; assuming no IHandlerV2",
			);
			false
		},
	}
}

/// Top-level submission entry. Transparently dispatches through either
/// `IHandlerV2.batchCall` (one tx for the whole batch) or the legacy
/// one-tx-per-message path, depending on what the chain's handler supports.
///
/// The V2 capability is probed once via ERC-165 `supportsInterface` and cached
/// on the client ([`EvmClient::supports_batch`]), so subsequent submissions
/// skip the probe.
pub async fn handle_message_submission(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<TxResult> {
	if messages.is_empty() {
		return Ok(TxResult::default());
	}

	let supports_batch = match client.supports_batch.get() {
		Some(cached) => *cached,
		None => {
			let probed = probe_handler_supports_batch(client).await;
			let _ = client.supports_batch.set(probed);
			probed
		},
	};

	let (receipts, unsuccessful) = if supports_batch {
		submit_batch_messages(client, messages.clone()).await?
	} else {
		tracing::debug!(
			target: "messaging-evm", chain = ?client.state_machine,
			msgs = messages.len(),
			"handler doesn't support IHandlerV2; using serial submit",
		);
		submit_messages(client, messages.clone()).await?
	};
	let height = client.client.get_block_number().await?;
	Ok(build_tx_receipts(receipts, unsuccessful, messages, height))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::AlloyProvider;
	use std::sync::Arc;

	#[tokio::test]
	#[ignore] // Requires local RPC node
	async fn test_get_block() {
		let _ = env_logger::builder().is_test(true).try_init();

		let provider = Arc::new(AlloyProvider::new_http("http://localhost:8545".parse().unwrap()));

		let block_number: u64 = 4726213;
		println!("Fetching block {block_number}...");

		let block: Option<alloy::rpc::types::Block> = provider
			.get_block_by_number(block_number.into())
			.full()
			.await
			.expect("Failed to fetch block");

		match block {
			Some(block) => {
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
			None => panic!("Block {block_number} not found"),
		}
	}
}
