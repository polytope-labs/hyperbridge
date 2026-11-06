//! The `ismp` pallet's events (`handlers/events/substrateChains`), read through substrate-eth-rpc.
//! Every timestamp here is `Timestamp.Now` in milliseconds, as SubQuery stored it.

use std::sync::Arc;

use alloy::primitives::{Bytes, B256};
use anyhow::{anyhow, bail};
use async_trait::async_trait;
use codec::{Compact, Decode};
use serde_json::{json, Value};

use super::{
	delivery::request_handled,
	extrinsic_hash, from_extrinsic, is_hyperbridge, is_substrate_chain, non_critical,
	requests::{
		defer_update, ensure_get_request, insert_get_response, insert_status, post_request_route,
		update_status, upsert_post_request, GetResponseRow, PostRequestRow, StatusUpdate,
	},
	state_machine::{
		save_state_machine_update, substrate_commitment_timestamp, StateMachineUpdate,
	},
	Shared,
};
use crate::{
	handlers::abi::pallets::Ismp,
	registry::{EventContext, EventHandler},
	services::pending::PendingEntity,
	store::{numeric, Status},
	substrate::encode_address,
};

/** The child trie the pallet keeps request metadata in. */
const ISMP_CHILD_TRIE: &[u8] = b":child_storage:default:ISMPv2";

/**
 * A facade state machine string as SubQuery's `formatChain` printed the same value: Tendermint
 * is `TENDERMINT-`, and a `Relay` state machine was the object it couldn't format.
 */
pub fn format_chain(state_machine: &str) -> String {
	if let Some(id) = state_machine.strip_prefix("TNDRMINT-") {
		format!("TENDERMINT-{id}")
	} else if state_machine.starts_with("RELAY-") {
		"RELAY-[object Object]".to_string()
	} else {
		state_machine.to_string()
	}
}

/**
 * `extractStateMachineIdFromSubstrateEventData`: as `format_chain`, except that it had no case
 * for `Relay` and skipped the event.
 */
pub fn extract_state_machine_id(state_machine: &str) -> Option<String> {
	(!state_machine.starts_with("RELAY-")).then(|| format_chain(state_machine))
}

/** `JSON.parse` of an integer, then `BigInt`: the nearest double, exactly. */
fn js_integer(value: &Value) -> anyhow::Result<u128> {
	let number = value.as_f64().ok_or_else(|| anyhow!("{value} is not a number"))?;
	Ok(number as u128)
}

/** `BigInt(Number(x))` for a `u128`: rounded to a double first. */
fn through_double(value: u128) -> u128 {
	value as f64 as u128
}

/** A SCALE `Vec<u8>`, tolerating a short buffer the way scale-ts `Bytes()` did. */
fn scale_bytes(input: &mut &[u8]) -> anyhow::Result<Vec<u8>> {
	let len = Compact::<u32>::decode(input).map_err(|e| anyhow!("{e}"))?.0 as usize;
	let take = len.min(input.len());
	let (bytes, rest) = input.split_at(take);
	*input = rest;
	Ok(bytes.to_vec())
}

/**
 * The most bytes a relayer's name is made from. A relayer picks its bytes, at any length, and a
 * name too long for a Postgres index would fail on every retry. No address or SCALE `Signature`
 * is this long.
 */
const MAX_RELAYER_BYTES: usize = 256;

fn check_relayer_length(bytes: &[u8]) -> anyhow::Result<()> {
	if bytes.len() > MAX_RELAYER_BYTES {
		bail!("a relayer of {} bytes is no address", bytes.len());
	}
	Ok(())
}

/**
 * `decodeRelayerAddress`: a relayer as SubQuery named it. 20 bytes are lowercase hex; the raw
 * key lengths SS58 accepts are SS58 with the generic prefix 42; other short values are hex;
 * longer values are a SCALE `Signature`, named by its address (EVM) or public key (SS58).
 */
