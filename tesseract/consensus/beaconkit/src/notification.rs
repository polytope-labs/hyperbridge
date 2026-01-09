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

//! Notification logic for BeaconKit relayer.

use crate::{BeaconKitHost, ConsensusState};
use codec::Decode;
use cometbft::block::Height;
use ismp_beacon_kit::BeaconKitUpdate;
use polkadot_sdk::sp_runtime::BoundedVec;
use std::{sync::Arc, vec::Vec};
use tendermint_primitives::{
	Client, CodecConsensusProof, ConsensusProof, TrustedState, ValidatorSet,
};
use tendermint_verifier::validate_validator_set_hash;
use tesseract_primitives::IsmpProvider;

/// Notification logic for BeaconKit relayer
///
/// This function checks for consensus updates and generates BeaconKitUpdate proofs
/// that include both the Tendermint header proof and all transactions from the block
/// for merkle root verification.
pub async fn consensus_notification(
	client: &BeaconKitHost,
	counterparty: Arc<dyn IsmpProvider>,
) -> anyhow::Result<Option<BeaconKitUpdate>> {
	let latest_height = client.prover.latest_height().await?;

	let consensus_state_serialized: Vec<u8> =
		counterparty.query_consensus_state(None, client.consensus_state_id).await?;

	let consensus_state: ConsensusState =
		ConsensusState::decode(&mut &consensus_state_serialized[..])?;

	let trusted_state: TrustedState = consensus_state.tendermint_state.into();

	let untrusted_header = client.prover.signed_header(latest_height).await?;

	let validator_set_hash_match = validate_validator_set_hash(
		&ValidatorSet::new(trusted_state.validators.clone(), None),
		untrusted_header.header.validators_hash,
		false,
	);

	let next_validator_set_hash_match = validate_validator_set_hash(
		&ValidatorSet::new(trusted_state.next_validators.clone(), None),
		untrusted_header.header.validators_hash,
		true,
	);

	match validator_set_hash_match.is_ok() && next_validator_set_hash_match.is_ok() {
		true => {
			log::trace!(target: "tesseract-beaconkit", "BeaconKit: Onchain Validator set matches signed header, constructing consensus proof");
			let next_validators = client.prover.next_validators(latest_height).await?;

			let tendermint_proof = CodecConsensusProof::from(&ConsensusProof::new(
				untrusted_header.clone(),
				if untrusted_header.header.next_validators_hash.is_empty() {
					None
				} else {
					Some(next_validators)
				},
			));

			// Fetch all transactions from the block
			let txs = fetch_block_txs(client, latest_height).await?;

			if txs.is_empty() {
				log::warn!(target: "tesseract-beaconkit", "BeaconKit: Block has no transactions, skipping update");
				return Ok(None);
			}

			return Ok(Some(BeaconKitUpdate {
				tendermint_update: tendermint_proof,
				txs: BoundedVec::truncate_from(txs),
			}));
		},
		false => {
			log::trace!(target: "tesseract-beaconkit", "BeaconKit: No match found between onchain validator set and latest header, will begin syncing");
			// Backward traversal in order to find a matching header
			let mut height = latest_height - 1;
			let mut matched_header = None;
			while height > trusted_state.height {
				log::trace!(target: "tesseract-beaconkit", "BeaconKit: Checking for validator set match at {height}");
				let header_res = client.prover.signed_header(height).await;
				let header = match header_res {
					Ok(h) => h,
					Err(e) => {
						log::trace!(target: "tesseract-beaconkit", "BeaconKit: Error fetching header for {height}, will retry \n {e:?}");
						continue;
					},
				};

				let validator_set_hash_match = validate_validator_set_hash(
					&ValidatorSet::new(trusted_state.validators.clone(), None),
					header.header.validators_hash,
					false,
				);
				let next_validator_set_hash_match = validate_validator_set_hash(
					&ValidatorSet::new(trusted_state.next_validators.clone(), None),
					header.header.validators_hash,
					true,
				);
				if validator_set_hash_match.is_ok() && next_validator_set_hash_match.is_ok() {
					log::trace!(target: "tesseract-beaconkit", "BeaconKit: validator set match found at {height}");
					matched_header = Some(header);
					break;
				}
				height -= 1;
			}

			if matched_header.is_some() {
				let matched_height = height;
				let matched_header = matched_header.expect("Header must be present if found");
				let next_validators = client.prover.next_validators(matched_height).await?;

				let tendermint_proof = CodecConsensusProof::from(&ConsensusProof::new(
					matched_header.clone(),
					if matched_header.header.next_validators_hash.is_empty() {
						None
					} else {
						Some(next_validators)
					},
				));

				// Fetch all transactions from the block
				let txs = fetch_block_txs(client, matched_height).await?;

				if txs.is_empty() {
					log::warn!(target: "tesseract-beaconkit", "BeaconKit: Block has no transactions at matched height, skipping update");
					return Ok(None);
				}

				return Ok(Some(BeaconKitUpdate {
					tendermint_update: tendermint_proof,
					txs: BoundedVec::truncate_from(txs),
				}));
			} else {
				log::error!(target: "tesseract-beaconkit", "BeaconKit: Fatal error, failed to find any header that matches onchain validator set");
			}
		},
	}
	log::trace!(target: "tesseract-beaconkit", "BeaconKit: No new update found");
	Ok(None)
}

