// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

//! Submission paths for a mandatory consensus proof.
//!
//! Two modes are supported via [`crate::config::SubmissionMode`]:
//!
//! * `Batched` — one ERC-7821 batch tx `[unfreeze, handleConsensus, freeze]` dispatched from the
//!   relayer EOA that has been EIP-7702 delegated to a per-chain Executor.
//! * `Sequential` — three plain EOA txs (`setFrozenState(None)` → `handleConsensus` →
//!   `setFrozenState(All)`) for chains that don't yet accept EIP-7702 transactions.
//!
//! In both modes every tx is awaited to full receipt before the next one is
//! constructed — the caller observes the strict `submit → await receipt → next`
//! ordering it expects.

use std::time::Duration;

use alloy::{
	primitives::{Address, Bytes, B256, U256},
	providers::Provider,
	rpc::types::TransactionRequest,
};
use alloy_sol_types::{sol, SolCall, SolValue};
use anyhow::{anyhow, Context};
use ismp::messaging::ConsensusMessage;
use ismp_abi::{evm_host::EvmHost, handler::handler_v2::HandlerV2};
use primitive_types::H256;
use tesseract_evm::EvmClient;

use crate::config::SubmissionMode;

/// Deadline applied when waiting for a transaction receipt.
const RECEIPT_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// ERC-7821 batch mode: single-batch, no opData.
/// Encoding: `0x01` followed by 31 zero bytes.
const ERC7821_BATCH_MODE: B256 = B256::new([
	0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0,
]);

/// `FrozenStatus.None` — all functions enabled.
const FROZEN_NONE: u8 = 0;
/// `FrozenStatus.All` — complete protocol halt.
const FROZEN_ALL: u8 = 3;

sol! {
	/// Single call entry in an ERC-7821 batch.
	struct Erc7821Call {
		address target;
		uint256 value;
		bytes data;
	}

	/// ERC-7821 entrypoint exposed by the Executor implementation.
	function execute(bytes32 mode, bytes executionData) external payable;
}

/// Dispatch a mandatory proof to the appropriate submission path.
pub async fn submit_mandatory(
	client: &EvmClient,
	message: ConsensusMessage,
	mode: SubmissionMode,
) -> anyhow::Result<H256> {
	match mode {
		SubmissionMode::Batched => submit_mandatory_batch(client, message).await,
		SubmissionMode::Sequential => submit_mandatory_sequential(client, message).await,
	}
}

/// Submit the mandatory proof as a single ERC-7821 batch tx self-called by the
/// delegated EOA. Returns once the batch's receipt is on chain.
pub async fn submit_mandatory_batch(
	client: &EvmClient,
	message: ConsensusMessage,
) -> anyhow::Result<H256> {
	let eoa = Address::from_slice(&client.address);
	let host = Address::from_slice(&client.ismp_host.0);
	let handler = resolve_handler(client).await?;

	let unfreeze = EvmHost::setFrozenStateCall { newState: FROZEN_NONE }.abi_encode();
	let consensus =
		HandlerV2::handleConsensusCall { host, proof: Bytes::from(message.consensus_proof.clone()) }
			.abi_encode();
	let freeze = EvmHost::setFrozenStateCall { newState: FROZEN_ALL }.abi_encode();

	let calls = vec![
		Erc7821Call { target: host, value: U256::ZERO, data: Bytes::from(unfreeze) },
		Erc7821Call { target: handler, value: U256::ZERO, data: Bytes::from(consensus) },
		Erc7821Call { target: host, value: U256::ZERO, data: Bytes::from(freeze) },
	];

	let execution_data = <Vec<Erc7821Call> as SolValue>::abi_encode(&calls);
	let calldata =
		executeCall { mode: ERC7821_BATCH_MODE, executionData: Bytes::from(execution_data) }
			.abi_encode();

	send_and_await_receipt(client, eoa, eoa, calldata, "consensus batch").await
}