pub fn decode_relayer_address(bytes: &[u8]) -> anyhow::Result<String> {
	check_relayer_length(bytes)?;
	let hex = |b: &[u8]| alloy::primitives::hex::encode_prefixed(b);
	match bytes.len() {
		0 => return Ok(String::new()),
		20 => return Ok(hex(bytes)),
		1 | 2 | 4 | 8 | 32 => return Ok(encode_address(bytes, 42).expect("length checked")),
		len if len <= 32 => return Ok(hex(bytes)),
		_ => {},
	}
	let mut input = bytes;
	let variant = input[0];
	input = &input[1..];
	let key = scale_bytes(&mut input)?;
	match variant {
		0 => Ok(hex(&key)),
		1 | 2 => encode_address(&key, 42).ok_or_else(|| {
			anyhow!("Invalid AccountId provided, expected 32 bytes, found {}", key.len())
		}),
		other => bail!("unknown Signature variant {other}"),
	}
}

/** `relayer.toString()`: how `GetRequestHandled` named a relayer, as its raw bytes in hex. */
pub fn raw_relayer_name(bytes: &[u8]) -> anyhow::Result<String> {
	check_relayer_length(bytes)?;
	Ok(Bytes::from(bytes.to_vec()).to_string())
}

fn hex_hash(hash: &B256) -> String {
	format!("{hash:#x}")
}

/** S1 `handleIsmpStateMachineUpdatedEvent`. */
pub struct StateMachineUpdated(pub Arc<Shared>);

#[async_trait]
impl EventHandler for StateMachineUpdated {
	type Event = Ismp::StateMachineUpdated;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let Some(state_machine_id) = extract_state_machine_id(&event.stateMachineId.stateId) else {
			tracing::warn!(id = %event.stateMachineId.stateId, "state machine SubQuery could not name");
			return Ok(());
		};
		let consensus_state_id =
			String::from_utf8_lossy(event.stateMachineId.consensusStateId.as_slice()).into_owned();
		let host = ctx.chain.state_machine_id();
		let log = ctx.log;
		let height = event.latestHeight;
		let (transaction_hash, transaction_index) = if from_extrinsic(log) {
			(hex_hash(&log.transaction_hash), log.transaction_index)
		} else {
			(String::new(), 0)
		};
		let timestamp = self.0.substrate_timestamp_ms(ctx).await?;
		let commitment = match substrate_commitment_timestamp(
			ctx.chain,
			&state_machine_id,
			&consensus_state_id,
			height,
			log.block_number,
		)
		.await
		{
			Ok(Some(commitment)) => commitment,
			Ok(None) => return Ok(()),
			Err(err) if super::is_transient(&err) => return Err(err),
			Err(err) => {
				tracing::warn!("state commitment for {state_machine_id}: {err:#}");
				return Ok(());
			},
		};
		save_state_machine_update(
			ctx.db,
			&StateMachineUpdate {
				id: format!("{state_machine_id}-{transaction_hash}-{height}"),
				state_machine_id: &state_machine_id,
				height,
				chain: host,
				block_hash: &hex_hash(&log.block_hash),
				block_number: log.block_number,
				transaction_hash: &transaction_hash,
				transaction_index,
				commitment_timestamp: alloy::primitives::U256::from(commitment),
				timestamp,
			},
		)
		.await
	}
}

/** The fee `RequestMetadata { offchain: { u64, u64 }, fee: { payer: [u8; 32], fee: u128 }, .. }`
 * holds. */
fn request_metadata_fee(raw: &[u8]) -> anyhow::Result<u128> {
	let bytes: [u8; 16] = raw
		.get(48..64)
		.and_then(|b| b.try_into().ok())
		.ok_or_else(|| anyhow!("RequestMetadata is {} bytes", raw.len()))?;
	Ok(u128::from_le_bytes(bytes))
}

/** S2 `handleSubstrateRequestEvent`: POST requests that start on this chain. */
pub struct Request(pub Arc<Shared>);

