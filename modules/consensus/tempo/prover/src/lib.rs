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

//! Prover for Tempo's commonware simplex consensus.
//!
//! A JSON-RPC client over a Tempo node's `consensus` and `eth` namespaces,
//! producing the finalization certificates and headers that
//! [`tempo_verifier`] consumes. Both namespaces are served from the same
//! endpoint.

use anyhow::{anyhow, Context};
use geth_primitives::CodecHeader;
use primitive_types::{H160, H256, U256};
use serde_json::{json, Value};
use std::time::Duration;
use tempo_verifier::primitives::{CodecConsensusContext, CodecTempoHeader};

/// A finalization certificate paired with the height and hash of the block it
/// finalizes.
#[derive(Debug, Clone)]
pub struct FinalizationInfo {
	/// Raw commonware-codec finalization bytes.
	pub finalization: Vec<u8>,
	/// Consensus epoch of the certificate.
	pub epoch: u64,
	/// The finalized block hash.
	pub digest: H256,
	/// The finalized block height.
	pub height: u64,
}

/// JSON-RPC client for a Tempo node. Both the `consensus_*` and `eth_*`
/// namespaces are served from the same endpoint.
#[derive(Clone)]
pub struct TempoProver {
	client: reqwest::Client,
	url: String,
}

/// Per-request timeout. Without one a stalled RPC connection would hang the
/// consensus task indefinitely, silently halting updates for this chain.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

impl TempoProver {
	pub fn new(url: &str) -> Self {
		let client = reqwest::Client::builder()
			.timeout(REQUEST_TIMEOUT)
			.build()
			.unwrap_or_else(|_| reqwest::Client::new());
		Self { client, url: url.to_string() }
	}

	async fn request(&self, method: &str, params: Value) -> anyhow::Result<Value> {
		let response: Value = self
			.client
			.post(&self.url)
			.json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
			.send()
			.await
			.context(format!("{method} request failed"))?
			.json()
			.await
			.context(format!("{method} returned invalid json"))?;
		if let Some(error) = response.get("error") {
			return Err(anyhow!("{method} error: {error}"));
		}
		response
			.get("result")
			.cloned()
			.ok_or_else(|| anyhow!("{method} response missing result"))
	}

	/// Fetches the finalization certificate for the given height, or the
	/// latest one when `height` is `None`.
	pub async fn finalization(&self, height: Option<u64>) -> anyhow::Result<FinalizationInfo> {
		let query = match height {
			Some(height) => json!([{ "height": height }]),
			None => json!(["latest"]),
		};
		let result = self.request("consensus_getFinalization", query).await?;

		let certificate = result["certificate"]
			.as_str()
			.ok_or_else(|| anyhow!("finalization missing certificate"))?;
		let finalization = hex::decode(certificate.trim_start_matches("0x"))
			.context("certificate is not valid hex")?;
		let epoch =
			result["epoch"].as_u64().ok_or_else(|| anyhow!("finalization missing epoch"))?;
		let digest = h256(&result["digest"]).context("finalization missing digest")?;
		let height = qty(&result["block"]["header"]["number"])
			.context("finalization missing block number")?;

		Ok(FinalizationInfo { finalization, epoch, digest, height })
	}

	/// Fetches a block header by number from the execution API.
	pub async fn header(&self, height: u64) -> anyhow::Result<CodecTempoHeader> {
		let result = self
			.request("eth_getBlockByNumber", json!([format!("{:#x}", height), false]))
			.await?;
		if result.is_null() {
			return Err(anyhow!("block {height} not found"));
		}
		parse_header(&result)
	}

	/// Fetches a block's `extraData` by number.
	pub async fn extra_data(&self, height: u64) -> anyhow::Result<Vec<u8>> {
		let result = self
			.request("eth_getBlockByNumber", json!([format!("{:#x}", height), false]))
			.await?;
		if result.is_null() {
			return Err(anyhow!("block {height} not found"));
		}
		bytes(&result["extraData"]).context("block missing extraData")
	}
}

fn bytes(value: &Value) -> anyhow::Result<Vec<u8>> {
	let s = value.as_str().ok_or_else(|| anyhow!("expected hex string, got {value}"))?;
	Ok(hex::decode(s.trim_start_matches("0x"))?)
}

/// Decodes a fixed-width hex value. The length is checked rather than passed to
/// `from_slice`, which panics on a mismatch — a malformed or unexpected RPC
/// response must surface as an error, not take down the consensus task.
fn fixed<const N: usize>(value: &Value) -> anyhow::Result<[u8; N]> {
	let decoded = bytes(value)?;
	decoded
		.len()
		.eq(&N)
		.then_some(())
		.ok_or_else(|| anyhow!("expected {N} bytes, got {} in {value}", decoded.len()))?;
	let mut out = [0u8; N];
	out.copy_from_slice(&decoded);
	Ok(out)
}

fn h256(value: &Value) -> anyhow::Result<H256> {
	Ok(H256(fixed::<32>(value)?))
}

fn qty(value: &Value) -> anyhow::Result<u64> {
	let s = value.as_str().ok_or_else(|| anyhow!("expected hex quantity, got {value}"))?;
	Ok(u64::from_str_radix(s.trim_start_matches("0x"), 16)?)
}

