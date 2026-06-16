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
	messaging::{hash_request, Message},
	router::Request,
};
use ismp_abi::{
	evm_host::{NewEpoch, PostRequestHandled},
	handler::handler_v2::{
		HandlerV2Instance, PostRequestLeaf, PostRequestMessage, Proof, StateMachineHeight,
	},
};
use pallet_ismp::offchain::{LeafIndexAndPos, Proof as MmrProof};
use primitive_types::{H256, U256};
use std::{collections::BTreeSet, time::Duration};
use tesseract_primitives::{Hasher, NewEpochEvent, Query, TxReceipt, TxResult};

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

/// Extract post-request handled commitments from a receipt's logs.
fn extract_event_commitments(receipt: &TransactionReceipt) -> BTreeSet<H256> {
	receipt
		.inner
		.logs()
		.iter()
		.filter_map(|log| {
			PostRequestHandled::decode_log(&log.inner)
				.map(|ev| H256::from_slice(ev.commitment.as_slice()))
				.ok()
		})
		.collect()
}

/// Scan a receipt's logs for `EvmHost::NewEpoch(set_id, relayer)`
/// events addressed to `self_address`. Returns every `set_id` we won
/// the race for in this submission — a single tx can carry multiple
/// consensus messages (catch-up batches) and each one that lands a new
/// authority set on chain emits its own `NewEpoch`. Each entry in the
/// returned vec earns a separate per-chain outbound-consensus delivery
/// reward.
///
/// Each entry is paired with the receipt's `block_number` so the
/// outbound-claim task can build its storage proof at exactly the block
/// in which `_epochs[set_id]` was written. Receipts without a
/// `block_number` (shouldn't happen for mined receipts, but the alloy
/// type is `Option<u64>`) are skipped — without a block we can't make a
/// useful claim.
fn extract_new_epochs_for_self(
	receipt: &TransactionReceipt,
	self_address: &[u8],
) -> Vec<NewEpochEvent> {
	let Some(block_number) = receipt.block_number else {
		return Vec::new();
	};
	receipt
		.inner
		.logs()
		.iter()
		.filter_map(|log| {
			let ev = NewEpoch::decode_log(&log.inner).ok()?;
			if ev.relayer.as_slice() == self_address {
				// `set_id` is uint256 on chain but always fits in u64 in practice
				// (BEEFY authority set ids are monotonic counters).
				let set_id = u64::try_from(ev.authoritySetId).ok()?;
				Some(NewEpochEvent { set_id, block_number })
			} else {
				None
			}
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
		client.ismp_host.0.into(),
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
				tracing::trace!(target: crate::LOG_TARGET, "Receipt available after {:?}", start.elapsed());
				return Ok(Some(receipt));
			},
			Ok(None) =>
				tracing::trace!(target: crate::LOG_TARGET, "Receipt not yet available, retrying in 7s"),
			Err(err) =>
				tracing::warn!(target: crate::LOG_TARGET, "Error querying receipt: {err:?}"),
		}

		if tokio::time::Instant::now() >= deadline {
			tracing::error!(target: crate::LOG_TARGET, "No receipt after 5 minutes");
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
	let contract = HandlerV2Instance::new(handler_addr, client.signer.clone());
	let ismp_host = Address::from_slice(&client.ismp_host.0);
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

			Message::Response(_) =>
				return Err(anyhow!("Response messages not supported by relayer")),

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
/// `handlePostRequests`) — `batchCall` delegatecalls
/// self so those selectors still dispatch correctly against HandlerV2.
async fn build_batch_inner_calls(
	client: &EvmClient,
	messages: &[Message],
) -> anyhow::Result<Vec<Bytes>> {
	let handler_addr = Address::from_slice(&client.handler().await?.0);
	let contract = HandlerV2Instance::new(handler_addr, client.signer.clone());
	let ismp_host = Address::from_slice(&client.ismp_host.0);

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

			Message::Response(_) =>
				return Err(anyhow!("Response messages are not supported by batchCall")),

			Message::Timeout(_) =>
				return Err(anyhow!("Timeout messages are not supported by batchCall")),

			Message::FraudProof(_) =>
				return Err(anyhow!("Unexpected fraud proof message in batchCall")),
		};
		inner.push(calldata);
	}
	Ok(inner)
}

