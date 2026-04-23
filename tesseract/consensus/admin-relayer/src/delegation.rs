// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

//! EIP-7702 self-delegation for the relayer's signing EOA.
//!
//! On startup the relayer inspects the on-chain code of its EOA on each configured
//! EVM chain. If that code does not begin with the EIP-7702 delegation indicator
//! `0xef0100 || <target>`, the relayer submits a type-0x04 self-authorization
//! transaction pointing the EOA at the per-chain delegation target.

use std::time::Duration;

use alloy::{
	eips::eip7702::Authorization,
	primitives::{Address, B256, U256},
	providers::Provider,
	rpc::types::TransactionRequest,
	signers::SignerSync,
};
use anyhow::{anyhow, Context};
use primitive_types::{H160, H256};
use tesseract_evm::EvmClient;

/// EIP-7702 delegation indicator prefix `0xef0100`.
const DELEGATION_INDICATOR: [u8; 3] = [0xef, 0x01, 0x00];

/// Floor gas for type-0x04 txs (some L2s reject alloy's default).
const DELEGATION_TX_GAS_FLOOR: u64 = 350_000;

/// Resolve the per-chain ERC-7821 delegation target address.
///
/// The mapping comes from the operator spec:
/// * Ethereum, Arbitrum, Optimism, BSC, Base, Polygon, Unichain →
///   `0x66C4459fa61E5Ca647152EEb6dA56150EE975512`
/// * Soneium (chain id 1868) → `0xEa392DfbcC405495E2a17398b1Cf97979Bc40b5e`
/// * Gnosis  (chain id 100)  → `0xd4d594C99f23b1Fb9d65fdd9062854B1A1C5780b`
pub fn delegation_target(chain_id: u64) -> anyhow::Result<Address> {
	// known "default" chains that share one target
	const DEFAULT_CHAINS: &[u64] = &[
		1,     // Ethereum
		10,    // Optimism
		56,    // BSC
		130,   // Unichain
		137,   // Polygon
		8453,  // Base
		42161, // Arbitrum
	];

	let hex_addr = if DEFAULT_CHAINS.contains(&chain_id) {
		"0x66C4459fa61E5Ca647152EEb6dA56150EE975512"
	} else if chain_id == 1868 {
		"0xEa392DfbcC405495E2a17398b1Cf97979Bc40b5e"
	} else if chain_id == 100 {
		"0xd4d594C99f23b1Fb9d65fdd9062854B1A1C5780b"
	} else {
		return Err(anyhow!("no ERC-7821 delegation target is configured for chain id {chain_id}"));
	};

	hex_addr
		.parse::<Address>()
		.map_err(|e| anyhow!("invalid delegation address literal: {e}"))
}

/// Check whether `eoa` is already delegated to `target` via EIP-7702 on the chain
/// reachable through `client`.
pub async fn is_delegated(
	client: &EvmClient,
	eoa: Address,
	target: Address,
) -> anyhow::Result<bool> {
	let code = client.client.get_code_at(eoa).await.context("eth_getCode failed")?;
	let bytes = code.as_ref();

	if bytes.len() != DELEGATION_INDICATOR.len() + 20 {
		return Ok(false);
	}
	if &bytes[..DELEGATION_INDICATOR.len()] != DELEGATION_INDICATOR {
		return Ok(false);
	}

	let delegated_to = Address::from_slice(&bytes[DELEGATION_INDICATOR.len()..]);
	Ok(delegated_to == target)
}

/// Ensure the EOA on the given chain is EIP-7702 delegated to the spec-defined
/// target. No-op if the delegation is already in place.
pub async fn ensure_delegated(client: &EvmClient) -> anyhow::Result<()> {
	let eoa = Address::from_slice(&client.address);
	let chain_id = client.chain_id;
	let chain = client.state_machine;
	let target = delegation_target(chain_id)?;

	if is_delegated(client, eoa, target).await? {
		log::info!("[{chain}] signer {eoa:?} already delegated to {target:?}");
		return Ok(());
	}

	log::info!("[{chain}] delegating signer {eoa:?} -> {target:?}");

	// EIP-7702: auth nonce = tx-execution nonce = current_nonce + 1 (self-submit).
	let tx_nonce = client
		.signer
		.get_transaction_count(eoa)
		.await
		.context("failed to fetch pending nonce")?;
	let auth_nonce = tx_nonce
		.checked_add(1)
		.ok_or_else(|| anyhow!("nonce overflow while preparing authorization"))?;

	let authorization =
		Authorization { chain_id: U256::from(chain_id), address: target, nonce: auth_nonce };

	let sig_hash: B256 = authorization.signature_hash();
	let signature = client
		.private_key_signer
		.sign_hash_sync(&sig_hash)
		.context("failed to sign EIP-7702 authorization")?;
	let signed = authorization.into_signed(signature);

	let mut tx = TransactionRequest::default()
		.from(eoa)
		.to(eoa)
		.nonce(tx_nonce)
		.gas_limit(DELEGATION_TX_GAS_FLOOR);
	tx.authorization_list = Some(vec![signed]);

	let pending = client
		.signer
		.send_transaction(tx)
		.await
		.context("sending delegation tx failed")?;
	let tx_hash = H256::from_slice(pending.tx_hash().as_slice());
	log::info!("[{chain}] delegation tx submitted: {tx_hash:?}");

	let deadline = tokio::time::Instant::now() + Duration::from_secs(5 * 60);
	loop {
		if tokio::time::Instant::now() >= deadline {
			return Err(anyhow!("timed out waiting for delegation tx {tx_hash:?}"));
		}

		if let Some(receipt) =
			client.client.get_transaction_receipt(pending.tx_hash().clone()).await?
		{
			if !receipt.status() {
				return Err(anyhow!("delegation tx reverted: {tx_hash:?}"));
			}
			break;
		}
		tokio::time::sleep(Duration::from_secs(5)).await;
	}

	if !is_delegated(client, eoa, target).await? {
		return Err(anyhow!("delegation tx mined but code mismatch on chain {chain}"));
	}

	log::info!("[{chain}] delegation confirmed");
	Ok(())
}

/// Convenience: convert `primitive_types::H160` → alloy `Address`.
#[allow(dead_code)]
pub fn h160_to_address(addr: H160) -> Address {
	Address::from_slice(&addr.0)
}
