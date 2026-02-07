// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! TRON transport layer for the tesseract ISMP relayer.
//!
//! TRON's TVM is EVM-compatible and exposes Ethereum JSON-RPC, so we **reuse
//! [`tesseract_evm::EvmClient`]** for every read operation (querying consensus
//! state, events, proofs, storage slots, etc.).
//!
//! The only part that differs is **transaction submission**: TRON does not accept
//! standard Ethereum-signed transactions via `eth_sendRawTransaction`.  Instead
//! we build, sign and broadcast transactions through TRON's native HTTP API
//! (`/wallet/triggersmartcontract` → sign → `/wallet/broadcasttransaction`).
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │                   TronClient                     │
//! │                                                  │
//! │  ┌──────────────┐        ┌────────────────────┐  │
//! │  │   EvmClient  │        │     TronApi        │  │
//! │  │  (JSON-RPC)  │        │ (/wallet/* HTTP)   │  │
//! │  │              │        │                    │  │
//! │  │  • queries   │        │  • triggerSmartCon │  │
//! │  │  • events    │        │  • broadcastTx     │  │
//! │  │  • proofs    │        │  • getTransInfo    │  │
//! │  │  • estimates │        │                    │  │
//! │  └──────────────┘        └────────────────────┘  │
//! └──────────────────────────────────────────────────┘
//! ```

pub mod api;
pub mod provider;
pub mod tx;

use std::sync::Arc;

use anyhow::anyhow;
use ismp::{host::StateMachine, messaging::Message};
use polkadot_sdk::frame_support::crypto;
use serde::{Deserialize, Serialize};
use sp_core::Pair;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{
	queue::{start_pipeline, PipelineQueue},
	TxResult,
};

use crate::api::{to_tron_hex, TronApi, TronApiConfig};

/// TRON mainnet chain ID, matching `TronHost.CHAIN_ID`.
pub const TRON_MAINNET_CHAIN_ID: u32 = 728126428;

/// Configuration for a [`TronClient`].
///
/// Embeds a standard [`EvmConfig`] for JSON-RPC reads, plus TRON-specific
/// fields for the native HTTP API used for transaction submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TronConfig {
	/// Standard EVM configuration.
	///
	/// `rpc_urls` should point to the Ethereum JSON-RPC endpoint exposed by
	/// the TRON node:
	/// - TRE local:      `http://127.0.0.1:8545`  (or proxied through 9090)
	/// - TronGrid:       `https://api.trongrid.io/jsonrpc`
	/// - Nile testnet:   `https://nile.trongrid.io/jsonrpc`
	#[serde(flatten)]
	pub evm: EvmConfig,

	/// Base URL of the TRON full-node **native** HTTP API.
	///
	/// This is the endpoint for `/wallet/*` calls (triggerSmartContract,
	/// broadcastTransaction, etc.).
	///
	/// - TRE local:     `http://127.0.0.1:9090`
	/// - TronGrid:      `https://api.trongrid.io`
	/// - Nile testnet:  `https://nile.trongrid.io`
	pub tron_api_url: String,

	/// Optional TRON API key for services like TronGrid.
	///
	/// If provided, this will be sent as the `TRON-PRO-API-KEY` header.
	/// Alternatively, you can include the API key directly in the URL:
	/// `https://api.trongrid.io?api_key=YOUR_API_KEY`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tron_api_key: Option<String>,

	/// Maximum fee for contract trigger transactions, in SUN
	/// (1 TRX = 1_000_000 SUN).  Default: 1_000_000_000 (1000 TRX).
	#[serde(default = "default_fee_limit")]
	pub fee_limit: u64,

	/// HTTP request timeout for the TRON native API, in seconds.
	/// Default: 180.
	#[serde(default = "default_timeout")]
	pub tron_api_timeout_secs: u64,
}

fn default_fee_limit() -> u64 {
	1_000_000_000
}

fn default_timeout() -> u64 {
	180
}

impl TronConfig {
	pub async fn into_client(self) -> anyhow::Result<TronClient> {
		TronClient::new(self).await
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm.state_machine
	}
}

/// TRON client for the tesseract ISMP relayer.
///
/// Wraps an [`EvmClient`] for all read operations (JSON-RPC) and uses the
/// TRON native HTTP API exclusively for transaction submission.
pub struct TronClient {
	/// Inner EVM client — handles all queries, event scanning, proof
	/// generation, storage key derivation, etc.
	pub evm: EvmClient,

	/// TRON native HTTP API client — used only for transaction submission.
	pub tron_api: TronApi,