/// Build per-message `batchCall([prelude, msg_i])` [`TransactionRequest`]s —
/// used for gas estimation so each non-consensus message is traced alongside
/// the consensus update that will land with it in the same tx. Without the
/// prelude the request's proof verification would simulate against the old
/// state commitment and either underestimate or silently mis-trace.
///
/// If the prelude is `None`, falls through to [`generate_contract_calls`].
pub async fn generate_batched_contract_calls(
	client: &EvmClient,
	prelude: Option<&Message>,
	messages: &[Message],
	debug_trace: bool,
) -> anyhow::Result<(Vec<TransactionRequest>, U256)> {
	let Some(prelude) = prelude else {
		return generate_contract_calls(client, messages, debug_trace).await;
	};

	let handler_addr = Address::from_slice(&client.handler().await?.0);
	let contract = HandlerV2Instance::new(handler_addr, client.signer.clone());
	let from = Address::from_slice(&client.address);
	let gas_price = fetch_gas_price(client, debug_trace).await?;
	let chain_gas_limit = get_chain_gas_limit(client.state_machine);

	// Single-entry list so we can reuse `build_batch_inner_calls` to encode
	// the prelude into its HandlerV1-shaped calldata.
	let prelude_inner = build_batch_inner_calls(client, std::slice::from_ref(prelude)).await?;
	let prelude_calldata = prelude_inner.into_iter().next().expect("one prelude in, one out");

	let mut txs = Vec::with_capacity(messages.len());
	for msg in messages {
		// Rebuild the single-message inner calldata the same way
		// `build_batch_inner_calls` does — reuses the same encoding path the
		// real submission will use.
		let msg_inner = build_batch_inner_calls(client, std::slice::from_ref(msg)).await?;
		let msg_calldata = msg_inner.into_iter().next().expect("one msg in, one out");

		let call = contract.batchCall(vec![prelude_calldata.clone(), msg_calldata]);
		let gas = call.estimate_gas().await.unwrap_or_else(|_| (chain_gas_limit * 8) / 10);
		txs.push(build_tx_request(
			from,
			handler_addr,
			call.calldata().clone(),
			gas_price,
			gas_with_buffer(gas),
		));
	}

	Ok((txs, gas_price))
}

/// Submit a full batch of ISMP messages as a single `IHandlerV2.batchCall` transaction.
///
/// One tx replaces what would otherwise be N separate txs (one per message),
/// cutting gas overhead and nonce management complexity. Atomic: if any
/// inner call reverts, the whole transaction reverts.
pub async fn submit_batch_messages(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<SubmitOutcome> {
	if messages.is_empty() {
		return Ok((BTreeSet::new(), Vec::new(), Vec::new()));
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

	// Consensus updates and application messages share the batch — count the
	// latter separately so logs make it obvious when a tx is carrying real
	// work vs. just advancing the light client.
	let non_consensus_msgs =
		messages.iter().filter(|m| !matches!(m, Message::Consensus(_))).count();

	tracing::info!(
		target: crate::LOG_TARGET, chain = ?client.state_machine,
		msgs = messages.len(),
		non_consensus_msgs,
		calldata_bytes = calldata_len,
		gas_estimate = gas,
		nonce,
		"dispatching HandlerV2.batchCall",
	);

	// Bounded rate-limit retry. If the remote is throttling for longer than
	// MAX_RATE_LIMIT_RETRIES * 1s, give up and let the outbound task retry on
	// the next ProofAccepted event — avoids pinning a task forever.
	const MAX_RATE_LIMIT_RETRIES: u32 = 5;
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
						target: crate::LOG_TARGET, chain = ?client.state_machine,
						attempt,
						max = MAX_RATE_LIMIT_RETRIES,
						"rate limited; retrying batchCall in 1s",
					);
					tokio::time::sleep(Duration::from_secs(10)).await;
				} else {
					return Err(err);
				}
			},
		}
	};

	let tx_hash = H256::from_slice(pending.tx_hash().as_slice());
	let (events, new_epochs) = match wait_for_success(client, tx_hash).await? {
		Some(evs) => evs,
		None => {
			cancel_transaction(client, from, nonce, gas_price, tx_hash).await;
			return Err(anyhow!("batchCall to {:?} was cancelled", client.state_machine));
		},
	};
	tracing::info!(
		target: crate::LOG_TARGET, chain = ?client.state_machine,
		?tx_hash,
		events = events.len(),
		new_epochs = ?new_epochs,
		"batchCall included",
	);

	// Atomic semantics: if the tx succeeded every inner call did, so no
	// per-message unsuccessful bucket.
	Ok((events, Vec::new(), new_epochs))
}

/// Result of a single transaction submission: which message commitments
/// fired, which messages didn't, and any `NewEpoch` logs the relayer
/// should claim on Hyperbridge.
type SubmitOutcome = (BTreeSet<H256>, Vec<Message>, Vec<NewEpochEvent>);