#[async_trait]
impl EventHandler for Request {
	type Event = Ismp::Request;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let source = format_chain(&event.sourceChain);
		let dest = format_chain(&event.destChain);
		let host = ctx.chain.state_machine_id();
		if !is_substrate_chain(&source) || (!is_hyperbridge(&source) && is_hyperbridge(host)) {
			return Ok(());
		}
		let Some(source_chain) = ctx.indexer.chain_by_state_machine(&source) else {
			return Ok(());
		};
		let commitment = hex_hash(&event.commitment);
		let requests: Vec<Value> = source_chain
			.substrate()?
			.call("ismp_queryRequests", json!([[{ "commitment": commitment }]]))
			.await?;
		let request = match requests.into_iter().next() {
			Some(request) => Some(request),
			None if source_chain.name == ctx.chain.name => {
				let block = ctx.log.block_number;
				match dispatched_request(ctx.chain, block, &event.sourceChain, event.requestNonce)
					.await
				{
					Ok(request) => request,
					Err(err) if super::is_transient(&err) => return Err(err),
					Err(err) => {
						tracing::warn!(%commitment, "reading the Request from block {block}: {err:#}");
						return Ok(());
					},
				}
			},
			None => None,
		};
		let Some(request) = request else {
			tracing::warn!(%commitment, "Request event for a request its chain doesn't have");
			return Ok(());
		};
		let Some(post) = request.get("Post") else { return Ok(()) };
		let text = |field: &str| post.get(field).and_then(Value::as_str).unwrap_or_default();

		let mut key = b"RequestCommitments".to_vec();
		key.extend_from_slice(event.commitment.as_slice());
		// Read where the request was dispatched; the TypeScript read the node's head.
		let block = if source_chain.name == ctx.chain.name {
			ctx.log.block_number
		} else {
			alloy::providers::Provider::get_block_number(&source_chain.provider).await?
		};
		let metadata = source_chain.child_storage(ISMP_CHILD_TRIE, &key, block).await?;
		let fee =
			if metadata.is_empty() { 0 } else { through_double(request_metadata_fee(&metadata)?) };

		let timestamp = self.0.substrate_timestamp_ms(ctx).await?;
		let nonce = post.get("nonce").map(js_integer).transpose()?.unwrap_or(0);
		let timeout = post.get("timeoutTimestamp").map(js_integer).transpose()?.unwrap_or(0);
		upsert_post_request(
			ctx.db,
			&PostRequestRow {
				commitment: &commitment,
				chain: host,
				source: &source,
				dest: &dest,
				from: text("from"),
				to: text("to"),
				body: text("body"),
				nonce: numeric(nonce),
				timeout_timestamp: numeric(timeout),
				fee: numeric(fee),
				block_timestamp: timestamp,
			},
		)
		.await?;
		insert_status(
			ctx.db,
			"request_status_metadata",
			&StatusUpdate {
				commitment: &commitment,
				status: Status::Source,
				chain: host,
				timestamp,
				block_number: ctx.log.block_number,
				block_hash: &hex_hash(&ctx.log.block_hash),
				transaction_hash: &extrinsic_hash(ctx.log),
			},
		)
		.await
	}
}

/**
 * The request dispatched in `block` from `source` with `nonce`, as `ismp_queryRequests` returns
 * one. That reads the head, where a request that timed out is gone: a timeout deletes the
 * request's commitment, but not the request itself, so the block's ISMP events still have it.
 */
async fn dispatched_request(
	chain: &crate::chains::Chain,
	block: u64,
	source: &str,
	nonce: u64,
) -> anyhow::Result<Option<Value>> {
	let events: std::collections::HashMap<String, Vec<Value>> =
		chain.substrate()?.call("ismp_queryEvents", json!([block, block])).await?;
	Ok(events.into_values().flatten().find_map(|event| {
		let (kind, request) = event.as_object()?.iter().next()?;
		let kind = match kind.as_str() {
			"PostRequest" => "Post",
			"GetRequest" => "Get",
			_ => return None,
		};
		let dispatched =
			request.get("source")?.as_str()? == source && request.get("nonce")?.as_u64()? == nonce;
		dispatched.then(|| json!({ kind: request }))
	}))
}

