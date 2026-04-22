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

//! Reads accepted BEEFY proof bytes from Hyperbridge's node-local offchain storage.
//!
//! Requires the HB node to expose the `offchain_localStorageGet` JSON-RPC —
//! typically via `--rpc-methods Unsafe`. Proofs are node-local: a node that
//! wasn't running when a proof was submitted will have no record of it.

use anyhow::anyhow;
use pallet_beefy_consensus_proofs::types::offchain_key;
use subxt::ext::subxt_rpcs::{rpc_params, RpcClient};
pub use tesseract_primitives::ConsensusProofSource;

pub struct OffchainProofSource {
	rpc_client: RpcClient,
}

impl OffchainProofSource {
	pub fn new(rpc_client: RpcClient) -> Self {
		Self { rpc_client }
	}
}

#[async_trait::async_trait]
impl ConsensusProofSource for OffchainProofSource {
	async fn fetch(&self, height: u64) -> Result<Vec<u8>, anyhow::Error> {
		let key = offchain_key(height);
		let hex_key = format!("0x{}", hex::encode(&key));
		tracing::debug!(target: "tesseract", height, key = %hex_key, "offchain proof fetch");
		let params = rpc_params!["PERSISTENT", hex_key];

		let result: Option<String> = self
			.rpc_client
			.request("offchain_localStorageGet", params)
			.await
			.map_err(|err| anyhow!("offchain_localStorageGet failed: {err:?}"))?;

		let hex_bytes = result.ok_or_else(|| {
			tracing::warn!(target: "tesseract", height, "proof missing from HB offchain storage");
			anyhow!(
				"proof missing from HB offchain storage (h={height}). \
				 Ensure the HB node exposes unsafe RPCs and was up when the proof was submitted."
			)
		})?;

		let stripped = hex_bytes.strip_prefix("0x").unwrap_or(hex_bytes.as_str());
		let bytes = hex::decode(stripped)
			.map_err(|err| anyhow!("offchain proof not valid hex: {err:?}"))?;
		tracing::debug!(target: "tesseract", height, bytes = bytes.len(), "proof fetched");
		Ok(bytes)
	}
}
