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

//! Prover for Kaia's Istanbul BFT consensus.
//!
//! Fetches Kaia block headers over JSON-RPC (`kaia_*` namespace — the standard
//! `eth_*` block queries omit Kaia's `governanceData` and `voteData` header
//! fields), reconstructs their canonical RLP form, and assembles
//! [`KaiaClientUpdate`]s for the on-chain consensus client.
//!
//! Because the consensus client tracks the council incrementally from header
//! votes, an update must carry every header in the range whose `voteData`
//! contains a validator vote, plus epoch blocks with non-empty
//! `governanceData`, in ascending order, capped by the target header.

use anyhow::{anyhow, Context};
use futures::{stream, StreamExt, TryStreamExt};
use ismp::messaging::Keccak256;
use kaia_verifier::{
	primitives::{
		header_hash, parse_istanbul_extra, ConsensusState, HeaderVote, KaiaClientUpdate,
		KaiaCodecHeader, PendingParams, RatifiedParams,
	},
	MAX_HEADERS_PER_UPDATE,
};
use primitive_types::{H160, H256, U256};
use serde::Deserialize;
use std::{
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	time::Duration,
};

pub use kaia_verifier;

/// Hashing host for the prover.
pub struct Host;

impl Keccak256 for Host {
	fn keccak256(bytes: &[u8]) -> H256 {
		polkadot_sdk::sp_io::hashing::keccak_256(bytes).into()
	}
}

/// A Kaia block header as returned by `kaia_getBlockByNumber`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KaiaHeaderJson {
	pub parent_hash: H256,
	/// The proposer's reward address (`Rewardbase`).
	pub reward: H160,
	pub state_root: H256,
	pub transactions_root: H256,
	pub receipts_root: H256,
	pub logs_bloom: HexBytes,
	pub block_score: U256,
	pub number: U256,
	pub gas_used: U256,
	pub timestamp: U256,
	#[serde(rename = "timestampFoS", default)]
	pub timestamp_fos: Option<U256>,
	pub extra_data: HexBytes,
	pub governance_data: Option<HexBytes>,
	pub vote_data: Option<HexBytes>,
	pub base_fee_per_gas: Option<U256>,
	pub random_reveal: Option<HexBytes>,
	pub mix_hash: Option<HexBytes>,
	pub blob_gas_used: Option<U256>,
	pub excess_blob_gas: Option<U256>,
	pub vrank: Option<HexBytes>,
	pub hash: H256,
}

/// Hex-encoded bytes ("0x...") in RPC responses.
#[derive(Debug, Clone, Deserialize)]
pub struct HexBytes(#[serde(with = "hex_serde")] pub Vec<u8>);

mod hex_serde {
	use serde::{Deserialize, Deserializer};

	pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
		let s = String::deserialize(deserializer)?;
		hex::decode(s.trim_start_matches("0x")).map_err(serde::de::Error::custom)
	}
}

impl KaiaHeaderJson {
	/// Converts the RPC representation into the codec form, verifying that the
	/// reconstructed header hashes to the block hash reported by the node.
	pub fn into_verified_header(self) -> Result<KaiaCodecHeader, anyhow::Error> {
		let non_empty = |bytes: Option<HexBytes>| -> Option<Vec<u8>> {
			bytes.map(|b| b.0).filter(|b| !b.is_empty())
		};
		if self.logs_bloom.0.len() != 256 {
			Err(anyhow!("logsBloom must be 256 bytes, got {}", self.logs_bloom.0.len()))?
		}

		let header = KaiaCodecHeader {
			parent_hash: self.parent_hash,
			rewardbase: self.reward,
			state_root: self.state_root,
			transactions_root: self.transactions_root,
			receipts_root: self.receipts_root,
			logs_bloom: ethabi::ethereum_types::Bloom::from_slice(&self.logs_bloom.0),
			block_score: self.block_score,
			number: self.number,
			gas_used: self.gas_used.low_u64(),
			timestamp: self.timestamp.low_u64(),
			timestamp_fos: self.timestamp_fos.map(|v| v.low_u64() as u8).unwrap_or_default(),
			extra_data: self.extra_data.0,
			governance: self.governance_data.map(|b| b.0).unwrap_or_default(),
			vote: self.vote_data.map(|b| b.0).unwrap_or_default(),
			base_fee_per_gas: self.base_fee_per_gas,
			random_reveal: non_empty(self.random_reveal),
			mix_hash: non_empty(self.mix_hash),
			blob_gas_used: self.blob_gas_used.map(|v| v.low_u64()),
			excess_blob_gas: self.excess_blob_gas.map(|v| v.low_u64()),
			vrank: non_empty(self.vrank),
		};

		let (vanity, extra) = parse_istanbul_extra(&header.extra_data)
			.map_err(|e| anyhow!("invalid extra data in header {}: {e:?}", header.number))?;
		let computed = header_hash::<Host>(&header, &vanity, &extra);
		if computed != self.hash {
			Err(anyhow!(
				"reconstructed header {} hashes to {computed:?}, node reports {:?}; \
				 the RLP layout is out of date",
				header.number,
				self.hash
			))?
		}
		Ok(header)
	}
}

