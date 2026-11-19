//! The block explorer: every block, extrinsic, event and balance transfer of a Substrate chain,
//! from substrate-eth-rpc's `explorer` logs, plus the balances of the accounts they touch. An
//! explorer config (`explorer = true`) runs these handlers and nothing else.

use std::{collections::BTreeSet, sync::Arc};

use async_trait::async_trait;
use serde_json::{json, Value};
use sp_crypto_hashing::{blake2_128, twox_128};
use tokio_postgres::Transaction;

use crate::{
	config::{ChainConfig, ChainKind},
	handlers::{
		abi::pallets::{Balances, Explorer},
		pallets::{extrinsic, read_u128},
		Handlers,
	},
	registry::{
		BlockContext, BlockHook, EventContext, EventHandler, LogMeta, Source, Subscription,
	},
	store::numeric,
	substrate::{account_id, encode_address},
};

/** The prefix addresses are stored with, which the Statescan site shows for Hyperbridge. */
const SS58_PREFIX: u8 = 0;

/** `System.Account` keys read per request when seeding the accounts. */
const SEED_PAGE: usize = 1000;

pub fn handlers() -> Handlers {
	Handlers {
		subscriptions: vec![
			Subscription::new(Source::Pallet("explorer"), Block),
			Subscription::new(Source::Pallet("explorer"), Extrinsic),
			Subscription::new(Source::Pallet("explorer"), Event),
			Subscription::new(Source::Pallet("balances"), Transfer),
		],
		hooks: vec![Arc::new(Accounts)],
	}
}

pub struct Block;

#[async_trait]
impl EventHandler for Block {
	type Event = Explorer::Block;

	async fn handle(&self, ctx: &EventContext<'_>, block: Self::Event) -> anyhow::Result<()> {
		write_block(ctx.db, ctx.log, &block).await
	}
}

pub async fn write_block(
	db: &Transaction<'_>,
	log: &LogMeta,
	block: &Explorer::Block,
) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO explorer.blocks (height, hash, parent_hash, state_root, extrinsics_root,
			time, spec_version, extrinsics_count, events_count)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT DO NOTHING",
		&[
			&height(log),
			&log.block_hash.to_string(),
			&block.parentHash.to_string(),
			&block.stateRoot.to_string(),
			&block.extrinsicsRoot.to_string(),
			&time(log),
			&(block.specVersion as i32),
			&(block.extrinsicsCount as i32),
			&(block.eventsCount as i32),
		],
	)
	.await?;
	Ok(())
}

pub struct Extrinsic;

#[async_trait]
impl EventHandler for Extrinsic {
	type Event = Explorer::Extrinsic;

	async fn handle(&self, ctx: &EventContext<'_>, call: Self::Event) -> anyhow::Result<()> {
		write_extrinsic(ctx.db, ctx.log, &call).await
	}
}

pub async fn write_extrinsic(
	db: &Transaction<'_>,
	log: &LogMeta,
	call: &Explorer::Extrinsic,
) -> anyhow::Result<()> {
	let signer = (!call.signer.is_empty())
		.then(|| encode_address(&call.signer, SS58_PREFIX))
		.flatten();
	db.execute(
		"INSERT INTO explorer.extrinsics (height, index, hash, time, signed, success, signer,
			section, method, args, events_count)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) ON CONFLICT DO NOTHING",
		&[
			&height(log),
			&(call.index as i32),
			&log.transaction_hash.to_string(),
			&time(log),
			&call.signed,
			&call.success,
			&signer,
			&call.section,
			&call.method,
			&args(&call.args),
			&(call.eventsCount as i32),
		],
	)
	.await?;
	record_name(db, log, "call", &call.section, &call.method).await
}

pub struct Event;

#[async_trait]
impl EventHandler for Event {
	type Event = Explorer::Event;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		write_event(ctx.db, ctx.log, &event).await
	}
}

