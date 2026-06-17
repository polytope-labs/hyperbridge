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

//! Beacon API client for BeaconKit nodes.

use blst::min_pk::PublicKey as BlstPublicKey;
use cometbft::{
	block::signed_header::SignedHeader, validator::Info as Validator, vote::Power, PublicKey,
};
use sync_committee_prover::responses::beacon_block_header_response;
use tendermint_primitives::{prover::ProverError, Client};

#[derive(Debug, serde::Deserialize)]
struct BeaconValidator {
	validator: BeaconValidatorData,
}

#[derive(Debug, serde::Deserialize)]
struct BeaconValidatorData {
	pubkey: String,
	effective_balance: String,
}

#[derive(Debug, serde::Deserialize)]
struct CometBftResponse<T> {
	data: T,
}

fn int_to_string(val: &mut serde_json::Value) {
	if let Some(n) = val.as_u64() {
		*val = serde_json::Value::String(n.to_string());
	}
}

// BeaconKit returns some CometBFT integer fields as JSON numbers instead of strings.
fn normalize_signed_header_json(mut v: serde_json::Value) -> serde_json::Value {
	for path in &[
		"/data/header/version/block",
		"/data/header/version/app",
		"/data/header/height",
		"/data/commit/height",
	] {
		if let Some(field) = v.pointer_mut(path) {
			int_to_string(field);
		}
	}
	v
}

// BeaconKit's CometBFT fork uses the 96-byte uncompressed G1 point for both validator
// address derivation and SimpleValidator hashing, while the beacon API returns the
// standard 48-byte compressed form. We expand here so the two sides agree.
fn bls_compressed_to_serialized(compressed_hex: &str) -> Result<Vec<u8>, ProverError> {
	let hex_str = compressed_hex.trim_start_matches("0x");
	let compressed = hex::decode(hex_str)
		.map_err(|e| ProverError::NetworkError(format!("Invalid BLS pubkey hex: {e}")))?;

	if compressed.len() != 48 {
		return Err(ProverError::NetworkError(format!(
			"Expected 48-byte compressed BLS key, got {} bytes",
			compressed.len()
		)));
	}

	let pk = BlstPublicKey::key_validate(&compressed)
		.map_err(|e| ProverError::NetworkError(format!("Invalid BLS pubkey: {e:?}")))?;

	Ok(pk.serialize().to_vec())
}

fn beacon_validator_to_cometbft(
	pubkey_hex: &str,
	effective_balance_gwei: u64,
) -> Result<Validator, ProverError> {
	let pub_key = PublicKey::Bls12_381(bls_compressed_to_serialized(pubkey_hex)?);
	// BeaconKit sets power = int64(effectiveBalance.Unwrap()), raw Gwei with no conversion
	let power = Power::try_from(effective_balance_gwei as i64)
		.map_err(|e| ProverError::NetworkError(format!("Invalid voting power: {e}")))?;
	Ok(Validator::new(pub_key, power))
}

pub struct BeaconKitApiClient {
	pub(crate) base_url: String,
	pub(crate) http: reqwest::Client,
}

impl BeaconKitApiClient {
	pub fn new(beacon_api_url: &str) -> Result<Self, ProverError> {
		Ok(Self {
			base_url: beacon_api_url.trim_end_matches('/').to_string(),
			http: reqwest::Client::new(),
		})
	}

	async fn validators_from_beacon_api(&self, slot: u64) -> Result<Vec<Validator>, ProverError> {
		let url = format!(
			"{}/eth/v1/beacon/states/{}/validators?status=active_ongoing",
			self.base_url, slot
		);

		let mut last_err: Option<ProverError> = None;
		let mut data_opt: Option<Vec<BeaconValidator>> = None;

		for attempt in 0..5u32 {
			if attempt > 0 {
				tokio::time::sleep(std::time::Duration::from_millis(500)).await;
			}
			match self.http.get(&url).send().await {
				Ok(resp) if resp.status().is_success() => {
					match resp.json::<CometBftResponse<Vec<BeaconValidator>>>().await {
						Ok(body) => {
							data_opt = Some(body.data);
							break;
						},
						Err(e) => {
							last_err = Some(ProverError::NetworkError(format!(
								"Failed to decode beacon validators: {e}"
							)));
						},
					}
				},
				Ok(resp) => {
					last_err = Some(ProverError::NetworkError(format!(
						"Beacon validators request returned HTTP {}",
						resp.status()
					)));
				},
				Err(e) => {
					last_err = Some(ProverError::NetworkError(format!(
						"Beacon validators request failed: {e}"
					)));
				},
			}
		}

		let beacon_validators = data_opt
			.ok_or_else(|| last_err.unwrap_or(ProverError::NetworkError("No response".into())))?;

		beacon_validators
			.into_iter()
			.map(|bv| {
				let effective_balance: u64 =
					bv.validator.effective_balance.parse().map_err(|e| {
						ProverError::NetworkError(format!("Invalid effective_balance: {e}"))
					})?;
				beacon_validator_to_cometbft(&bv.validator.pubkey, effective_balance)
			})
			.collect()
	}
}

#[async_trait::async_trait]
impl Client for BeaconKitApiClient {
	async fn latest_height(&self) -> Result<u64, ProverError> {
		let url =
			format!("{}{}", self.base_url, sync_committee_prover::routes::header_route("head"));

		let resp: beacon_block_header_response::Response = self
			.http
			.get(&url)
			.send()
			.await
			.map_err(|e| ProverError::NetworkError(format!("Header request failed: {e}")))?
			.json()
			.await
			.map_err(|e| {
				ProverError::NetworkError(format!("Failed to parse header response: {e}"))
			})?;

		Ok(resp.data.header.message.slot)
	}

	async fn signed_header(&self, height: u64) -> Result<SignedHeader, ProverError> {
		let url = format!("{}/cometbft/v1/signed_header/{}", self.base_url, height);

		let mut last_err = None;
		for attempt in 0..5u32 {
			if attempt > 0 {
				tokio::time::sleep(std::time::Duration::from_millis(500)).await;
			}
			let response = self.http.get(&url).send().await.map_err(|e| {
				ProverError::NetworkError(format!("Signed header request failed: {e}"))
			})?;

			if response.status().is_success() {
				let raw: serde_json::Value = response.json().await.map_err(|e| {
					ProverError::NetworkError(format!(
						"Failed to parse signed header response: {e}"
					))
				})?;
				let normalized = normalize_signed_header_json(raw);
				let resp: CometBftResponse<SignedHeader> = serde_json::from_value(normalized)
					.map_err(|e| {
						ProverError::NetworkError(format!(
							"Failed to decode signed header response: {e}"
						))
					})?;
				return Ok(resp.data);
			}

			last_err = Some(ProverError::NetworkError(format!(
				"Signed header request returned HTTP {}",
				response.status()
			)));
		}

		Err(last_err
			.unwrap_or_else(|| ProverError::NetworkError("Signed header request failed".into())))
	}

	async fn validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		self.validators_from_beacon_api(height).await
	}

	async fn next_validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		self.validators(height + 1).await
	}

	async fn chain_id(&self) -> Result<String, ProverError> {
		let latest = self.latest_height().await?;
		let header = self.signed_header(latest).await?;
		Ok(header.header.chain_id.to_string())
	}

	async fn is_healthy(&self) -> Result<bool, ProverError> {
		let url = format!("{}/eth/v1/node/health", self.base_url);
		match self.http.get(&url).send().await {
			Ok(resp) => Ok(resp.status().is_success()),
			Err(_) => Ok(false),
		}
	}
}