/// Tunables for [`KaiaProver`]'s RPC behaviour.
#[derive(Debug, Clone)]
pub struct ProverOptions {
	/// Concurrent header requests while scanning a block range.
	pub scan_concurrency: usize,
	/// Attempts per endpoint before giving up on a request.
	pub attempts_per_url: usize,
	/// Base backoff between retry rounds, doubling up to four times.
	pub retry_backoff_ms: u64,
	/// Per-request timeout.
	pub request_timeout_secs: u64,
}

impl Default for ProverOptions {
	fn default() -> Self {
		Self {
			scan_concurrency: 16,
			attempts_per_url: 3,
			retry_backoff_ms: 250,
			request_timeout_secs: 30,
		}
	}
}

/// JSON-RPC client for Kaia full nodes with failover across multiple URLs.
#[derive(Clone)]
pub struct KaiaProver {
	client: reqwest::Client,
	urls: Arc<Vec<String>>,
	active_url: Arc<AtomicUsize>,
	options: ProverOptions,
}

impl KaiaProver {
	pub fn new(urls: Vec<String>) -> Result<Self, anyhow::Error> {
		Self::with_options(urls, ProverOptions::default())
	}

	pub fn with_options(urls: Vec<String>, options: ProverOptions) -> Result<Self, anyhow::Error> {
		if urls.is_empty() {
			return Err(anyhow!("At least one RPC URL must be provided"));
		}
		let client = reqwest::Client::builder()
			.timeout(Duration::from_secs(options.request_timeout_secs))
			.build()?;
		Ok(Self {
			client,
			urls: Arc::new(urls),
			active_url: Arc::new(AtomicUsize::new(0)),
			options,
		})
	}

	/// Issues a JSON-RPC call, rotating across the configured endpoints and
	/// retrying with backoff.
	///
	/// Every failure mode is treated as failover-eligible, including
	/// JSON-RPC-level errors and a null result: public Kaia endpoints are
	/// load balanced, so a replica lagging the head routinely answers
	/// "unknown block" for a block its peers already have.
	async fn request<T: serde::de::DeserializeOwned>(
		&self,
		method: &str,
		params: json::Value,
	) -> Result<T, anyhow::Error> {
		let body = json::json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
		let mut last_err = None;
		let attempts = self.urls.len() * self.options.attempts_per_url.max(1);
		for attempt in 0..attempts {
			if attempt > 0 {
				// Back off once every endpoint has been tried in this round.
				if attempt % self.urls.len() == 0 {
					let round = (attempt / self.urls.len()) as u32;
					let backoff = self.options.retry_backoff_ms * 2u64.pow(round.min(4) - 1);
					tokio::time::sleep(Duration::from_millis(backoff)).await;
				}
			}
			let index = (self.active_url.load(Ordering::Relaxed) + attempt) % self.urls.len();
			let result: Result<T, anyhow::Error> = async {
				let response: json::Value = self
					.client
					.post(&self.urls[index])
					.json(&body)
					.send()
					.await?
					.error_for_status()?
					.json()
					.await?;
				if let Some(error) = response.get("error").filter(|e| !e.is_null()) {
					Err(anyhow!("{method} failed: {error}"))?
				}
				match response.get("result") {
					Some(json::Value::Null) | None => {
						Err(anyhow!("{method} returned no result; the endpoint may be lagging"))?
					},
					Some(result) => Ok(json::from_value(result.clone())
						.with_context(|| format!("failed to decode {method} response"))?),
				}
			}
			.await;
			match result {
				Ok(decoded) => {
					self.active_url.store(index, Ordering::Relaxed);
					return Ok(decoded);
				},
				Err(err) => {
					tracing::warn!("rpc {method} failed on {}: {err:?}", self.urls[index]);
					last_err = Some(err);
				},
			}
		}
		Err(last_err.unwrap_or_else(|| anyhow!("{method}: all rpc urls failed")))
	}