/// Send a zero-value self-transfer at 10× gas to evict a stuck transaction from the mempool.
#[tracing::instrument(skip_all, fields(chain = ?client.state_machine))]
async fn cancel_transaction(
	client: &EvmClient,
	from: Address,
	nonce: u64,
	gas_price: U256,
	stuck_tx: H256,
) {
	tracing::warn!(target: crate::LOG_TARGET, "Cancelling stuck tx {stuck_tx:#?} at nonce {nonce}",);
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
		tracing::info!(target: crate::LOG_TARGET, "Cancellation tx for {:?} {status}", client.state_machine);
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
) -> anyhow::Result<SubmitOutcome> {
	let (tx_requests, gas_price) = generate_contract_calls(client, &messages, false).await?;

	let mut events = BTreeSet::new();
	let mut unsuccessful = Vec::new();
	let mut new_epochs: Vec<NewEpochEvent> = Vec::new();

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
						tracing::info!(target: crate::LOG_TARGET, chain = ?client.state_machine, "Rate limited, retrying submission in 1s");
						tokio::time::sleep(Duration::from_secs(1)).await;
					} else {
						return Err(err);
					}
				},
			}
		};

		let tx_hash = H256::from_slice(pending.tx_hash().as_slice());

		let (evs, epochs) = match wait_for_success(client, tx_hash).await? {
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
		new_epochs.extend(epochs);
	}

	if !events.is_empty() {
		tracing::trace!(
			target: crate::LOG_TARGET, chain = ?client.state_machine,
			"Got {} receipts",
			events.len(),
		);
	}

	Ok((events, unsuccessful, new_epochs))
}

/// Wait for a transaction to be mined and verify it succeeded.
///
/// Returns `Some((commitments, new_epochs))` on success — `commitments`
/// from `PostRequestHandled` logs, `new_epochs` from
/// every `EvmHost::NewEpoch(set_id, relayer)` log that names this
/// client as the relayer (empty when no such logs are present, multiple
/// entries when a single tx batched multiple consensus messages). Each
/// `NewEpochEvent` carries the destination block in which the log was
/// emitted, so the outbound-claim task can later prove `_epochs[set_id]`
/// at exactly that height.
/// Returns `None` on timeout and `Err` if the tx reverted.
#[tracing::instrument(skip(client), fields(chain = ?client.state_machine, ?tx_hash))]
pub async fn wait_for_success(
	client: &EvmClient,
	tx_hash: H256,
) -> anyhow::Result<Option<(BTreeSet<H256>, Vec<NewEpochEvent>)>> {
	match wait_for_transaction_receipt(tx_hash, client).await? {
		Some(receipt) =>
			if receipt.inner.status_or_post_state() == Eip658Value::Eip658(true) {
				tracing::info!(target: crate::LOG_TARGET, "Tx for {:?} succeeded", client.state_machine);
				let commitments = extract_event_commitments(&receipt);
				let new_epochs = extract_new_epochs_for_self(&receipt, &client.address);
				Ok(Some((commitments, new_epochs)))
			} else {
				tracing::info!(
					target: crate::LOG_TARGET, "Tx {:?} for {:?} reverted",
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
/// batched submission paths. `new_epochs` is propagated through verbatim
/// so the outbound layer can enqueue an outbound consensus delivery claim
/// for every set_id this submission won.
fn build_tx_receipts(
	receipts: BTreeSet<H256>,
	unsuccessful: Vec<Message>,
	messages: Vec<Message>,
	height: u64,
	new_epochs: Vec<NewEpochEvent>,
) -> TxResult {
	let mut results = vec![];
	for msg in messages {
		match msg {
			Message::Request(req_msg) =>
				for post in req_msg.requests {
					let req = Request::Post(post);
					let commitment = hash_request::<Hasher>(&req);
					if receipts.contains(&commitment) {
						results.push(TxReceipt {
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
			// `Message::Response` carries only GetRequests being responded to post-#840.
			// GetResponse delivery is on-chain via dispatch; no relayer receipt to record here.
			_ => {},
		}
	}
	TxResult { receipts: results, unsuccessful, new_epochs }
}

/// Top-level submission entry.
///
/// - **Batch of 1** (e.g. the mandatory-consensus-only chunks from the outbound rotation catch-up)
///   routes through the legacy per-message [`submit_messages`] path. Wrapping a single call in
///   `IHandlerV2.batchCall` adds a self-delegatecall frame with no upside, costs extra gas, and
///   makes the receipt harder to interpret downstream.
/// - **Batch of ≥2** dispatches through [`submit_batch_messages`], the atomic
///   `IHandlerV2.batchCall` path. Chains whose handler doesn't implement `IHandlerV2` will revert
///   at the handler address — the legacy serial-submit fallback is no longer supported for real
///   batches.
pub async fn handle_message_submission(
	client: &EvmClient,
	messages: Vec<Message>,
) -> anyhow::Result<TxResult> {
	if messages.is_empty() {
		return Ok(TxResult::default());
	}

	let (receipts, unsuccessful, new_epochs) = if messages.len() == 1 {
		submit_messages(client, messages.clone()).await?
	} else {
		submit_batch_messages(client, messages.clone()).await?
	};
	let height = client.client.get_block_number().await?;
	Ok(build_tx_receipts(receipts, unsuccessful, messages, height, new_epochs))
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