pub async fn write_event(
	db: &Transaction<'_>,
	log: &LogMeta,
	event: &Explorer::Event,
) -> anyhow::Result<()> {
	let extrinsic_index = event.isExtrinsic.then_some(event.extrinsicIndex as i32);
	db.execute(
		"INSERT INTO explorer.events (height, index, time, extrinsic_index, section, method,
			args)
		 VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
		&[
			&height(log),
			&(event.index as i32),
			&time(log),
			&extrinsic_index,
			&event.section,
			&event.method,
			&args(&event.args),
		],
	)
	.await?;
	record_name(db, log, "event", &event.section, &event.method).await
}

pub struct Transfer;

#[async_trait]
impl EventHandler for Transfer {
	type Event = Balances::Transfer;

	async fn handle(&self, ctx: &EventContext<'_>, transfer: Self::Event) -> anyhow::Result<()> {
		write_transfer(ctx.db, ctx.log, &transfer).await
	}
}

pub async fn write_transfer(
	db: &Transaction<'_>,
	log: &LogMeta,
	transfer: &Balances::Transfer,
) -> anyhow::Result<()> {
	let address = |account: &[u8]| encode_address(account, SS58_PREFIX).unwrap_or_default();
	let extrinsic_index = extrinsic(log).map(|(index, _)| index as i32);
	db.execute(
		"INSERT INTO explorer.transfers (height, event_index, time, extrinsic_index, \"from\",
			\"to\", amount)
		 VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
		&[
			&height(log),
			&(log.log_index as i32),
			&time(log),
			&extrinsic_index,
			&address(transfer.from.as_slice()),
			&address(transfer.to.as_slice()),
			&numeric(transfer.amount),
		],
	)
	.await?;
	Ok(())
}

/** Re-reads `System.Account` for every account a batch's transfers and extrinsics touched, and
 * the total issuance, at the batch's last block. */
pub struct Accounts;

#[async_trait]
impl BlockHook for Accounts {
	fn applies_to(&self, chain: &ChainConfig) -> bool {
		chain.kind == ChainKind::Substrate
	}

	async fn after_batch(&self, ctx: &BlockContext<'_>) -> anyhow::Result<()> {
		if !seeded(ctx.db).await? {
			seed_accounts(ctx).await?;
		}
		let from = ctx.previous_block.map_or(0, |block| block + 1);
		for address in touched_accounts(ctx.db, from, ctx.block_number).await? {
			let Some(account) = account_id(&address) else { continue };
			let info = ctx.chain.storage(&account_key(&account), ctx.block_number).await?;
			write_account(ctx.db, &address, &info, ctx.block_number).await?;
		}

		let issuance = ctx.chain.storage(&total_issuance_key(), ctx.block_number).await?;
		if let Some(issuance) = read_u128(&issuance, 0) {
			ctx.db
				.execute(
					"INSERT INTO explorer.status (name, value) VALUES ('totalIssuance', $1)
					 ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
					&[&issuance.to_string()],
				)
				.await?;
		}
		Ok(())
	}
}

/**
 * Every address blocks `from..=to` touched: in a transfer, as a signer, or in an event argument
 * typed as an account, which covers every balance change such as a deposit, slash or reserve.
 */
pub async fn touched_accounts(
	db: &Transaction<'_>,
	from: u64,
	to: u64,
) -> anyhow::Result<BTreeSet<String>> {
	let rows = db
		.query(
			"SELECT \"from\" FROM explorer.transfers WHERE height BETWEEN $1 AND $2
			 UNION SELECT \"to\" FROM explorer.transfers WHERE height BETWEEN $1 AND $2
			 UNION SELECT signer FROM explorer.extrinsics
			 WHERE height BETWEEN $1 AND $2 AND signer IS NOT NULL
			 UNION SELECT arg->>'value' FROM explorer.events e, jsonb_array_elements(e.args) arg
			 WHERE e.height BETWEEN $1 AND $2 AND jsonb_typeof(e.args) = 'array'
			   AND arg->>'typeName' LIKE 'AccountId%' AND jsonb_typeof(arg->'value') = 'string'",
			&[&(from as i64), &(to as i64)],
		)
		.await?;
	Ok(rows.iter().filter_map(|row| address(row.get(0))).collect())
}

/** An address as the tables hold it, from an SS58 address or an account's hex in an event. */
fn address(value: String) -> Option<String> {
	match value.strip_prefix("0x") {
		Some(hex) => encode_address(&hex::decode(hex).ok()?, SS58_PREFIX),
		None => Some(value),
	}
}