	/// The 32-byte secp256k1 secret key (same key as used by `evm.signer`,
	/// but we need the raw bytes for TRON's signing scheme).
	pub(crate) secret_key: [u8; 32],

	/// Owner address in TRON hex format (`41`-prefixed, 42 hex chars).
	pub owner_address: String,

	/// Handler contract address in TRON hex format.
	pub handler_address: String,

	/// IsmpHost contract address in TRON hex format.
	pub ismp_host_address: String,

	/// Fee limit for transactions, in SUN.
	pub fee_limit: u64,

	/// Full configuration (retained for introspection).
	pub config: TronConfig,

	/// Transaction submission pipeline.
	queue: Option<Arc<PipelineQueue<Vec<Message>, anyhow::Result<TxResult>>>>,
}

impl TronClient {
	/// Create a new [`TronClient`] from the provided configuration.
	///
	/// This initialises an [`EvmClient`] for JSON-RPC reads and a [`TronApi`]
	/// for TRON-native transaction submission.
	pub async fn new(config: TronConfig) -> anyhow::Result<Self> {
		let key_bytes = match sp_core::bytes::from_hex(&config.evm.signer) {
			Ok(bytes) => bytes,
			Err(_) => {
				let contents = tokio::fs::read_to_string(&config.evm.signer).await?;
				sp_core::bytes::from_hex(contents.trim())?
			},
		};

		if key_bytes.len() != 32 {
			return Err(anyhow!("Signer key must be 32 bytes, got {}", key_bytes.len()));
		}

		let mut secret_key = [0u8; 32];
		secret_key.copy_from_slice(&key_bytes);

		// Derive the TRON hex address from the secret key.
		let pair = sp_core::ecdsa::Pair::from_seed_slice(&secret_key)?;
		let evm_addr = crypto::ecdsa::ECDSAExt::to_eth_address(&pair.public()).expect("Infallible");
		let owner_address = to_tron_hex(&hex::encode(evm_addr));

		let evm = EvmClient::new(config.evm.clone()).await?;

		let tron_api = TronApi::new(TronApiConfig {
			full_host: config.tron_api_url.clone(),
			api_key: config.tron_api_key.clone(),
			timeout: std::time::Duration::from_secs(config.tron_api_timeout_secs),
		})?;

		let ismp_host_address = to_tron_hex(&format!("{:?}", config.evm.ismp_host));

		// Query handler from the host contract via JSON-RPC (uses force_legacy).
		let handler_h160 = evm.handler().await?;
		let handler_address = to_tron_hex(&format!("{:x}", handler_h160));

		let fee_limit = config.fee_limit;
		let config_clone = config.clone();

		let mut client = Self {
			evm,
			tron_api,
			secret_key,
			owner_address,
			handler_address,
			ismp_host_address,
			fee_limit,
			config: config_clone,
			queue: None,
		};

		let client_for_pipeline = client.clone();
		let queue = start_pipeline(move |messages| {
			let c = client_for_pipeline.clone();
			async move { tx::handle_message_submission(&c, messages).await }
		});
		client.queue = Some(Arc::new(queue));

		log::info!(
			"[tron] Initialized TronClient for {:?} (host={}, handler={}, relayer={})",
			client.evm.state_machine,
			client.ismp_host_address,
			client.handler_address,
			client.owner_address,
		);

		Ok(client)
	}

	/// Reference to the inner [`EvmClient`] for direct JSON-RPC access.
	pub fn inner(&self) -> &EvmClient {
		&self.evm
	}

	/// Return a reference to the transaction submission queue.
	pub(crate) fn queue(
		&self,
	) -> anyhow::Result<&Arc<PipelineQueue<Vec<Message>, anyhow::Result<TxResult>>>> {
		self.queue
			.as_ref()
			.ok_or_else(|| anyhow!("Transaction submission pipeline was not initialized"))
	}
}

/// Query the handler address from the host contract using the TRON native API.
///
/// Calls `hostParams()` via `triggerConstantContract` and extracts the
/// `handler` field (index 5) from the A

impl Clone for TronClient {
	fn clone(&self) -> Self {
		Self {
			evm: self.evm.clone(),
			tron_api: self.tron_api.clone(),
			secret_key: self.secret_key,
			owner_address: self.owner_address.clone(),
			handler_address: self.handler_address.clone(),
			ismp_host_address: self.ismp_host_address.clone(),
			fee_limit: self.fee_limit,
			config: self.config.clone(),
			queue: self.queue.clone(),
		}
	}
}