/** S3 `handleSubstrateResponseEvent`: GET responses. POST responses are not stored. */
pub struct Response(pub Arc<Shared>);

#[async_trait]
impl EventHandler for Response {
	type Event = Ismp::Response;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let host = ctx.chain.state_machine_id();
		let timestamp = self.0.substrate_timestamp_ms(ctx).await?;
		let commitment = hex_hash(&event.commitment);
		let responses: Vec<Value> = ctx
			.chain
			.substrate()?
			.call("ismp_queryResponses", json!([[{ "commitment": commitment }]]))
			.await?;
		let Some(response) = responses.first() else { return Ok(()) };
		let Some(get) = response.get("get").filter(|get| !is_falsy(get)) else { return Ok(()) };
		let values = response
			.get("values")
			.and_then(Value::as_array)
			.map(|values| {
				values
					.iter()
					.map(|value| value.get("value").and_then(Value::as_str).map(str::to_string))
					.collect()
			})
			.unwrap_or_default();
		let timeout = get.get("timeoutTimestamp").map(js_integer).transpose()?.unwrap_or(0);
		insert_get_response(
			ctx.db,
			&GetResponseRow {
				commitment: &commitment,
				chain: host,
				request: &hex_hash(&event.reqCommitment),
				values,
				timeout_timestamp: numeric(timeout),
				block_timestamp: timestamp,
				block_number: ctx.log.block_number,
				block_hash: &hex_hash(&ctx.log.block_hash),
				transaction_hash: &extrinsic_hash(ctx.log),
			},
		)
		.await
	}
}

/** JavaScript truthiness of a JSON value. */
fn is_falsy(value: &Value) -> bool {
	match value {
		Value::Null => true,
		Value::Bool(b) => !b,
		Value::Number(n) => n.as_f64() == Some(0.0),
		Value::String(s) => s.is_empty(),
		_ => false,
	}
}

fn status_update<'a>(
	ctx: &'a EventContext<'_>,
	commitment: &'a str,
	status: Status,
	timestamp: u64,
	block_hash: &'a str,
	transaction_hash: &'a str,
) -> StatusUpdate<'a> {
	StatusUpdate {
		commitment,
		status,
		chain: ctx.chain.state_machine_id(),
		timestamp,
		block_number: ctx.log.block_number,
		block_hash,
		transaction_hash,
	}
}

/** S4 `handleSubstratePostRequestHandledEvent`. */
pub struct PostRequestHandled(pub Arc<Shared>);

#[async_trait]
impl EventHandler for PostRequestHandled {
	type Event = Ismp::PostRequestHandled;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !from_extrinsic(ctx.log) {
			return Ok(());
		}
		let host = ctx.chain.state_machine_id();
		let timestamp = self.0.substrate_timestamp_ms(ctx).await?;
		let commitment = hex_hash(&event.commitment);
		// Only Hyperbridge routes requests; on any other chain a handled request has arrived.
		let route = post_request_route(ctx.db, &commitment).await?;
		let dest = route.as_ref().and_then(|(_, dest)| dest.as_deref());
		let status = if dest == Some(host) || !is_hyperbridge(host) {
			Status::Destination
		} else {
			Status::HyperbridgeDelivered
		};
		let block_hash = hex_hash(&ctx.log.block_hash);
		let tx = extrinsic_hash(ctx.log);
		let update = status_update(ctx, &commitment, status, timestamp, &block_hash, &tx);
		// A request another chain hasn't indexed waits, and the flush makes a waiting
		// `HYPERBRIDGE_DELIVERED` its `DESTINATION` if the request is for Hyperbridge. It waits
		// even if the request is indexed by now, which would skip that check. The TypeScript
		// dropped the status.
		match route {
			Some(_) => update_status(ctx.db, PendingEntity::Request, &update).await?,
			None => defer_update(ctx.db, PendingEntity::Request, &update).await?,
		}
		// The relayer picks the bytes it's named by, so bytes that don't decode lose only the
		// stats. The TypeScript decoded them first and failed the block.
		let stats = async {
			let relayer = decode_relayer_address(&event.relayer)?;
			request_handled(ctx, &relayer, host, timestamp, None).await
		};
		non_critical(ctx, "PostRequestHandled stats", stats).await
	}
}

