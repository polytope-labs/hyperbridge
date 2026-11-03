//! The BRIDGE token's total and circulating supply, refreshed every 3600 blocks.
//! Ported from `handleBridgeTokenSupplyIndexing` and `services/bridgeTokenSupply.service.ts`.

use async_trait::async_trait;
use codec::Decode;
use serde::Deserialize;
use serde_json::json;
use tokio_postgres::Transaction;

use super::{account_free, is_hyperbridge, read_u128};
use crate::{
	config::ChainConfig,
	registry::{BlockContext, BlockHook},
	store::{normalize_timestamp, numeric, timestamp_to_date},
	substrate::SubstrateRpc,
};

/** SubQuery ran the job on every block whose number is a multiple of this. */
pub const INTERVAL: u64 = 3600;

/** The native token's decimals. */
const DECIMALS: u32 = 12;

const TIMESTAMP_NOW: &str = "0xf0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb";
const TOTAL_ISSUANCE: &str = "0xc2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80";
const INACTIVE_ISSUANCE: &str =
	"0xc2261276cc9d1f8598ea4b6a74b15c2f1ccde6872881f893a21de93dfe970cd5";
const LOCKS_PREFIX: &str = "0xc2261276cc9d1f8598ea4b6a74b15c2f218f26c73add634897550b4003b26bc6";

/** The foundation and team accounts, whose balances and locks are not circulating. */
const FOUNDATION: &str = "1e3df2cbfa10ceadc854bd5aa305462c2f81bbc81cc7f6ef462ce857bd0c90b3";
const TEAM: &str = "5525104d00c439213dcbf428b867f0930754f100b75873382f2103ce5f846163";
const FOUNDATION_ACCOUNT: &str = "0x26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9cc2c6dacd6a81b5cfa93c676700c2ccf1e3df2cbfa10ceadc854bd5aa305462c2f81bbc81cc7f6ef462ce857bd0c90b3";
const TEAM_ACCOUNT: &str = "0x26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9a6f084e8a3b94abf6e45b5ee4bcc89db5525104d00c439213dcbf428b867f0930754f100b75873382f2103ce5f846163";

const KEYS_PAGE: usize = 1000;
const QUERY_BATCH: usize = 100;

/**
 * `handleBridgeTokenSupplyIndexing`. A batch that reaches a multiple of 3600 past the chain's
 * cursor runs the job once, at that block: the supply is read in that block's state and
 * stamped with its `Timestamp.Now`.
 */
pub struct BridgeTokenSupply;

#[async_trait]
impl BlockHook for BridgeTokenSupply {
	fn applies_to(&self, chain: &ChainConfig) -> bool {
		is_hyperbridge(&chain.state_machine_id)
	}

	async fn after_batch(&self, ctx: &BlockContext<'_>) -> anyhow::Result<()> {
		// The cursor still holds the previous batch's last log: it is written after the hooks.
		let previous = ctx
			.db
			.query_opt(
				"SELECT block_number FROM indexer.cursors WHERE network = $1",
				&[&ctx.chain.name],
			)
			.await?
			.map(|row| row.get::<_, i64>(0) as u64);
		let start = ctx.chain.config.start_block;
		let Some(block) = trigger_block(previous, start, ctx.block_number) else {
			return Ok(());
		};
		let rpc = ctx.chain.substrate()?;
		let Some(supply) = read_supply(rpc, block).await? else {
			tracing::warn!(block, "bridge token supply: no TotalIssuance; not updated");
			return Ok(());
		};
		write_supply(ctx.db, &supply).await
	}
}

/**
 * The block to run at: the last multiple of `INTERVAL` in `(previous, last]`, where `previous`
 * is the cursor's block, or the block before `start` on the first batch.
 */
pub fn trigger_block(previous: Option<u64>, start: u64, last: u64) -> Option<u64> {
	let block = last / INTERVAL * INTERVAL;
	let after = match previous {
		Some(previous) => block > previous,
		None => block >= start,
	};
	(after && block <= last).then_some(block)
}

/** What one run writes. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supply {
	/** Whole tokens. */
	pub total: i128,
	/** Whole tokens. */
	pub circulating: i128,
	/** `Timestamp.Now` of the block read, in milliseconds. */
	pub timestamp_ms: u64,
}

/** The raw balances behind one run, in planck. */
#[derive(Debug, Clone, Copy)]
pub struct Balances {
	pub total_issuance: u128,
	pub inactive_issuance: u128,
	pub team_and_foundation: u128,
	pub locks: u128,
}