/** Whether every account the chain held has been read once. */
async fn seeded(db: &Transaction<'_>) -> anyhow::Result<bool> {
	let row = db.query_opt("SELECT 1 FROM explorer.status WHERE name = 'accountsSeeded'", &[]);
	Ok(row.await?.is_some())
}

/**
 * Reads every account the chain holds at the batch's block, once, so accounts that never move,
 * such as those funded at genesis, are listed too. Events keep them current after that.
 */
async fn seed_accounts(ctx: &BlockContext<'_>) -> anyhow::Result<()> {
	let node = ctx.chain.substrate()?;
	let hash: String = node.call("chain_getBlockHash", json!([ctx.block_number])).await?;
	let prefix = format!("0x{}", hex::encode([twox_128(b"System"), twox_128(b"Account")].concat()));
	let mut start: Option<String> = None;
	loop {
		let keys: Vec<String> =
			node.call("state_getKeysPaged", json!([prefix, SEED_PAGE, start, hash])).await?;
		let Some(last) = keys.last().cloned() else { break };
		let sets: Value = node.call("state_queryStorageAt", json!([keys, hash])).await?;
		let changes = sets[0]["changes"].as_array().cloned().unwrap_or_default();
		for change in changes {
			let (Some(key), Some(info)) = (change[0].as_str(), change[1].as_str()) else {
				continue
			};
			let key = hex::decode(key.trim_start_matches("0x"))?;
			let Some(address) =
				key.get(48..).and_then(|account| encode_address(account, SS58_PREFIX))
			else {
				continue;
			};
			let info = hex::decode(info.trim_start_matches("0x"))?;
			write_account(ctx.db, &address, &info, ctx.block_number).await?;
		}
		if keys.len() < SEED_PAGE {
			break;
		}
		start = Some(last);
	}
	ctx.db
		.execute(
			"INSERT INTO explorer.status (name, value) VALUES ('accountsSeeded', $1)",
			&[&ctx.block_number.to_string()],
		)
		.await?;
	Ok(())
}

/** Stores an account's `AccountInfo`, or removes it once the chain has reaped it. */
pub async fn write_account(
	db: &Transaction<'_>,
	address: &str,
	info: &[u8],
	height: u64,
) -> anyhow::Result<()> {
	let counter = |at: usize| {
		info.get(at..at + 4)
			.and_then(|bytes| bytes.try_into().ok())
			.map(|bytes| u32::from_le_bytes(bytes) as i64)
	};
	let (Some(nonce), Some(consumers), Some(providers), Some(sufficients)) =
		(counter(0), counter(4), counter(8), counter(12))
	else {
		db.execute("DELETE FROM explorer.accounts WHERE address = $1", &[&address])
			.await?;
		return Ok(());
	};
	let balance = |at| read_u128(info, at).unwrap_or_default();
	let (free, reserved, frozen) = (balance(16), balance(32), balance(48));
	db.execute(
		"INSERT INTO explorer.accounts (address, nonce, consumers, providers, sufficients, free,
			reserved, frozen, total, height)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		 ON CONFLICT (address) DO UPDATE SET nonce = EXCLUDED.nonce,
			consumers = EXCLUDED.consumers, providers = EXCLUDED.providers,
			sufficients = EXCLUDED.sufficients, free = EXCLUDED.free,
			reserved = EXCLUDED.reserved, frozen = EXCLUDED.frozen, total = EXCLUDED.total,
			height = EXCLUDED.height",
		&[
			&address,
			&nonce,
			&consumers,
			&providers,
			&sufficients,
			&numeric(free),
			&numeric(reserved),
			&numeric(frozen),
			&numeric(free + reserved),
			&(height as i64),
		],
	)
	.await?;
	Ok(())
}

async fn record_name(
	db: &Transaction<'_>,
	log: &LogMeta,
	kind: &str,
	section: &str,
	method: &str,
) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO explorer.call_names (spec_version, kind, section, method)
		 SELECT spec_version, $2, $3, $4 FROM explorer.blocks WHERE height = $1
		 ON CONFLICT DO NOTHING",
		&[&height(log), &kind, &section, &method],
	)
	.await?;
	Ok(())
}