/** S5 `handleSubstratePostRequestTimeoutHandledEvent`. */
pub struct PostRequestTimeoutHandled(pub Arc<Shared>);

#[async_trait]
impl EventHandler for PostRequestTimeoutHandled {
	type Event = Ismp::PostRequestTimeoutHandled;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !from_extrinsic(ctx.log) {
			return Ok(());
		}
		let host = ctx.chain.state_machine_id();
		let timestamp = self.0.substrate_timestamp_ms(ctx).await?;
		let commitment = hex_hash(&event.commitment);
		// The event names the request's source, so a request that isn't indexed yet can wait with
		// its status. The TypeScript read the source from the request, and dropped the status
		// while the request wasn't indexed.
		let status = if format_chain(&event.source) == host {
			Status::TimedOut
		} else {
			Status::HyperbridgeTimedOut
		};
		let block_hash = hex_hash(&ctx.log.block_hash);
		let tx = extrinsic_hash(ctx.log);
		let update = status_update(ctx, &commitment, status, timestamp, &block_hash, &tx);
		update_status(ctx.db, PendingEntity::Request, &update).await
	}
}

/** S6 `handleSubstrateGetRequestHandledEvent`. */
pub struct GetRequestHandled(pub Arc<Shared>);

#[async_trait]
impl EventHandler for GetRequestHandled {
	type Event = Ismp::GetRequestHandled;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !from_extrinsic(ctx.log) {
			return Ok(());
		}
		let host = ctx.chain.state_machine_id();
		let timestamp = self.0.substrate_timestamp_ms(ctx).await?;
		let commitment = hex_hash(&event.commitment);
		let source = ensure_get_request(ctx.db, &commitment, timestamp).await?;
		let status =
			if source == host { Status::Destination } else { Status::HyperbridgeDelivered };
		let block_hash = hex_hash(&ctx.log.block_hash);
		let tx = extrinsic_hash(ctx.log);
		let update = status_update(ctx, &commitment, status, timestamp, &block_hash, &tx);
		update_status(ctx.db, PendingEntity::GetRequest, &update).await?;
		let stats = async {
			let relayer = raw_relayer_name(&event.relayer)?;
			request_handled(ctx, &relayer, host, timestamp, None).await
		};
		non_critical(ctx, "GetRequestHandled stats", stats).await
	}
}

/** S7 `handleSubstrateGetRequestTimeoutHandledEvent`. */
pub struct GetRequestTimeoutHandled(pub Arc<Shared>);

#[async_trait]
impl EventHandler for GetRequestTimeoutHandled {
	type Event = Ismp::GetRequestTimeoutHandled;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !from_extrinsic(ctx.log) {
			return Ok(());
		}
		let host = ctx.chain.state_machine_id();
		let commitment = hex_hash(&event.commitment);
		// As in `PostRequestTimeoutHandled`, the status comes from the event's source.
		let status = if format_chain(&event.source) == host {
			Status::TimedOut
		} else {
			Status::HyperbridgeTimedOut
		};
		let timestamp = self.0.substrate_timestamp_ms(ctx).await?;
		let block_hash = hex_hash(&ctx.log.block_hash);
		let tx = extrinsic_hash(ctx.log);
		let update = status_update(ctx, &commitment, status, timestamp, &block_hash, &tx);
		update_status(ctx.db, PendingEntity::GetRequest, &update).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::substrate::ss58;

	fn bytes(hex: &str) -> Vec<u8> {
		alloy::primitives::hex::decode(hex).unwrap()
	}