impl Balances {
	/** `total - (inactive + locks + team)`, both scaled to whole tokens by truncation. */
	pub fn supply(&self, timestamp_ms: u64) -> Supply {
		let scale = 10i128.pow(DECIMALS);
		let total = self.total_issuance as i128;
		let excluded =
			self.inactive_issuance as i128 + self.locks as i128 + self.team_and_foundation as i128;
		Supply { total: total / scale, circulating: (total - excluded) / scale, timestamp_ms }
	}
}

/** Reads the supply in `block`'s state. `None` if `TotalIssuance` is absent. */
pub async fn read_supply(rpc: &SubstrateRpc, block: u64) -> anyhow::Result<Option<Supply>> {
	let hash: String = rpc.call("chain_getBlockHash", json!([block])).await?;
	let Some(total_issuance) = storage(rpc, TOTAL_ISSUANCE, &hash).await? else {
		return Ok(None);
	};
	let balances = Balances {
		total_issuance: read_u128(&total_issuance, 0).unwrap_or(0),
		inactive_issuance: storage(rpc, INACTIVE_ISSUANCE, &hash)
			.await?
			.and_then(|value| read_u128(&value, 0))
			.unwrap_or(0),
		team_and_foundation: team_and_foundation(rpc, &hash).await?,
		locks: total_locks(rpc, &hash).await?,
	};
	let now = storage(rpc, TIMESTAMP_NOW, &hash).await?.unwrap_or_default();
	let timestamp = u64::decode(&mut &now[..]).unwrap_or(0);
	Ok(Some(balances.supply(normalize_timestamp(timestamp))))
}

/** Upserts the single `hyperbridge-token-supply` row. `created_at` is set only on insert. */
pub async fn write_supply(db: &Transaction<'_>, supply: &Supply) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO app.bridge_token_supplies
		   (id, total_supply, circulating_supply, last_updated_at, created_at)
		 VALUES ('hyperbridge-token-supply', $1, $2, $3, $4)
		 ON CONFLICT (id) DO UPDATE SET
		   total_supply = EXCLUDED.total_supply,
		   circulating_supply = EXCLUDED.circulating_supply,
		   last_updated_at = EXCLUDED.last_updated_at",
		&[
			&numeric(supply.total),
			&numeric(supply.circulating),
			&numeric(supply.timestamp_ms),
			&timestamp_to_date(supply.timestamp_ms),
		],
	)
	.await?;
	Ok(())
}

async fn storage(rpc: &SubstrateRpc, key: &str, hash: &str) -> anyhow::Result<Option<Vec<u8>>> {
	let value: Option<String> = rpc.call("state_getStorage", json!([key, hash])).await?;
	value.map(|value| decode_hex(&value)).transpose()
}

fn decode_hex(value: &str) -> anyhow::Result<Vec<u8>> {
	Ok(hex::decode(value.trim_start_matches("0x"))?)
}

#[derive(Deserialize)]
struct StorageChangeSet {
	changes: Vec<(String, Option<String>)>,
}

/** `state_queryStorageAt`: every `(key, value)` present at `hash`. */
async fn query_storage(
	rpc: &SubstrateRpc,
	keys: &[String],
	hash: &str,
) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
	let sets: Vec<StorageChangeSet> = rpc.call("state_queryStorageAt", json!([keys, hash])).await?;
	let mut values = vec![];
	for (key, value) in sets.into_iter().flat_map(|set| set.changes) {
		if let Some(value) = value {
			values.push((key, decode_hex(&value)?));
		}
	}
	Ok(values)
}

/** The free balances of the foundation and team accounts. */
async fn team_and_foundation(rpc: &SubstrateRpc, hash: &str) -> anyhow::Result<u128> {
	let keys = [FOUNDATION_ACCOUNT.to_string(), TEAM_ACCOUNT.to_string()];
	let values = query_storage(rpc, &keys, hash).await?;
	Ok(values.iter().map(|(_, value)| account_free(value).unwrap_or(0)).sum())
}

/** Every `Balances.Locks` amount, except the foundation's and team's. */
async fn total_locks(rpc: &SubstrateRpc, hash: &str) -> anyhow::Result<u128> {
	let mut total = 0u128;
	let mut start: Option<String> = None;
	loop {
		let keys: Vec<String> = rpc
			.call("state_getKeysPaged", json!([LOCKS_PREFIX, KEYS_PAGE, start, hash]))
			.await?;
		for batch in keys.chunks(QUERY_BATCH) {
			for (key, value) in query_storage(rpc, batch, hash).await? {
				total += account_locks(&key, &value);
			}
		}
		if keys.len() < KEYS_PAGE {
			break;
		}
		start = keys.last().cloned();
	}
	Ok(total)
}

