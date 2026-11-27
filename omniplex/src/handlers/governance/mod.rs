//! The governance forum's tables: OpenGov referenda with their votes and delegations, technical
//! committee motions, whitelisted calls, preimages and treasury spends, from substrate-eth-rpc's
//! `governance` logs. A chain entry with `handlers = "governance"` runs these handlers and
//! nothing else. Storage is read at the event's block, so the history they keep is exact.

mod motions;
mod preimages;
mod referenda;
mod treasury;
mod voting;

use anyhow::Context as _;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::{Map, Value};
use sp_crypto_hashing::{blake2_128, twox_128, twox_64};
use tokio::sync::OnceCell;
use tokio_postgres::Transaction;

use crate::{
	chains::Chain,
	handlers::{abi::pallets::Governance, pallets::read_u128, Handlers},
	registry::{EventContext, EventHandler, LogMeta, Source, Subscription},
	store::timestamp_ms,
};

pub fn handlers() -> Handlers {
	Handlers {
		subscriptions: vec![Subscription::new(Source::Pallet("governance"), Event)],
		hooks: vec![],
	}
}

pub struct Event;

#[async_trait]
impl EventHandler for Event {
	type Event = Governance::Event;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let handled = match event.section.as_str() {
			"referenda" => referenda::METHODS.contains(&event.method.as_str()),
			"convictionVoting" => voting::METHODS.contains(&event.method.as_str()),
			"scheduler" => event.method == "Dispatched",
			"technicalCollective" => motions::METHODS.contains(&event.method.as_str()),
			"whitelist" => true,
			"preimage" => preimages::METHODS.contains(&event.method.as_str()),
			"treasury" => treasury::METHODS.contains(&event.method.as_str()),
			_ => false,
		};
		if !handled {
			return Ok(());
		}
		let ev = Context::new(ctx.chain, ctx.db, ctx.log, &event).await?;
		match event.section.as_str() {
			"referenda" => referenda::handle(&ev).await,
			"convictionVoting" => voting::handle(&ev).await,
			"scheduler" => referenda::dispatched(&ev).await,
			"technicalCollective" => motions::handle(&ev).await,
			"whitelist" => motions::whitelist(&ev).await,
			"preimage" => preimages::handle(&ev).await,
			_ => treasury::handle(&ev).await,
		}
		.with_context(|| format!("{}.{} at block {}", event.section, event.method, ev.block))
	}
}

/** One governance event, with what every handler stores about where it happened. */
pub(crate) struct Context<'a> {
	pub chain: &'a Chain,
	pub db: &'a Transaction<'a>,
	pub method: String,
	/** The event's arguments by name, as the timelines store them. */
	pub args: Map<String, Value>,
	/** The same arguments in order, for events whose fields have no names. */
	pub values: Vec<Value>,
	pub block: u64,
	pub position: Position,
	/** `Referenda::Tracks` and the electorate at this block, read once whoever needs them. */
	tracks: OnceCell<Vec<(i32, Value)>>,
	electorate: OnceCell<u128>,
}

/** Where an event happened, as the event tables key and show it. */
pub(crate) struct Position {
	/** `{blockNumber}-{eventIndex}` */
	pub id: String,
	pub block_number: i32,
	pub block_hash: String,
	pub extrinsic_index: Option<i32>,
	pub event_index: i32,
	pub timestamp: NaiveDateTime,
}

impl<'a> Context<'a> {
	async fn new(
		chain: &'a Chain,
		db: &'a Transaction<'a>,
		log: &LogMeta,
		event: &Governance::Event,
	) -> anyhow::Result<Self> {
		let args: Vec<Value> = serde_json::from_str::<Option<Vec<Value>>>(&event.args)
			.context("governance event arguments")?
			.unwrap_or_default();
		let named = args
			.iter()
			.filter_map(|arg| Some((arg["name"].as_str()?.to_string(), arg["value"].clone())))
			.collect();
		let values = args.into_iter().map(|mut arg| arg["value"].take()).collect();
		let block = log.block_number;
		// Read from storage, since the log carries the block time in seconds only
		let now = chain.storage(&key("Timestamp", "Now"), block).await?;
		let now = now.get(..8).and_then(|ms| ms.try_into().ok()).map(u64::from_le_bytes);
		let position = Position {
			id: format!("{block}-{}", event.index),
			block_number: block as i32,
			block_hash: log.block_hash.to_string(),
			extrinsic_index: event.isExtrinsic.then_some(event.extrinsicIndex as i32),
			event_index: event.index as i32,
			timestamp: timestamp_ms(now.unwrap_or(log.block_timestamp * 1000)),
		};
		Ok(Self {
			chain,
			db,
			method: event.method.clone(),
			args: named,
			values,
			block,
			position,
			tracks: OnceCell::new(),
			electorate: OnceCell::new(),
		})
	}

	/** A named argument. */
	fn arg(&self, name: &str) -> &Value {
		self.args.get(name).unwrap_or(&Value::Null)
	}

	/** The arguments as the event tables store them. */
	fn args_text(&self) -> String {
		Value::Object(self.args.clone()).to_string()
	}

	/** The active issuance, which conviction voting measures turnout and support against. */
	async fn electorate(&self) -> anyhow::Result<u128> {
		let read = |entry| async move {
			let value = self.chain.storage(&key("Balances", entry), self.block).await?;
			anyhow::Ok(read_u128(&value, 0).unwrap_or_default())
		};
		let electorate = self
			.electorate
			.get_or_try_init(|| async {
				let (total, inactive) =
					tokio::try_join!(read("TotalIssuance"), read("InactiveIssuance"))?;
				anyhow::Ok(total.saturating_sub(inactive))
			})
			.await?;
		Ok(*electorate)
	}
}

/** The storage key of a plain storage value, and the prefix of a map's keys. */
fn key(pallet: &str, entry: &str) -> Vec<u8> {
	[twox_128(pallet.as_bytes()), twox_128(entry.as_bytes())].concat()
}

fn blake2_128_concat(data: &[u8]) -> Vec<u8> {
	[&blake2_128(data)[..], data].concat()
}

fn twox_64_concat(data: &[u8]) -> Vec<u8> {
	[&twox_64(data)[..], data].concat()
}

/** A balance or count from the JSON substrate-eth-rpc writes: a number, or a decimal string
 * when it is too large for one. */
fn amount(value: &Value) -> Option<u128> {
	match value {
		Value::Number(n) => n.as_u64().map(u128::from),
		Value::String(text) => match text.strip_prefix("0x") {
			Some(hex) => u128::from_str_radix(hex, 16).ok(),
			None => text.parse().ok(),
		},
		_ => None,
	}
}

fn int(value: &Value) -> Option<i32> {
	value.as_u64().and_then(|n| i32::try_from(n).ok())
}

/** A JSON value as a text column holds it, or `NULL` for a value that wasn't there. */
fn text(value: &Value) -> Option<String> {
	(!value.is_null()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;

	#[test]
	fn amounts_read_as_substrate_eth_rpc_and_polkadot_js_write_them() {
		assert_eq!(amount(&json!(42)), Some(42));
		assert_eq!(amount(&json!("5000000000000000000")), Some(5_000_000_000_000_000_000));
		assert_eq!(
			amount(&json!("0x0000000000000000058d15e176280000")),
			Some(400_000_000_000_000_000)
		);
		assert_eq!(amount(&json!(null)), None);
	}
}