	fn account(hex: &str) -> [u8; 32] {
		bytes(hex).try_into().unwrap()
	}

	const KEY: &str = "588177cf191e890d83b10e9653cfd663c8a382d5afce8518636dd9565c22a631";

	#[test]
	fn raw_keys_are_ss58_and_evm_addresses_hex() {
		assert_eq!(decode_relayer_address(&bytes(KEY)).unwrap(), ss58(&account(KEY), 42));
		let evm = "0xd19651565559e361e984640ce5f69f1575149e63";
		assert_eq!(decode_relayer_address(&bytes(evm)).unwrap(), evm);
		assert_eq!(decode_relayer_address(&[]).unwrap(), "");
		assert_eq!(decode_relayer_address(&[1, 2, 3]).unwrap(), "0x010203");
	}

	#[test]
	fn signatures_name_their_signer() {
		let evm = "d19651565559E361e984640ce5F69F1575149E63";
		let encoded = bytes(&format!("0050{evm}0501{}", "00".repeat(65)));
		assert_eq!(decode_relayer_address(&encoded).unwrap(), format!("0x{}", evm.to_lowercase()));

		let sr25519 = bytes(&format!("0180{KEY}0101{}", "00".repeat(64)));
		assert_eq!(decode_relayer_address(&sr25519).unwrap(), ss58(&account(KEY), 42));

		let ed_key = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
		let ed25519 = bytes(&format!("0280{ed_key}0101{}", "00".repeat(64)));
		assert_eq!(decode_relayer_address(&ed25519).unwrap(), ss58(&account(ed_key), 42));

		let signed = bytes("01805254bab90041a14167817f565825aaefd6067695b343ec57a0263be9986b6b5c0101d8609bd3642cfa496af519019c3d5f5da538d016066e40ae9ffa0e04cb36e75d0b7e67a95f66a6dbd346b4c9e5fdcfb3b0bacc184c862b024e5e49fcc3a72d87");
		let signer = "5254bab90041a14167817f565825aaefd6067695b343ec57a0263be9986b6b5c";
		assert_eq!(decode_relayer_address(&signed).unwrap(), ss58(&account(signer), 42));
	}

	#[test]
	fn relayers_longer_than_any_signature_name_nobody() {
		let mut crafted = vec![0u8, 0xa1, 0x1f];
		crafted.extend([0x42; 2000]);
		assert!(decode_relayer_address(&crafted).is_err());
		assert!(raw_relayer_name(&crafted).is_err());
		assert_eq!(raw_relayer_name(&[0xab; 20]).unwrap(), format!("0x{}", "ab".repeat(20)));
	}

	#[test]
	fn state_machine_strings_read_as_subquery_wrote_them() {
		assert_eq!(format_chain("POLKADOT-3367"), "POLKADOT-3367");
		assert_eq!(format_chain("TNDRMINT-osmo"), "TENDERMINT-osmo");
		assert_eq!(format_chain("RELAY-pase-1000"), "RELAY-[object Object]");
		assert_eq!(extract_state_machine_id("RELAY-pase-1000"), None);
		assert_eq!(extract_state_machine_id("EVM-1").as_deref(), Some("EVM-1"));
	}

	#[test]
	fn fees_lose_precision_through_a_double() {
		assert_eq!(through_double(1_000_000_000_000_000_001), 1_000_000_000_000_000_000);
		assert_eq!(through_double(12_345), 12_345);
		let mut raw = vec![0u8; 48];
		raw.extend(77u128.to_le_bytes());
		raw.push(0);
		assert_eq!(request_metadata_fee(&raw).unwrap(), 77);
	}

	#[test]
	fn json_integers_read_like_javascript() {
		assert_eq!(js_integer(&json!(9007199254740993u64)).unwrap(), 9007199254740992);
		assert_eq!(js_integer(&json!(1_700_000_000)).unwrap(), 1_700_000_000);
	}
}