/// Fetch all transactions from a block at the given height.
///
/// Returns all transactions in the block as a vector.
/// The first transaction (txs[0]) is the SSZ-encoded SignedBeaconBlock.
async fn fetch_block_txs(
	client: &BeaconKitHost,
	height: u64,
) -> anyhow::Result<Vec<Vec<u8>>> {
	let height = Height::try_from(height)
		.map_err(|e| anyhow::anyhow!("Invalid height: {}", e))?;

	let rpc_url = &client.host.rpc_url;

	let block_request = serde_json::json!({
		"jsonrpc": "2.0",
		"id": "1",
		"method": "block",
		"params": {
			"height": height.value().to_string()
		}
	});

	let http_client = reqwest::Client::new();
	let block_response = http_client
		.post(rpc_url)
		.json(&block_request)
		.send()
		.await
		.map_err(|e| anyhow::anyhow!("Block fetch request failed: {}", e))?;

	if !block_response.status().is_success() {
		return Err(anyhow::anyhow!("HTTP error fetching block: {}", block_response.status()));
	}

	let block_json: serde_json::Value = block_response
		.json()
		.await
		.map_err(|e| anyhow::anyhow!("Failed to parse block response: {}", e))?;

	let txs = block_json
		.get("result")
		.and_then(|r| r.get("block"))
		.and_then(|b| b.get("data"))
		.and_then(|d| d.get("txs"))
		.and_then(|t| t.as_array())
		.ok_or_else(|| anyhow::anyhow!("Failed to extract txs from block response"))?;

	if txs.is_empty() {
		return Ok(Vec::new());
	}

	let mut all_tx_bytes: Vec<Vec<u8>> = Vec::new();
	for tx in txs {
		let tx_str = tx.as_str().ok_or_else(|| anyhow::anyhow!("Transaction is not a string"))?;
		let tx_bytes = base64_decode(tx_str)?;
		all_tx_bytes.push(tx_bytes);
	}

	Ok(all_tx_bytes)
}

/// Decode a base64 string to bytes
fn base64_decode(s: &str) -> anyhow::Result<Vec<u8>> {
	use std::io::Read;
	let mut decoder = base64::read::DecoderReader::new(
		s.as_bytes(),
		&base64::engine::general_purpose::STANDARD,
	);
	let mut decoded = Vec::new();
	decoder.read_to_end(&mut decoded)?;
	Ok(decoded)
}