/** `System.Account` for one account. */
fn account_key(account: &[u8; 32]) -> Vec<u8> {
	[&twox_128(b"System")[..], &twox_128(b"Account"), &blake2_128(account), account].concat()
}

fn total_issuance_key() -> Vec<u8> {
	[twox_128(b"Balances"), twox_128(b"TotalIssuance")].concat()
}

/** A string, or a list of strings, longer than this in total is stored as a note of its size.
 * Those are proofs, trie nodes or runtime code, which nobody reads on a page and which would
 * make up most of the database. */
const STRING_LIMIT: usize = 512;

/** Call and event arguments as JSON, or `NULL` when they didn't decode. Every argument keeps its
 * name, type and structure; only its long strings are replaced by their size. */
fn args(json: &str) -> Option<Value> {
	let args: Value = serde_json::from_str(json).ok()?;
	args.is_array().then(|| shorten(args))
}

/** `value` with every long string, and every long list of strings such as a proof's trie
 * nodes, replaced by a note of its size. */
fn shorten(value: Value) -> Value {
	match value {
		Value::String(text) if text.len() > STRING_LIMIT => {
			let bytes = text.strip_prefix("0x").map_or(text.len(), |hex| hex.len() / 2);
			Value::String(format!("{bytes} bytes, not stored"))
		},
		Value::Array(items) if strings_length(&items).is_some_and(|n| n > STRING_LIMIT) =>
			Value::String(format!("{} items, not stored", items.len())),
		Value::Array(items) => Value::Array(items.into_iter().map(shorten).collect()),
		Value::Object(fields) =>
			Value::Object(fields.into_iter().map(|(key, value)| (key, shorten(value))).collect()),
		other => other,
	}
}

/** The total length of a list that holds only strings. */
fn strings_length(items: &[Value]) -> Option<usize> {
	items.iter().map(|item| item.as_str().map(str::len)).sum()
}

fn height(log: &LogMeta) -> i64 {
	log.block_number as i64
}

fn time(log: &LogMeta) -> i64 {
	log.block_timestamp as i64 * 1000
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn storage_keys_match_the_runtime() {
		assert_eq!(
			hex::encode(total_issuance_key()),
			"c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80"
		);
		let key = account_key(&[7; 32]);
		assert_eq!(
			hex::encode(&key[..32]),
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9"
		);
		assert_eq!(&key[48..], &[7; 32]);
	}

	#[test]
	fn long_strings_are_cut_and_the_rest_is_kept() {
		let small = r#"[{"name":"now","type":"Moment","value":1}]"#;
		assert_eq!(args(small), serde_json::from_str(small).ok());
		assert_eq!(args("null"), None);
		let proof = format!("0x{}", "ab".repeat(4000));
		let messages = json!([{ "name": "messages", "type": "Vec<Message>", "value": [{ "request": {
			"proof": { "height": { "height": 53267715 }, "proof": proof },
			"requests": [{ "post": { "source": { "evm": 97 }, "dest": { "evm": 80002 }, "nonce": 716 } }],
		} }] }]);
		let stored = args(&messages.to_string()).unwrap();
		let message = &stored[0]["value"][0]["request"];
		assert_eq!(message["proof"]["proof"], "4000 bytes, not stored");
		assert_eq!(message["proof"]["height"]["height"], 53267715);
		assert_eq!(message["requests"][0]["post"]["nonce"], 716);
		assert_eq!(message["requests"][0]["post"]["dest"]["evm"], 80002);

		let nodes: Vec<String> = (0..20).map(|_| format!("0x{}", "cd".repeat(40))).collect();
		let state = json!([{ "name": "data", "type": "Data", "value": { "trieNodes": nodes } }]);
		assert_eq!(
			args(&state.to_string()).unwrap()[0]["value"]["trieNodes"],
			"20 items, not stored"
		);
	}

	#[test]
	fn addresses_round_trip() {
		let address = encode_address(&[7; 32], SS58_PREFIX).unwrap();
		assert_eq!(account_id(&address), Some([7; 32]));
		assert_eq!(account_id("not an address"), None);
	}
}
