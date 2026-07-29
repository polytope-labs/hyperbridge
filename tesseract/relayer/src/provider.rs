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
use pallet_beefy_consensus_proofs::types::{messaging_offchain_key, rotation_offchain_key};
use sp_crypto_hashing::twox_128;
use std::collections::BTreeMap;
use subxt::ext::subxt_rpcs::{rpc_params, RpcClient};
pub use tesseract_primitives::{ConsensusProofSource, ProofKey, RotationProof};

pub struct OffchainProofSource {
	rpc_client: RpcClient,
}

impl OffchainProofSource {
	pub fn new(rpc_client: RpcClient) -> Self {
		Self { rpc_client }
	}
}

/// SCALE storage key for a `pallet_beefy_consensus_proofs` item
/// (`twox_128("BeefyConsensusProofs") ++ twox_128(item)`). Computed at call time rather
/// than once-at-startup because it's only used on the rotation-catch-up path, which runs
/// at most once per accepted proof.
fn storage_key(item: &[u8]) -> Vec<u8> {
	let mut key = twox_128(b"BeefyConsensusProofs").to_vec();
	key.extend_from_slice(&twox_128(item));
	key
}

impl OffchainProofSource {
	/// Raw read from the node's persistent offchain store. `None` when nothing was ever
	/// written under `storage_key` on this node.
	async fn read_offchain(&self, storage_key: &[u8]) -> Result<Option<Vec<u8>>, anyhow::Error> {
		let hex_key = format!("0x{}", hex::encode(storage_key));
		let result: Option<String> = self
			.rpc_client
			.request("offchain_localStorageGet", rpc_params!["PERSISTENT", hex_key])
			.await
			.map_err(|err| anyhow!("offchain_localStorageGet failed: {err:?}"))?;

		result
			.map(|hex_bytes| {
				let stripped = hex_bytes.strip_prefix("0x").unwrap_or(hex_bytes.as_str());
				hex::decode(stripped)
					.map_err(|err| anyhow!("offchain proof not valid hex: {err:?}"))
			})
			.transpose()
	}

	/// The pallet's `set_id → parachain height` rotation index. Empty when the storage item
	/// was never written, which is normal on a fresh chain.
	async fn rotation_proofs(&self) -> Result<BTreeMap<u64, u64>, anyhow::Error> {
		let hex_key = format!("0x{}", hex::encode(storage_key(b"RotationProofs")));
		let raw: Option<String> = self
			.rpc_client
			.request("state_getStorage", rpc_params![hex_key])
			.await
			.map_err(|err| anyhow!("state_getStorage(RotationProofs) failed: {err:?}"))?;

		let Some(hex_value) = raw else { return Ok(BTreeMap::new()) };
		let stripped = hex_value.strip_prefix("0x").unwrap_or(hex_value.as_str());
		let bytes = hex::decode(stripped)
			.map_err(|err| anyhow!("RotationProofs storage not valid hex: {err:?}"))?;
		// `BoundedBTreeMap` encodes exactly like `BTreeMap`, so we skip the bound generic.
		Decode::decode(&mut &bytes[..])
			.map_err(|err| anyhow!("decode RotationProofs BTreeMap: {err:?}"))
	}
}

#[async_trait::async_trait]
impl ConsensusProofSource for OffchainProofSource {
	async fn fetch(&self, key: ProofKey) -> Result<Vec<u8>, anyhow::Error> {
		let storage_key = match key {
			ProofKey::Messaging(height) => messaging_offchain_key(height),
			ProofKey::Rotation(set_id) => rotation_offchain_key(set_id),
		};
		tracing::debug!(target: crate::LOG_TARGET, ?key, "offchain proof fetch");

		if let Some(bytes) = self.read_offchain(&storage_key).await? {
			tracing::debug!(target: crate::LOG_TARGET, ?key, bytes = bytes.len(), "proof fetched");
			return Ok(bytes);
		}

		// A mandatory proof advances the proven height but writes no messaging blob, so a
		// caller holding only that height finds nothing here. Resolve the height through the
		// rotation index and read it out of the namespace it actually landed in.
		if let ProofKey::Messaging(height) = key {
			let rotated = self
				.rotation_proofs()
				.await?
				.into_iter()
				.find_map(|(set_id, at)| (at == height).then_some(set_id));

			if let Some(set_id) = rotated {
				if let Some(bytes) = self.read_offchain(&rotation_offchain_key(set_id)).await? {
					tracing::debug!(
						target: crate::LOG_TARGET,
						height,
						set_id,
						"height belongs to a rotation proof",
					);
					return Ok(bytes);
				}
			}
		}

		tracing::warn!(target: crate::LOG_TARGET, ?key, "proof missing from HB offchain storage");
		Err(anyhow!(
			"proof missing from HB offchain storage ({key:?}). \
			 Ensure the HB node exposes unsafe RPCs and was up when the proof was submitted."
		))
	}

	async fn rotation_proofs_from(
		&self,
		from_set_id: u64,
	) -> Result<Vec<RotationProof>, anyhow::Error> {
		// An empty map means no rotations are recorded yet, which is normal on a fresh HB.
		let map = self.rotation_proofs().await?;
		if map.is_empty() {
			return Ok(Vec::new());
		}

		let next_set_id = from_set_id + 1;
		if !map.contains_key(&next_set_id) {
			return Err(anyhow!(
				"RotationProofs missing entry for set_id {next_set_id} (catching up from {from_set_id})"
			));
		}

		// Walk ascending so the caller can submit rotations in order. The blob for each
		// rotation lives under its set id, which is the key we are already iterating.
		// Rotations recorded before the namespace split still sit under the messaging key at
		// their recorded height, so fall back there rather than strand a lagging destination.
		// Drop the fallback once every live destination is past the upgrade.
		let mut out = Vec::new();
		for (set_id, height) in map.into_iter().filter(|(set_id, _)| *set_id > from_set_id) {
			let proof = match self.fetch(ProofKey::Rotation(set_id)).await {
				Ok(proof) => proof,
				Err(_) => self.fetch(ProofKey::Messaging(height)).await.map_err(|err| {
					anyhow!("rotation proof missing from offchain storage (set_id={set_id}, height={height}): {err:?}")
				})?,
			};
			out.push(RotationProof { set_id, height, proof });
		}
		Ok(out)
	}
}
