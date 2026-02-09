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

//! TRON transaction submission pipeline.
//!
//! This module **reuses [`tesseract_evm::tx::generate_contract_calls`]** for all
//! the complex ABI encoding (consensus proofs, MMR proofs, post request/response
//! structs, etc.) and only implements the TRON-specific parts:
//!
//! 1. Extract raw calldata from the ethers `FunctionCall` objects
//! 2. POST to `/wallet/triggersmartcontract` with a `data` field
//! 3. Sign the returned unsigned transaction (secp256k1 over SHA-256)
//! 4. Broadcast via `/wallet/broadcasttransaction`
//! 5. Poll `/wallet/gettransactioninfobyid` for the receipt
//!
//! The calldata is byte-identical to Ethereum because TRON's TVM uses the same
//! ABI encoding.  Only the transaction envelope differs.

use std::{collections::BTreeSet, time::Duration};

use anyhow::{anyhow, Context};
use ethers::providers::Middleware;
use ismp::{
	messaging::{hash_request, hash_response, Message, ResponseMessage},
	router::{Request, RequestResponse},
};
use primitive_types::H256;
use serde::Serialize;
use sp_core::keccak_256;
use tesseract_primitives::{Hasher, Query, TxReceipt, TxResult};

use crate::{
	address::{hex_to_base58, is_base58_address},
	api::{
		SignedTransaction, TransactionInfo, TriggerConstantContractResponse,
		TriggerContractRequest, TriggerSmartContractResponse, TronApi,
	},
	catfee::CatFeeClient,
	TronClient,
};

// Re-use tesseract-evm's ABI encoding for all ISMP message types.
use tesseract_evm::tx::generate_contract_calls;

/// Request body for `POST /wallet/triggersmartcontract` using the raw `data`
/// field instead of `function_selector` + `parameter`.
///
/// When `data` is present the TRON node uses it directly as the contract
/// input, which is exactly the 4-byte selector + ABI-encoded parameters
/// that ethers produces.
#[derive(Debug, Serialize)]
struct TriggerWithDataRequest {
	owner_address: String,
	contract_address: String,
	/// Full hex-encoded calldata (function selector + abi params, no `0x` prefix).
	data: String,
	fee_limit: u64,
	call_value: u64,
	visible: bool,
}

/// Entry point for the transaction submission pipeline.
///
/// Called by the [`PipelineQueue`] consumer.  Mirrors
/// `tesseract_evm::tx::handle_message_submission` but routes through the TRON
/// native API.
pub async fn handle_message_submission(
	client: &TronClient,
	messages: Vec<Message>,
) -> Result<TxResult, anyhow::Error> {
	log::trace!("[tron::tx] handle_message_submission called with {} messages", messages.len());

	let (receipts, cancelled) = submit_messages(client, messages.clone()).await?;
	log::trace!(
		"[tron::tx] submit_messages returned {} receipts, {} cancelled",
		receipts.len(),
		cancelled.len()
	);

	let height = client.evm.client.get_block_number().await.map(|n| n.as_u64()).unwrap_or(0);
	log::trace!("[tron::tx] Current block height: {}", height);

	let mut results = Vec::new();

	for msg in &messages {
		match msg {
			Message::Request(req_msg) => {
				log::trace!(
					"[tron::tx] Processing Request message with {} posts",
					req_msg.requests.len()
				);
				for post in &req_msg.requests {
					let req = Request::Post(post.clone());
					let commitment = hash_request::<Hasher>(&req);
					log::trace!("[tron::tx] Request commitment: {:?}", commitment);
					if receipts.contains(&commitment) {
						log::trace!(
							"[tron::tx] Adding receipt for request commitment {:?}",
							commitment
						);
						results.push(TxReceipt::Request {
							query: Query {
								source_chain: req.source_chain(),
								dest_chain: req.dest_chain(),
								nonce: req.nonce(),
								commitment,
							},
							height,
						});
					}
				}
			},
			Message::Response(ResponseMessage {
				datagram: RequestResponse::Response(resp),
				..
			}) => {
				log::trace!("[tron::tx] Processing Response message with {} responses", resp.len());
				for res in resp {
					let commitment = hash_response::<Hasher>(res);
					let request_commitment = hash_request::<Hasher>(&res.request());
					log::trace!(
						"[tron::tx] Response commitment: {:?}, request commitment: {:?}",
						commitment,
						request_commitment
					);
					if receipts.contains(&commitment) {
						log::trace!(
							"[tron::tx] Adding receipt for response commitment {:?}",
							commitment
						);
						results.push(TxReceipt::Response {
							query: Query {
								source_chain: res.source_chain(),
								dest_chain: res.dest_chain(),
								nonce: res.nonce(),
								commitment,
							},
							request_commitment,
							height,
						});
					}
				}
			},
			_ => {
				log::trace!("[tron::tx] Skipping non-request/response message");
			},
		}
	}

	log::info!(
		"[tron::tx] handle_message_submission complete: {} receipts, {} unsuccessful",
		results.len(),
		cancelled.len()
	);
	Ok(TxResult { receipts: results, unsuccessful: cancelled })
}