	pub async fn chain_id(&self) -> Result<u32, anyhow::Error> {
		let id: U256 = self.request("eth_chainId", json::json!([])).await?;
		Ok(id.low_u64() as u32)
	}

	pub async fn latest_block_number(&self) -> Result<u64, anyhow::Error> {
		let number: U256 = self.request("kaia_blockNumber", json::json!([])).await?;
		Ok(number.low_u64())
	}

	/// Fetches a header by number and verifies its reconstruction against the
	/// node-reported block hash.
	pub async fn fetch_header(&self, number: u64) -> Result<KaiaCodecHeader, anyhow::Error> {
		let header: KaiaHeaderJson = self
			.request("kaia_getBlockByNumber", json::json!([format!("{number:#x}"), false]))
			.await?;
		header.into_verified_header()
	}

	/// The council (registered validators) at the given block.
	pub async fn fetch_council(&self, number: u64) -> Result<Vec<H160>, anyhow::Error> {
		let mut council: Vec<H160> =
			self.request("kaia_getCouncil", json::json!([format!("{number:#x}")])).await?;
		council.sort();
		council.dedup();
		Ok(council)
	}

	/// The effective governance parameter set at the given block.
	pub async fn fetch_params(&self, number: u64) -> Result<GovernanceParams, anyhow::Error> {
		let params: json::Map<String, json::Value> =
			self.request("kaia_getParams", json::json!([format!("{number:#x}")])).await?;
		let committee_size = params
			.get("istanbul.committeesize")
			.and_then(|v| v.as_u64())
			.ok_or_else(|| anyhow!("istanbul.committeesize missing from kaia_getParams"))?;
		let epoch = params
			.get("istanbul.epoch")
			.and_then(|v| v.as_u64())
			.ok_or_else(|| anyhow!("istanbul.epoch missing from kaia_getParams"))?;
		let governing_node = params
			.get("governance.governingnode")
			.and_then(|v| v.as_str())
			.and_then(|s| hex::decode(s.trim_start_matches("0x")).ok())
			.filter(|b| b.len() == 20)
			.map(|b| H160::from_slice(&b))
			.ok_or_else(|| anyhow!("governance.governingnode missing from kaia_getParams"))?;
		Ok(GovernanceParams { committee_size, epoch, governing_node })
	}

	/// Builds the trusted consensus state at `number`, from which the client
	/// can verify all subsequent finalized headers.
	pub async fn fetch_consensus_state(
		&self,
		number: u64,
	) -> Result<(ConsensusState, KaiaCodecHeader), anyhow::Error> {
		let header = self.fetch_header(number).await?;
		let params = self.fetch_params(number).await?;
		if params.epoch == 0 {
			Err(anyhow!("kaia_getParams reported a zero istanbul.epoch"))?
		}
		// `kaia_getCouncil(n)` returns the set that validates block `n`, i.e.
		// votes applied through `n - 1`. The first header this state verifies
		// is `number + 1`, which the chain validates against the council that
		// includes a vote cast in `number` itself, so seed one block ahead.
		let council = self.fetch_council(number + 1).await?;
		let (vanity, extra) = parse_istanbul_extra(&header.extra_data)
			.map_err(|e| anyhow!("invalid extra data: {e:?}"))?;

		// Parameters ratified at the last epoch block do not take effect until
		// the next one, up to a week later. Seeding inside that window without
		// staging them would leave the client permanently out of step, since
		// the ratifying block is already below the finalized height and can
		// never be submitted.
		let pending_params = self.fetch_pending_params(number, params.epoch).await?;

		let state = ConsensusState {
			council,
			governing_node: params.governing_node,
			epoch: params.epoch,
			committee_size: params.committee_size,
			pending_params,
			finalized_height: number,
			finalized_hash: header_hash::<Host>(&header, &vanity, &extra),
			chain_id: self.chain_id().await?,
		};
		Ok((state, header))
	}