/// Parses an `eth_getBlockByNumber` response into a [`CodecTempoHeader`].
fn parse_header(block: &Value) -> anyhow::Result<CodecTempoHeader> {
	let consensus_context = match &block["consensusContext"] {
		Value::Null => None,
		ctx => Some(CodecConsensusContext {
			epoch: ctx["epoch"]
				.as_u64()
				.ok_or_else(|| anyhow!("consensusContext missing epoch"))?,
			view: ctx["view"].as_u64().ok_or_else(|| anyhow!("consensusContext missing view"))?,
			parent_view: ctx["parentView"]
				.as_u64()
				.ok_or_else(|| anyhow!("consensusContext missing parentView"))?,
			proposer: h256(&ctx["proposer"])?,
		}),
	};

	Ok(CodecTempoHeader {
		general_gas_limit: qty(&block["mainBlockGeneralGasLimit"])?,
		shared_gas_limit: qty(&block["sharedGasLimit"])?,
		timestamp_millis_part: qty(&block["timestampMillisPart"])?,
		inner: CodecHeader {
			parent_hash: h256(&block["parentHash"])?,
			uncle_hash: h256(&block["sha3Uncles"])?,
			coinbase: H160(fixed::<20>(&block["miner"])?),
			state_root: h256(&block["stateRoot"])?,
			transactions_root: h256(&block["transactionsRoot"])?,
			receipts_root: h256(&block["receiptsRoot"])?,
			logs_bloom: ethabi_bloom(&block["logsBloom"])?,
			difficulty: U256::from(qty(&block["difficulty"])?),
			number: U256::from(qty(&block["number"])?),
			gas_limit: qty(&block["gasLimit"])?,
			gas_used: qty(&block["gasUsed"])?,
			timestamp: qty(&block["timestamp"])?,
			extra_data: bytes(&block["extraData"])?,
			mix_hash: h256(&block["mixHash"])?,
			nonce: ethabi_h64(&block["nonce"])?,
			base_fee_per_gas: opt(&block["baseFeePerGas"], |v| Ok(U256::from(qty(v)?)))?,
			withdrawals_hash: opt(&block["withdrawalsRoot"], h256)?,
			blob_gas_used: opt(&block["blobGasUsed"], qty)?,
			excess_blob_gas_used: opt(&block["excessBlobGas"], qty)?,
			parent_beacon_root: opt(&block["parentBeaconBlockRoot"], h256)?,
			requests_hash: opt(&block["requestsHash"], h256)?,
		},
		consensus_context,
	})
}

fn opt<T>(value: &Value, parse: impl Fn(&Value) -> anyhow::Result<T>) -> anyhow::Result<Option<T>> {
	match value {
		Value::Null => Ok(None),
		value => Ok(Some(parse(value)?)),
	}
}

fn ethabi_bloom(value: &Value) -> anyhow::Result<ethabi::ethereum_types::Bloom> {
	Ok(ethabi::ethereum_types::Bloom(fixed::<256>(value)?))
}

fn ethabi_h64(value: &Value) -> anyhow::Result<ethabi::ethereum_types::H64> {
	Ok(ethabi::ethereum_types::H64(fixed::<8>(value)?))
}

#[cfg(test)]
mod tests {
	use super::*;
	use sha3::Digest;
	use tempo_verifier::primitives::{
		compute_epoch, decode_dkg_outcome, ConsensusState, TempoClientUpdate,
		TEMPO_MAINNET_CHAIN_ID,
	};

	struct KeccakHasher;
	impl ismp::messaging::Keccak256 for KeccakHasher {
		fn keccak256(bytes: &[u8]) -> H256 {
			let mut hasher = sha3::Keccak256::new();
			hasher.update(bytes);
			H256::from_slice(&hasher.finalize())
		}
	}

	const MAINNET_RPC: &str = "https://rpc.presto.tempo.xyz";
	const MAINNET_EPOCH_LENGTH: u64 = 21_600;

	/// End-to-end smoke test against Tempo mainnet: fetches the latest
	/// finalization, bootstraps a consensus state from the boundary block that
	/// governs it, and runs full verification. Run with
	/// `cargo test -p tempo-prover -- --ignored`.
	#[tokio::test]
	#[ignore]
	async fn verify_live_mainnet_finalization() {
		let prover = TempoProver::new(MAINNET_RPC);
		let latest = prover.finalization(None).await.unwrap();
		let header = prover.header(latest.height).await.unwrap();

		let epoch = compute_epoch(latest.height, MAINNET_EPOCH_LENGTH);
		let boundary_height = epoch * MAINNET_EPOCH_LENGTH - 1;
		let extra_data = prover.extra_data(boundary_height).await.unwrap();
		let outcome = decode_dkg_outcome(&extra_data).unwrap();

		let consensus_state = ConsensusState {
			network_identity: outcome.network_identity,
			from_epoch: outcome.epoch,
			full_dkg_pending: outcome.is_next_full_dkg,
			finalized_height: latest.height - 1,
			finalized_hash: Default::default(),
			epoch_length: MAINNET_EPOCH_LENGTH,
			chain_id: TEMPO_MAINNET_CHAIN_ID,
		};
		let update = TempoClientUpdate { finalization: latest.finalization, header };

		let result =
			tempo_verifier::verify_tempo_update::<KeccakHasher>(&consensus_state, &update).unwrap();
		assert_eq!(result.hash, latest.digest);
		assert_eq!(result.height, latest.height);
	}
}