/// Submit a batch of [`Message`]s to the TRON network.
///
/// Returns a set of commitment hashes for successfully-processed messages and
/// a list of messages that were not delivered.
pub async fn submit_messages(
	client: &TronClient,
	messages: Vec<Message>,
) -> anyhow::Result<(BTreeSet<H256>, Vec<Message>)> {
	log::trace!("[tron::tx] submit_messages called with {} messages", messages.len());

	//
	// We pass `debug_trace = true` to skip the gas-price oracle (which may
	// not be available on TRON).  Gas estimation failures are caught by
	// `unwrap_or` inside `generate_contract_calls`, so this is safe.
	log::trace!("[tron::tx] Calling generate_contract_calls with debug_trace=true");
	let calls = generate_contract_calls(&client.evm, messages.clone(), true).await?;
	log::trace!("[tron::tx] generate_contract_calls returned {} calls", calls.len());

	let mut events = BTreeSet::new();
	let mut cancelled: Vec<Message> = Vec::new();

	for (index, call) in calls.iter().enumerate() {
		log::trace!("[tron::tx] Processing call {} of {}", index + 1, calls.len());

		let calldata = match call.calldata() {
			Some(bytes) => {
				log::trace!("[tron::tx] Call {} has calldata of {} bytes", index, bytes.len());
				bytes
			},
			None => {
				log::error!("[tron] Message at index {index} produced empty calldata, skipping");
				cancelled.push(messages[index].clone());
				continue;
			},
		};

		let calldata_hex = hex::encode(calldata.as_ref());
		log::trace!(
			"[tron::tx] Call {} calldata (hex): {}...",
			index,
			&calldata_hex[..std::cmp::min(64, calldata_hex.len())]
		);

		let label = match &messages[index] {
			Message::Consensus(_) => "handleConsensus",
			Message::Request(_) => "handlePostRequests",
			Message::Response(_) => "handlePostResponses",
			Message::Timeout(_) => "handleTimeout",
			Message::FraudProof(_) => "fraudProof",
		};

		log::info!(
			"[tron] Submitting {label} to {:?} ({} bytes calldata) ...",
			client.evm.state_machine,
			calldata.len(),
		);

		match trigger_sign_broadcast(client, &calldata_hex).await {
			Ok(info) => {
				log::trace!("[tron::tx] trigger_sign_broadcast returned for call {}", index);

				if info.succeeded() {
					let energy = info.receipt.as_ref().map(|r| r.energy_usage_total).unwrap_or(0);

					log::info!(
						"[tron] {label} succeeded (tx={}, block={}, energy={energy})",
						info.id,
						info.block_number,
					);

					let msg_events = extract_commitment_hashes(&info);
					log::trace!(
						"[tron::tx] Extracted {} commitment hashes from tx {}",
						msg_events.len(),
						info.id
					);

					// For request/response messages, an empty event set means
					// the message was likely a duplicate (already delivered).
					if matches!(messages[index], Message::Request(_) | Message::Response(_)) &&
						msg_events.is_empty()
					{
						log::warn!("[tron::tx] Request/Response message at index {} produced no events (likely duplicate), cancelling", index);
						cancelled.push(messages[index].clone());
					}
					events.extend(msg_events);
				} else {
					let reason = decode_revert_message(&info);
					log::error!("[tron] {label} reverted (tx={}): {reason}", info.id);
					cancelled.push(messages[index].clone());
				}
			},
			Err(err) => {
				log::error!("[tron] {label} submission failed: {err:#}");
				return Err(err);
			},
		}
	}

	if !events.is_empty() {
		log::trace!(
			"[tron] Got {} receipts from executing on {:?}",
			events.len(),
			client.evm.state_machine,
		);
	}

	log::trace!(
		"[tron::tx] submit_messages complete: {} events, {} cancelled",
		events.len(),
		cancelled.len()
	);
	Ok((events, cancelled))
}