	/// Reads the governance ratified at the epoch block at or before `number`
	/// and stages it when it has not yet activated.
	async fn fetch_pending_params(
		&self,
		number: u64,
		epoch: u64,
	) -> Result<Option<PendingParams>, anyhow::Error> {
		let boundary = number / epoch * epoch;
		let activation_block = boundary.saturating_add(epoch);
		if boundary == 0 || activation_block <= number {
			return Ok(None);
		}
		let header = self.fetch_header(boundary).await?;
		if header.governance.is_empty() {
			return Ok(None);
		}
		let params = RatifiedParams::parse(&header.governance)
			.map_err(|e| anyhow!("invalid governance data at epoch block {boundary}: {e:?}"))?;
		if params.is_empty() {
			return Ok(None);
		}
		tracing::info!(
			"staging governance ratified at {boundary}, effective from {activation_block}",
		);
		Ok(Some(PendingParams {
			activation_block,
			committee_size: params.committee_size,
			governing_node: params.governing_node,
		}))
	}

	/// Assembles a client update advancing the state from
	/// `state.finalized_height` to `target`: every header in between whose
	/// vote field carries a validator vote (or, at epoch blocks, whose
	/// governance field is non-empty), followed by the target header.
	///
	/// If the range holds more mandatory headers than one update may carry,
	/// the update stops at the last one that fits and that header becomes the
	/// target. It is itself finalized, so the client still advances and the
	/// next call resumes from there — rather than rebuilding an oversized
	/// update the verifier rejects every tick.
	pub async fn fetch_kaia_update(
		&self,
		state: &ConsensusState,
		target: u64,
	) -> Result<Option<KaiaClientUpdate>, anyhow::Error> {
		if target <= state.finalized_height {
			return Ok(None);
		}
		let mut headers = self
			.scan_for_council_updates(state.finalized_height + 1, target - 1, state.epoch)
			.await?;

		if headers.len() >= MAX_HEADERS_PER_UPDATE {
			headers.truncate(MAX_HEADERS_PER_UPDATE);
			let capped = headers
				.last()
				.map(|header| header.number.low_u64())
				.expect("truncate leaves the vector non-empty; qed");
			tracing::info!(
				"capping update at {} mandatory headers: advancing to {capped} instead of {target}",
				headers.len(),
			);
		} else {
			headers.push(self.fetch_header(target).await?);
		}
		Ok(Some(KaiaClientUpdate { headers }))
	}

	/// Scans `[from, to]` for headers the consensus client must not skip.
	/// Headers are fetched concurrently but returned in ascending order.
	pub async fn scan_for_council_updates(
		&self,
		from: u64,
		to: u64,
		epoch: u64,
	) -> Result<Vec<KaiaCodecHeader>, anyhow::Error> {
		if from > to {
			return Ok(vec![]);
		}
		let headers = stream::iter(from..=to)
			.map(|number| async move { self.fetch_header(number).await })
			.buffered(self.options.scan_concurrency.max(1))
			.try_collect::<Vec<_>>()
			.await?;

		Ok(headers
			.into_iter()
			.filter(|header| {
				let validator_vote = !header.vote.is_empty()
					&& matches!(HeaderVote::parse_validator_vote(&header.vote), Ok(Some(_)));
				let epoch_governance =
					header.number.low_u64() % epoch == 0 && !header.governance.is_empty();
				validator_vote || epoch_governance
			})
			.collect())
	}
}

/// Governance parameters relevant to consensus verification.
#[derive(Debug, Clone)]
pub struct GovernanceParams {
	pub committee_size: u64,
	pub epoch: u64,
	pub governing_node: H160,
}

#[cfg(test)]
mod test;