/** `Vec<BalanceLock { id: [u8; 8], amount: u128, reasons: u8 }>`. */
#[derive(Decode)]
struct BalanceLock {
	_id: [u8; 8],
	amount: u128,
	_reasons: u8,
}

/** The sum of one account's locks. The team's and foundation's count 0, as do unreadable ones. */
pub fn account_locks(key: &str, value: &[u8]) -> u128 {
	if key.ends_with(FOUNDATION) || key.ends_with(TEAM) {
		return 0;
	}
	match Vec::<BalanceLock>::decode(&mut &value[..]) {
		Ok(locks) => locks.iter().map(|lock| lock.amount).sum(),
		Err(err) => {
			tracing::warn!(key, "bridge token supply: unreadable locks, counted as 0: {err}");
			0
		},
	}
}

#[cfg(test)]
mod tests {
	use codec::Encode;

	use super::*;

	#[test]
	fn runs_once_per_multiple_of_3600() {
		// First batch: a multiple at or after the start block.
		assert_eq!(trigger_block(None, 10_190_400, 10_190_900), None);
		assert_eq!(trigger_block(None, 10_190_400, 10_191_600), Some(10_191_600));
		assert_eq!(trigger_block(None, 10_191_600, 10_191_600), Some(10_191_600));
		// Later batches: a multiple in (previous, last].
		assert_eq!(trigger_block(Some(10_194_000), 0, 10_194_500), None);
		assert_eq!(trigger_block(Some(10_194_999), 0, 10_195_200), Some(10_195_200));
		assert_eq!(trigger_block(Some(10_195_200), 0, 10_195_700), None);
		assert_eq!(trigger_block(Some(10_195_100), 0, 10_195_300), Some(10_195_200));
		// A gap of several intervals runs once, at the latest.
		assert_eq!(trigger_block(Some(10_000), 0, 20_000), Some(18_000));
	}

	#[test]
	fn circulating_excludes_inactive_locks_and_team() {
		let unit = 10u128.pow(DECIMALS);
		let balances = Balances {
			total_issuance: 1_000 * unit + 999,
			inactive_issuance: 100 * unit,
			team_and_foundation: 200 * unit,
			locks: 300 * unit + 1,
		};
		let supply = balances.supply(1_793_703_978_000);
		assert_eq!(supply.total, 1_000);
		// 400 tokens and 998 planck, truncated.
		assert_eq!(supply.circulating, 400);
		// Below zero, BigInt division truncates toward zero, as Rust's does.
		let over = Balances { locks: 2_000 * unit + 1, ..balances }.supply(0);
		assert_eq!(over.circulating, -1_299);
	}

	#[test]
	fn team_and_foundation_locks_are_skipped() {
		let locks = vec![([1u8; 8], 5u128, 0u8), ([2u8; 8], 7u128, 2u8)].encode();
		assert_eq!(account_locks("0xabc", &locks), 12);
		assert_eq!(account_locks(&format!("0x{LOCKS_PREFIX}{TEAM}"), &locks), 0);
		assert_eq!(account_locks(&format!("0x{LOCKS_PREFIX}{FOUNDATION}"), &locks), 0);
		assert_eq!(account_locks("0xabc", &[4, 1]), 0);
	}

	/** The live checks `bridgeTokenSupply.service.test.ts` makes, against
	 * `HYPERBRIDGE_NEXUS_RPC`. */
	#[tokio::test]
	async fn live_supply_is_sane() {
		let Ok(url) = std::env::var("HYPERBRIDGE_NEXUS_RPC") else { return };
		let rpc = SubstrateRpc::new(&url, reqwest::Client::new());
		let hash: String = rpc.call("chain_getFinalizedHead", json!([])).await.unwrap();
		let header: serde_json::Value = rpc.call("chain_getHeader", json!([hash])).await.unwrap();
		let number = header["number"].as_str().unwrap().trim_start_matches("0x");
		let block = u64::from_str_radix(number, 16).unwrap();

		let total = storage(&rpc, TOTAL_ISSUANCE, &hash).await.unwrap().unwrap();
		assert!(read_u128(&total, 0).unwrap() > 0);
		assert!(team_and_foundation(&rpc, &hash).await.unwrap() > 0);
		let supply = read_supply(&rpc, block).await.unwrap().unwrap();
		assert!(supply.total > 0);
		assert!(supply.circulating >= 0);
		assert!(supply.circulating <= supply.total);
		assert!(supply.timestamp_ms > 1_700_000_000_000);
	}
}