/// Build, sign, broadcast a TRON smart-contract call and wait for its receipt.
///
/// # Arguments
///
/// * `client`       – the TronClient (provides API client, keys, addresses)
/// * `calldata_hex` – full hex-encoded calldata (4-byte selector + ABI params, **no** `0x` prefix),
///   as produced by ethers
async fn trigger_sign_broadcast(
	client: &TronClient,
	calldata_hex: &str,
) -> anyhow::Result<TransactionInfo> {
	log::trace!("[tron::tx] trigger_sign_broadcast: preparing request");
	log::trace!(
		"[tron::tx] owner_address: {}, contract_address: {}, fee_limit: {}",
		client.owner_address,
		client.handler_address,
		client.fee_limit
	);

	// Step 1: Estimate energy requirements
	let estimated_energy = estimate_transaction_energy(client, calldata_hex).await?;

	log::trace!("[tron] Estimated energy: {}", estimated_energy);

	// Step 2: Purchase energy via CatFee if available
	if let Some(catfee_client) = &client.catfee {
		purchase_energy_via_catfee(catfee_client, &client.owner_address, estimated_energy).await?;
	}

	let trigger_req = TriggerWithDataRequest {
		owner_address: client.owner_address.clone(),
		contract_address: client.handler_address.clone(),
		data: calldata_hex.to_string(),
		fee_limit: client.fee_limit,
		call_value: 0,
		visible: false,
	};

	log::trace!("[tron::tx] Calling POST /wallet/triggersmartcontract");
	let resp: TriggerSmartContractResponse =
		post_json(client.tron_api.full_host(), "/wallet/triggersmartcontract", &trigger_req, None)
			.await
			.context("triggerSmartContract failed")?;

	log::trace!("[tron::tx] triggerSmartContract response received, checking result");
	resp.result.into_result().context("triggerSmartContract rejected")?;

	let unsigned = resp
		.transaction
		.ok_or_else(|| anyhow!("triggerSmartContract returned no transaction"))?;

	let tx_id = unsigned.tx_id.clone();
	log::trace!("[tron::tx] Got unsigned transaction with tx_id={}", tx_id);

	log::trace!("[tron::tx] Signing transaction");
	let signed = SignedTransaction::sign(unsigned, &client.secret_key)
		.context("failed to sign TRON transaction")?;

	log::trace!("[tron::tx] Broadcasting transaction tx_id={}", tx_id);
	let broadcast_result = client
		.tron_api
		.broadcast_transaction(&signed)
		.await
		.context("broadcastTransaction failed")?;

	log::trace!("[tron::tx] Broadcast response received, checking result");
	broadcast_result.into_result().context("broadcastTransaction rejected")?;

	log::trace!("[tron] Broadcast tx_id={tx_id}");

	log::trace!("[tron::tx] Waiting for receipt for tx_id={}", tx_id);
	wait_for_receipt(&client.tron_api, &tx_id).await
}

