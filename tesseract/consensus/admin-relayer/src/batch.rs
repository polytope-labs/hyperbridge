// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

//! Build and submit the ERC-7821 batch transaction that atomically:
//!
//!   1. unfreezes the ISMP host,
//!   2. submits the consensus update via `Handler.handleConsensus`,
//!   3. re-freezes the host.
//!
//! The batch is dispatched from the relayer EOA, which has been delegated to an
//! ERC-7821 Executor via EIP-7702 (see [`crate::delegation`]).

use std::time::Duration;

use alloy::{
	primitives::{Address, Bytes, B256, U256},
	providers::Provider,
	rpc::types::TransactionRequest,
};
use alloy_sol_types::{sol, SolCall, SolValue};
use anyhow::{anyhow, Context};
use ismp::messaging::ConsensusMessage;
use ismp_solidity_abi::{evm_host::EvmHost, handler::Handler};
use primitive_types::H256;
use tesseract_evm::EvmClient;

use crate::config::SubmissionMode;

/// ERC-7821 batch mode: single-batch, no opData.
/// Encoding: `0x01 00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00`.
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

/// Submit a single mandatory consensus proof as an ERC-7821 batch
/// (unfreeze → handleConsensus → freeze) from the delegated EOA.
///
/// Blocks until the transaction receipt is available; returns the tx hash on
/// success or an error if the tx reverts or times out.
pub async fn submit_mandatory_batch(
	client: &EvmClient,
	message: ConsensusMessage,
) -> anyhow::Result<H256> {
	let eoa = Address::from_slice(&client.address);
	let host = Address::from_slice(&client.config.ismp_host.0);
	let handler = {
		let h = client.handler().await.context("failed to read handler from host params")?;
		Address::from_slice(&h.0)
	};

	let unfreeze = EvmHost::setFrozenStateCall { newState: FROZEN_NONE }.abi_encode();
	let consensus = Handler::handleConsensusCall {
		host,
		proof: Bytes::from(message.consensus_proof.clone()),
	}
	.abi_encode();
	let freeze = EvmHost::setFrozenStateCall { newState: FROZEN_ALL }.abi_encode();

	let calls = vec![
		Erc7821Call { target: host, value: U256::ZERO, data: Bytes::from(unfreeze) },
		Erc7821Call { target: handler, value: U256::ZERO, data: Bytes::from(consensus) },
		Erc7821Call { target: host, value: U256::ZERO, data: Bytes::from(freeze) },
	];

	// `bytes executionData` = abi.encode(Call[])
	let execution_data = <Vec<Erc7821Call> as SolValue>::abi_encode(&calls);

	let calldata = executeCall {
		mode: ERC7821_BATCH_MODE,
		executionData: Bytes::from(execution_data),
	}
	.abi_encode();

	let mut req = TransactionRequest::default()
		.from(eoa)
		.to(eoa)
		.input(Bytes::from(calldata).into());

	match client.signer.estimate_gas(req.clone()).await {
		Ok(gas) => {
			let bumped = gas + gas / 20; // +5% buffer
			req = req.gas_limit(bumped);
		},
		Err(err) => {
			let fallback = tesseract_evm::tx::get_chain_gas_limit(client.state_machine);
			log::warn!(
				"[{}] estimateGas for consensus batch failed ({err:?}); falling back to {fallback}",
				client.state_machine
			);
			req = req.gas_limit(fallback);
		},
	}

	let pending =
		client.signer.send_transaction(req).await.context("sending batch tx failed")?;
	let tx_hash = H256::from_slice(pending.tx_hash().as_slice());
	log::info!("[{}] consensus batch submitted: {tx_hash:?}", client.state_machine);

	let deadline = tokio::time::Instant::now() + Duration::from_secs(5 * 60);
	loop {
		if tokio::time::Instant::now() >= deadline {
			return Err(anyhow!("timed out waiting for batch tx {tx_hash:?}"));
		}
		if let Some(receipt) =
			client.client.get_transaction_receipt(pending.tx_hash().clone()).await?
		{
			if !receipt.status() {
				return Err(anyhow!("consensus batch reverted: {tx_hash:?}"));
			}
			log::info!(
				"[{}] consensus batch confirmed in block {:?}",
				client.state_machine,
				receipt.block_number
			);
			return Ok(tx_hash);
		}
		tokio::time::sleep(Duration::from_secs(5)).await;
	}
}

/// Submit a mandatory proof on a chain whose RPC does not yet accept EIP-7702
/// transactions. Sends three independent txs from a plain EOA:
/// `setFrozenState(None)` → `handleConsensus` → `setFrozenState(All)`.
///
/// We attempt the refreeze *always* (even if the consensus call reverts) so the
/// host is not left unfrozen, then return based on the consensus step's result.
pub async fn submit_mandatory_sequential(
	client: &EvmClient,
	message: ConsensusMessage,
) -> anyhow::Result<H256> {
	let eoa = Address::from_slice(&client.address);
	let host = Address::from_slice(&client.config.ismp_host.0);
	let handler = {
		let h = client.handler().await.context("failed to read handler from host params")?;
		Address::from_slice(&h.0)
	};
	let chain = client.state_machine;

	let unfreeze_cd = EvmHost::setFrozenStateCall { newState: FROZEN_NONE }.abi_encode();
	let consensus_cd = Handler::handleConsensusCall {
		host,
		proof: Bytes::from(message.consensus_proof.clone()),
	}
	.abi_encode();
	let freeze_cd = EvmHost::setFrozenStateCall { newState: FROZEN_ALL }.abi_encode();

	// 1. Unfreeze
	log::info!("[{chain}] sequential step 1/3: unfreeze host");
	send_and_wait(client, eoa, host, unfreeze_cd, "unfreeze").await?;

	// 2. Submit consensus update — capture error without early return so we always refreeze
	log::info!("[{chain}] sequential step 2/3: handleConsensus");
	let consensus_result = send_and_wait(client, eoa, handler, consensus_cd, "handleConsensus").await;

	// 3. Refreeze (best effort, always attempted)
	log::info!("[{chain}] sequential step 3/3: refreeze host");
	match send_and_wait(client, eoa, host, freeze_cd, "refreeze").await {
		Ok(tx) => log::info!("[{chain}] refreeze confirmed: {tx:?}"),
		Err(err) => log::error!("[{chain}] refreeze FAILED: {err:?} — host may be left unfrozen"),
	}

	consensus_result
}

/// Encode + estimate + send one step of the sequential flow, wait for the receipt.
async fn send_and_wait(
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
	log::info!("[{chain}] {label} tx submitted: {tx_hash:?}");

	let deadline = tokio::time::Instant::now() + Duration::from_secs(5 * 60);
	loop {
		if tokio::time::Instant::now() >= deadline {
			return Err(anyhow!("timed out waiting for {label} tx {tx_hash:?}"));
		}
		if let Some(receipt) =
			client.client.get_transaction_receipt(pending.tx_hash().clone()).await?
		{
			if !receipt.status() {
				return Err(anyhow!("{label} reverted: {tx_hash:?}"));
			}
			log::info!(
				"[{chain}] {label} confirmed in block {:?}",
				receipt.block_number
			);
			return Ok(tx_hash);
		}
		tokio::time::sleep(Duration::from_secs(5)).await;
	}
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
