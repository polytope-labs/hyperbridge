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
//!
//! Implements `tendermint_primitives::Client` using BeaconKit's beacon API endpoints
//! instead of CometBFT RPC. Uses:
//! - `GET /eth/v1/beacon/headers/head` for latest height
//! - `GET /cometbft/v1/signed_header/{height}` for signed headers
//! - `GET /eth/v1/beacon/states/{slot}/validators?status=active_ongoing,active_exiting,
//!   active_slashed` for validators
//! - `GET /eth/v1/node/health` for health checks

use cometbft::block::signed_header::SignedHeader;
use sha2::{Digest, Sha256};
use sync_committee_primitives::consensus_types::Validator as BeaconValidator;
use sync_committee_prover::responses::beacon_block_header_response;
use tendermint_primitives::{prover::ProverError, Client, Validator};

/// Response wrapper for the CometBFT-compatible endpoints on BeaconKit.
#[derive(Debug, serde::Deserialize)]
struct CometBftResponse<T> {
	data: T,
}

/// A single validator entry from the beacon API bulk validators endpoint.
#[derive(Debug, serde::Deserialize)]
struct ValidatorEntry {
	#[allow(dead_code)]
	index: String,
	#[allow(dead_code)]
	balance: String,
	#[allow(dead_code)]
	status: String,
	validator: BeaconValidator,
}

/// Response wrapper for the beacon API validators endpoint.
#[derive(Debug, serde::Deserialize)]
struct ValidatorsResponse {
	data: Vec<ValidatorEntry>,
}

/// Beacon API client for BeaconKit nodes.
///
/// Implements the `Client` trait by querying beacon API endpoints
/// exposed by BeaconKit nodes.
pub struct BeaconKitApiClient {
	pub(crate) base_url: String,
	pub(crate) http: reqwest::Client,
}

impl BeaconKitApiClient {
	/// Create a new BeaconKitApiClient.
	///
	/// `beacon_api_url` is the base URL of the BeaconKit beacon API
	/// (e.g. `http://localhost:3500`).
	pub fn new(beacon_api_url: &str) -> Result<Self, ProverError> {
		let base_url = beacon_api_url.trim_end_matches('/').to_string();
		let http = reqwest::Client::new();
		Ok(Self { base_url, http })
	}

	/// Convert a beacon validator to a CometBFT validator.
	///
	/// Mapping (from BeaconKit's `convertValidatorUpdate`):
	/// - pubkey: same BLS12-381 bytes
	/// - voting_power: `effective_balance` as i64
	/// - address: `SHA256(pubkey_bytes)[..20]`
	/// - proposer_priority: 0 (not used in validator set hash)
	fn beacon_validator_to_cometbft(
		beacon_val: &BeaconValidator,
	) -> Result<Validator, ProverError> {
		let pubkey_bytes: &[u8] = beacon_val.public_key.as_ref();

		let pub_key = cometbft::public_key::PublicKey::Bls12_381(pubkey_bytes.to_vec());

		let address_hash = Sha256::digest(pubkey_bytes);
		let address = cometbft::account::Id::try_from(address_hash[..20].to_vec())
			.map_err(|e| ProverError::ConversionError(format!("Invalid address: {}", e)))?;

		let power = cometbft::vote::Power::try_from(beacon_val.effective_balance)
			.map_err(|e| ProverError::ConversionError(format!("Invalid voting power: {}", e)))?;

		Ok(Validator {
			address,
			pub_key,
			power,
			name: None,
			proposer_priority: cometbft::validator::ProposerPriority::default(),
		})
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
			.map_err(|e| ProverError::NetworkError(format!("Header request failed: {}", e)))?
			.json()
			.await
			.map_err(|e| {
				ProverError::NetworkError(format!("Failed to parse header response: {}", e))
			})?;

		Ok(resp.data.header.message.slot)
	}

	async fn signed_header(&self, height: u64) -> Result<SignedHeader, ProverError> {
		let url = format!("{}/cometbft/v1/signed_header/{}", self.base_url, height);

		let response = self.http.get(&url).send().await.map_err(|e| {
			ProverError::NetworkError(format!("Signed header request failed: {}", e))
		})?;

		if !response.status().is_success() {
			return Err(ProverError::NetworkError(format!(
				"Signed header request returned HTTP {}",
				response.status()
			)));
		}

		let resp: CometBftResponse<SignedHeader> = response.json().await.map_err(|e| {
			ProverError::NetworkError(format!("Failed to parse signed header response: {}", e))
		})?;

		Ok(resp.data)
	}

	async fn validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		let url = format!(
			"{}/eth/v1/beacon/states/{}/validators?status=active_ongoing,active_exiting,active_slashed",
			self.base_url, height
		);

		let response =
			self.http.get(&url).send().await.map_err(|e| {
				ProverError::NetworkError(format!("Validators request failed: {}", e))
			})?;

		if !response.status().is_success() {
			return Err(ProverError::NetworkError(format!(
				"Validators request returned HTTP {}",
				response.status()
			)));
		}

		let resp: ValidatorsResponse = response.json().await.map_err(|e| {
			ProverError::NetworkError(format!("Failed to parse validators response: {}", e))
		})?;

		resp.data
			.iter()
			.map(|entry| Self::beacon_validator_to_cometbft(&entry.validator))
			.collect()
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