/// Poll `getTransactionInfoById` until a receipt appears or we time out.
///
/// TRON blocks are produced every ~3 seconds.  We poll every 4 seconds for up
/// to 5 minutes.
pub async fn wait_for_receipt(api: &TronApi, tx_id: &str) -> anyhow::Result<TransactionInfo> {
	log::trace!("[tron::tx] wait_for_receipt: starting poll for tx_id={}", tx_id);

	let poll_interval = Duration::from_secs(4);
	let max_duration = Duration::from_secs(5 * 60);
	let start = tokio::time::Instant::now();

	let mut attempt = 0;
	loop {
		attempt += 1;
		let elapsed = start.elapsed();

		if elapsed >= max_duration {
			log::error!(
				"[tron::tx] wait_for_receipt: timeout after {} attempts ({} seconds) for tx: {}",
				attempt,
				elapsed.as_secs(),
				tx_id
			);
			return Err(anyhow!("Transaction receipt not found after 5 min for tx: {tx_id}"));
		}

		log::trace!("[tron::tx] wait_for_receipt: attempt {} for tx_id={}", attempt, tx_id);

		match api.get_transaction_info(tx_id).await {
			Ok(Some(info)) => {
				log::trace!("[tron::tx] wait_for_receipt: receipt found after {} attempts ({} seconds) for tx: {}",
					attempt, elapsed.as_secs(), tx_id);
				log::trace!("[tron] Receipt found for tx: {tx_id}");
				return Ok(info);
			},
			Ok(None) => {
				log::trace!(
					"[tron] Receipt not yet available for {tx_id}, retrying in {}s (attempt {})",
					poll_interval.as_secs(),
					attempt,
				);
			},
			Err(err) => {
				log::warn!(
					"[tron] Error querying receipt for {tx_id} (attempt {}): {err:#}, will retry",
					attempt
				);
			},
		}

		tokio::time::sleep(poll_interval).await;
	}
}

/// Extract commitment hashes from the event logs in a [`TransactionInfo`].
///
/// Looks for `PostRequestHandled(bytes32 indexed commitment, address relayer)`
/// and `PostResponseHandled(bytes32 indexed commitment, address relayer)` events.
/// The commitment is the first **indexed** topic (topics[1]).
fn extract_commitment_hashes(info: &TransactionInfo) -> BTreeSet<H256> {
	log::trace!(
		"[tron::tx] extract_commitment_hashes: processing {} log entries for tx {}",
		info.log.len(),
		info.id
	);

	let request_topic = H256::from(keccak_256(b"PostRequestHandled(bytes32,address)"));
	let response_topic = H256::from(keccak_256(b"PostResponseHandled(bytes32,address)"));

	log::trace!("[tron::tx] Looking for request_topic: {:?}", request_topic);
	log::trace!("[tron::tx] Looking for response_topic: {:?}", response_topic);

	let mut hashes = BTreeSet::new();

	for (idx, log_entry) in info.log.iter().enumerate() {
		log::trace!(
			"[tron::tx] Processing log entry {} with {} topics",
			idx,
			log_entry.topics.len()
		);

		if log_entry.topics.is_empty() {
			log::trace!("[tron::tx] Log entry {} has no topics, skipping", idx);
			continue;
		}

		// topics[0] is the event signature hash.
		let topic0 = match hex::decode(&log_entry.topics[0]) {
			Ok(bytes) if bytes.len() == 32 => {
				let h = H256::from_slice(&bytes);
				log::trace!("[tron::tx] Log entry {} topic0: {:?}", idx, h);
				h
			},
			Ok(bytes) => {
				log::trace!(
					"[tron::tx] Log entry {} topic0 has wrong length: {}",
					idx,
					bytes.len()
				);
				continue;
			},
			Err(e) => {
				log::trace!("[tron::tx] Log entry {} topic0 hex decode failed: {}", idx, e);
				continue;
			},
		};

		if topic0 != request_topic && topic0 != response_topic {
			log::trace!(
				"[tron::tx] Log entry {} topic0 doesn't match request/response topics",
				idx
			);
			continue;
		}

		log::trace!(
			"[tron::tx] Log entry {} matches {} event",
			idx,
			if topic0 == request_topic { "PostRequestHandled" } else { "PostResponseHandled" }
		);

		// topics[1] is the indexed `commitment` parameter.
		if log_entry.topics.len() >= 2 {
			if let Ok(bytes) = hex::decode(&log_entry.topics[1]) {
				if bytes.len() == 32 {
					let commitment = H256::from_slice(&bytes);
					log::trace!("[tron::tx] Extracted commitment: {:?}", commitment);
					hashes.insert(commitment);
				} else {
					log::trace!(
						"[tron::tx] Log entry {} topic1 has wrong length: {}",
						idx,
						bytes.len()
					);
				}
			} else {
				log::trace!("[tron::tx] Log entry {} topic1 hex decode failed", idx);
			}
		} else {
			log::trace!("[tron::tx] Log entry {} doesn't have topic1", idx);
		}
	}

	log::trace!("[tron::tx] extract_commitment_hashes: extracted {} commitments", hashes.len());
	hashes
}