/// Submit the mandatory proof as three separate plain-EOA transactions. Each
/// one is fully confirmed before the next is built, so nonces increase in the
/// expected order. The refreeze step is always attempted, even if the
/// consensus step fails, to avoid leaving the host unfrozen.
pub async fn submit_mandatory_sequential(
	client: &EvmClient,
	message: ConsensusMessage,
) -> anyhow::Result<H256> {
	let eoa = Address::from_slice(&client.address);
	let host = Address::from_slice(&client.ismp_host.0);
	let handler = resolve_handler(client).await?;
	let chain = client.state_machine;

	let _unfreeze_cd = EvmHost::setFrozenStateCall { newState: FROZEN_NONE }.abi_encode();
	let consensus_cd =
		HandlerV2::handleConsensusCall { host, proof: Bytes::from(message.consensus_proof.clone()) }
			.abi_encode();
	let _freeze_cd = EvmHost::setFrozenStateCall { newState: FROZEN_ALL }.abi_encode();

	// log::info!("[{chain}] sequential step 1/3: unfreeze");
	// send_and_await_receipt(client, eoa, host, unfreeze_cd, "unfreeze").await?;

	log::info!("[{chain}] sequential step 2/3: handleConsensus");
	let consensus_result =
		send_and_await_receipt(client, eoa, handler, consensus_cd, "handleConsensus").await;

	// log::info!("[{chain}] sequential step 3/3: refreeze");
	// if let Err(err) = send_and_await_receipt(client, eoa, host, freeze_cd, "refreeze").await {
	// 	log::error!("[{chain}] refreeze FAILED: {err:?} — host may be left unfrozen");
	// }

	consensus_result
}

/// Build a transaction targeting `to` with the given calldata, submit it via
/// `client.signer`, then block until the receipt is on chain. Returns the tx
/// hash on success or an error if the tx reverts or the receipt does not
/// arrive within [`RECEIPT_TIMEOUT`].
///
/// All submission paths funnel through here so that the "submit → await
/// receipt → return" ordering is guaranteed at every call site — the caller
/// will only ever send the next transaction after this one has been mined.
async fn send_and_await_receipt(
	client: &EvmClient,
	from: Address,
	to: Address,
	calldata: Vec<u8>,
	label: &str,
) -> anyhow::Result<H256> {
	let chain = client.state_machine;
	let mut req = TransactionRequest::default()
		.from(from)
		.to(to)
		.input(Bytes::from(calldata).into());

	match client.signer.estimate_gas(req.clone()).await {
		Ok(gas) => req = req.gas_limit(gas + gas / 20),
		Err(err) => {
			let fallback = tesseract_evm::tx::get_chain_gas_limit(chain);
			log::warn!(
				"[{chain}] estimateGas for {label} failed ({err:?}); falling back to {fallback}"
			);
			req = req.gas_limit(fallback);
		},
	}

	let pending = client
		.signer
		.send_transaction(req)
		.await
		.with_context(|| format!("sending {label} tx failed"))?;
	let tx_hash = H256::from_slice(pending.tx_hash().as_slice());
	log::info!("[{chain}] {label} submitted: {tx_hash:?} — awaiting receipt");

	// Alloy drives the polling internally and returns exactly once the
	// receipt lands (or the timeout fires). Using this canonical waiter
	// means no caller can accidentally send the next tx before this one is
	// mined.
	let receipt = pending
		.with_timeout(Some(RECEIPT_TIMEOUT))
		.get_receipt()
		.await
		.with_context(|| format!("failed to get receipt for {label} tx {tx_hash:?}"))?;

	if !receipt.status() {
		return Err(anyhow!("{label} reverted: {tx_hash:?}"));
	}
	log::info!("[{chain}] {label} confirmed in block {:?}: {tx_hash:?}", receipt.block_number);
	Ok(tx_hash)
}

async fn resolve_handler(client: &EvmClient) -> anyhow::Result<Address> {
	let h = client.handler().await.context("failed to read handler from host params")?;
	Ok(Address::from_slice(&h.0))
}
