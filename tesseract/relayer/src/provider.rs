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
use codec::Decode;
use pallet_beefy_consensus_proofs::types::offchain_key;
use sp_core::twox_128;
use std::collections::BTreeMap;
use subxt::ext::subxt_rpcs::{rpc_params, RpcClient};
pub use tesseract_primitives::{ConsensusProofSource, RotationProof};

pub struct OffchainProofSource {
	rpc_client: RpcClient,
}

impl OffchainProofSource {
	pub fn new(rpc_client: RpcClient) -> Self {
		Self { rpc_client }
	}
}

/// SCALE storage key for `pallet_beefy_consensus_proofs::RotationProofs`
/// (`twox_128("BeefyConsensusProofs") ++ twox_128("RotationProofs")`). Computed
/// at call time rather than once-at-startup because it's only used on the
/// rotation-catch-up path, which runs at most once per accepted proof.
fn rotation_proofs_storage_key() -> Vec<u8> {
	let mut key = twox_128(b"BeefyConsensusProofs").to_vec();
	key.extend_from_slice(&twox_128(b"RotationProofs"));
	key
}

#[async_trait::async_trait]
impl ConsensusProofSource for OffchainProofSource {
	async fn fetch(&self, height: u64) -> Result<Vec<u8>, anyhow::Error> {
		let key = offchain_key(height);
		let hex_key = format!("0x{}", hex::encode(&key));
		tracing::debug!(target: crate::LOG_TARGET, height, key = %hex_key, "offchain proof fetch");
		let params = rpc_params!["PERSISTENT", hex_key];

		let result: Option<String> = self
			.rpc_client
			.request("offchain_localStorageGet", params)
			.await
			.map_err(|err| anyhow!("offchain_localStorageGet failed: {err:?}"))?;

		let hex_bytes = result.ok_or_else(|| {
			tracing::warn!(target: crate::LOG_TARGET, height, "proof missing from HB offchain storage");
			anyhow!(
				"proof missing from HB offchain storage (h={height}). \
				 Ensure the HB node exposes unsafe RPCs and was up when the proof was submitted."
			)
		})?;

		let stripped = hex_bytes.strip_prefix("0x").unwrap_or(hex_bytes.as_str());
		let bytes = hex::decode(stripped)
			.map_err(|err| anyhow!("offchain proof not valid hex: {err:?}"))?;
		tracing::debug!(target: crate::LOG_TARGET, height, bytes = bytes.len(), "proof fetched");
		Ok(bytes)
	}

	async fn rotation_proofs_from(
		&self,
		from_set_id: u64,
	) -> Result<Vec<RotationProof>, anyhow::Error> {
		// Read the `BoundedBTreeMap<u64, u64, MaxStoredProofs>` storage value
		// via `state_getStorage`. SCALE encoding for `BoundedBTreeMap` is the
		// same as `BTreeMap` (a length-prefixed sequence of `(K, V)`), so we
		// decode straight into `BTreeMap<u64, u64>` and avoid bringing the
		// `MaxStoredProofs` generic into scope here.
		let hex_key = format!("0x{}", hex::encode(rotation_proofs_storage_key()));
		let params = rpc_params![hex_key];
		let raw: Option<String> = self
			.rpc_client
			.request("state_getStorage", params)
			.await
			.map_err(|err| anyhow!("state_getStorage(RotationProofs) failed: {err:?}"))?;

		let Some(hex_value) = raw else {
			// Storage not yet written → no rotations recorded yet. Normal on
			// a fresh HB — nothing to catch up to.
			return Ok(Vec::new());
		};

		let stripped = hex_value.strip_prefix("0x").unwrap_or(hex_value.as_str());
		let bytes = hex::decode(stripped)
			.map_err(|err| anyhow!("RotationProofs storage not valid hex: {err:?}"))?;
		let map: BTreeMap<u64, u64> = Decode::decode(&mut &bytes[..])
			.map_err(|err| anyhow!("decode RotationProofs BTreeMap: {err:?}"))?;

		// Walk ascending so the caller can submit rotations in order.
		let mut out = Vec::new();
		for (set_id, height) in map.into_iter().filter(|(set_id, _)| *set_id > from_set_id) {
			match self.fetch(height).await {
				Ok(proof) => out.push(RotationProof { set_id, height, proof }),
				Err(err) => {
					// A rotation listed in the on-chain map but missing from
					// this node's offchain storage means the destination is
					// stuck at an epoch this node can't prove for. Log and
					// continue — later entries may still be fetchable.
					tracing::warn!(
						target: crate::LOG_TARGET,
						set_id,
						height,
						?err,
						"rotation proof missing from offchain storage",
					);
				},
			}
		}
		Ok(out)
	}
}