/// Try to decode a human-readable revert reason from a failed transaction.
fn decode_revert_message(info: &TransactionInfo) -> String {
	log::trace!("[tron::tx] decode_revert_message for tx {}", info.id);
	log::trace!("[tron::tx] res_message: {:?}", info.res_message);
	log::trace!("[tron::tx] result: {:?}", info.result);

	let decoded = info
		.res_message
		.as_deref()
		.and_then(|hex_msg| {
			log::trace!("[tron::tx] Attempting to decode hex res_message: {}", hex_msg);
			hex::decode(hex_msg).ok()
		})
		.and_then(|bytes| {
			log::trace!("[tron::tx] Attempting UTF-8 decode of {} bytes", bytes.len());
			String::from_utf8(bytes).ok()
		})
		.unwrap_or_else(|| {
			let fallback = info.result.clone().unwrap_or_else(|| "unknown error".into());
			log::trace!("[tron::tx] Using fallback revert message: {}", fallback);
			fallback
		});

	log::trace!("[tron::tx] Decoded revert message: {}", decoded);
	decoded
}

/// Small helper to POST JSON to a TRON endpoint and deserialize the response.
///
/// This is used instead of [`TronApi::trigger_smart_contract`] because we need
/// to send a `data` field (raw calldata) which the existing
/// [`TriggerContractRequest`](api::TriggerContractRequest) does not expose.
async fn post_json<Req: Serialize, Res: serde::de::DeserializeOwned>(
	base_url: &str,
	path: &str,
	body: &Req,
	api_key: Option<&str>,
) -> anyhow::Result<Res> {
	let url = format!("{base_url}{path}");
	log::trace!("[tron::tx] post_json: POST {}", url);

	let client = reqwest::Client::new();
	let mut builder = client.post(&url).json(body);

	if let Some(key) = api_key {
		log::trace!("[tron::tx] post_json: Adding TRON-PRO-API-KEY header");
		builder = builder.header("TRON-PRO-API-KEY", key);
	}

	log::trace!("[tron::tx] post_json: Sending request to {}", url);
	let resp = builder.send().await.with_context(|| format!("POST {url} failed"))?;

	let status = resp.status();
	log::trace!("[tron::tx] post_json: Received HTTP {} from {}", status, url);

	if !status.is_success() {
		let text = resp.text().await.unwrap_or_default();
		log::error!("[tron::tx] post_json: HTTP error {}: {}", status, text);
		return Err(anyhow!("POST {url} returned HTTP {status}: {text}"));
	}

	let text = resp.text().await.context("failed to read response body")?;
	log::trace!("[tron::tx] post_json: Response body length: {} bytes", text.len());

	serde_json::from_str(&text)
		.with_context(|| format!("POST {url}: failed to deserialize: {text}"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_extract_commitment_hashes_empty_logs() {
		let info = TransactionInfo {
			id: "abc".into(),
			block_number: 1,
			block_timestamp: 0,
			fee: 0,
			result: None,
			res_message: None,
			contract_result: vec![],
			contract_address: None,
			receipt: None,
			log: vec![],
		};
		assert!(extract_commitment_hashes(&info).is_empty());
	}

	#[test]
	fn test_extract_commitment_hashes_with_request_event() {
		let topic0 = hex::encode(keccak_256(b"PostRequestHandled(bytes32,address)"));
		let commitment = H256::random();
		let topic1 = hex::encode(commitment.as_bytes());

		let info = TransactionInfo {
			id: "def".into(),
			block_number: 42,
			block_timestamp: 0,
			fee: 0,
			result: None,
			res_message: None,
			contract_result: vec![],
			contract_address: None,
			receipt: None,
			log: vec![crate::api::TransactionLog {
				address: String::new(),
				topics: vec![topic0, topic1],
				data: String::new(),
			}],
		};

		let hashes = extract_commitment_hashes(&info);
		assert_eq!(hashes.len(), 1);
		assert!(hashes.contains(&commitment));
	}

	#[test]
	fn test_decode_revert_message_hex() {
		let info = TransactionInfo {
			id: String::new(),
			block_number: 0,
			block_timestamp: 0,
			fee: 0,
			result: None,
			res_message: Some(hex::encode("insufficient energy")),
			contract_result: vec![],
			contract_address: None,
			receipt: None,
			log: vec![],
		};
		assert_eq!(decode_revert_message(&info), "insufficient energy");
	}

	#[test]
	fn test_decode_revert_message_fallback() {
		let info = TransactionInfo {
			id: String::new(),
			block_number: 0,
			block_timestamp: 0,
			fee: 0,
			result: Some("OUT_OF_ENERGY".into()),
			res_message: None,
			contract_result: vec![],
			contract_address: None,
			receipt: None,
			log: vec![],
		};
		assert_eq!(decode_revert_message(&info), "OUT_OF_ENERGY");
	}
}

/// Estimate the energy and bandwidth required for a transaction.
///
/// Uses `triggerConstantContract` to simulate the transaction and extract
/// resource usage estimates.
///
/// # Arguments
/// * `client` - The TronClient
/// * `calldata_hex` - Hex-encoded calldata (no 0x prefix)
///
/// # Returns
/// A tuple of (estimated_energy, estimated_bandwidth)
async fn estimate_transaction_energy(
	client: &TronClient,
	calldata_hex: &str,
) -> anyhow::Result<u64> {
	log::trace!("[tron::tx] Estimating transaction energy");

	// Use triggerConstantContract to simulate the call
	let trigger_req = TriggerContractRequest {
		owner_address: client.owner_address.clone(),
		contract_address: client.handler_address.clone(),
		function_selector: String::new(), // Empty when using raw data
		parameter: calldata_hex.to_string(),
		fee_limit: Some(client.fee_limit),
		call_value: Some(0),
		visible: Some(false),
	};

	let resp: TriggerConstantContractResponse = client
		.tron_api
		.trigger_constant_contract(&trigger_req)
		.await
		.context("Failed to estimate transaction energy via triggerConstantContract")?;

	// Check if the call would succeed
	if !resp.result.result {
		let msg = resp.result.message.as_deref().unwrap_or("unknown error");
		return Err(anyhow!(
			"Energy estimation failed: transaction simulation reverted with: {}",
			msg
		));
	}

	let estimated_energy = resp.energy_used + resp.energy_penalty;

	// Add 20% safety margin
	let energy_with_margin = (estimated_energy as f64 * 1.2) as u64;

	log::trace!(
		"[tron::tx] Raw energy estimate: {}, with 20% margin: {}",
		estimated_energy,
		energy_with_margin
	);

	Ok(energy_with_margin)
}

/// Purchase energy via the CatFee API.
///
/// This executes the complete purchase flow: create order and wait for confirmation.
///
/// # Arguments
/// * `catfee_client` - The CatFee API client
/// * `receiver_address` - The address that will receive the energy (hex or base58 format)
/// * `energy_required` - Amount of energy needed
async fn purchase_energy_via_catfee(
	catfee_client: &CatFeeClient,
	receiver_address: &str,
	energy_required: u64,
) -> anyhow::Result<()> {
	log::info!("[tron] Purchasing {} energy units via CatFee", energy_required);

	// Convert TRON hex address (41-prefixed) to base58 if needed
	let receiver_base58 = if is_base58_address(receiver_address) {
		// Already in base58 format
		receiver_address.to_string()
	} else {
		// Convert from hex to base58
		hex_to_base58(receiver_address)
			.context("Failed to convert TRON address from hex to base58")?
	};

	log::trace!("[tron] Using receiver address (base58): {}", receiver_base58);

	// Maximum wait time for order completion (3 minutes)
	let max_wait = Duration::from_secs(180);

	// Period: 1 hour (sufficient for immediate transaction)
	let period_hours = 1;

	let _result = catfee_client
		.purchase_energy(energy_required, &receiver_base58, period_hours, max_wait)
		.await
		.context("Failed to purchase energy via CatFee")?;

	log::info!("[tron] Energy purchase completed successfully");

	Ok(())
}
