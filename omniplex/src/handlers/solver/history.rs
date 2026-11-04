//! `solver_inventories` is the one historical table: each row is a version of an id over a
//! `_block_range` of block timestamps in milliseconds, as SubQuery wrote it under
//! `--historical=timestamp`. omnihedron answers `blockHeight: H` with the version whose range
//! contains H, and a query without one with the open version, whose range ends at infinity.

use alloy::primitives::I256;
use anyhow::{bail, Context};
use chrono::NaiveDateTime;
use tokio_postgres::{Row, Transaction};

use crate::store::{numeric, Numeric, SolverDiscoveryTrigger};

/** One `solver_inventories` row: a solver's balance of one token on one chain. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
	pub id: String,
	pub chain: String,
	pub solver: String,
	pub token_address: String,
	pub balance: I256,
	pub wallet: I256,
	pub vaults: I256,
	pub vault_shares: I256,
	pub block_number: u64,
	pub last_log_index: i32,
	pub observed_at: NaiveDateTime,
	pub refreshed_at: NaiveDateTime,
	pub genesis_block: u64,
	pub last_read_block: u64,
	pub last_reconciled_at: Option<NaiveDateTime>,
	pub last_reconciled_drift: Option<I256>,
	pub discovered_by: SolverDiscoveryTrigger,
}

const COLUMNS: &str = "id, chain, solver, token_address, balance, wallet, vaults, vault_shares, \
	block_number, last_log_index, observed_at, refreshed_at, genesis_block, last_read_block, \
	last_reconciled_at, last_reconciled_drift, discovered_by";

pub fn int(row: &Row, column: &str) -> anyhow::Result<I256> {
	let value: Numeric = row.try_get(column)?;
	I256::from_dec_str(value.as_str()).with_context(|| format!("{column} = {value}"))
}

pub fn opt_int(row: &Row, column: &str) -> anyhow::Result<Option<I256>> {
	let value: Option<Numeric> = row.try_get(column)?;
	value
		.map(|v| I256::from_dec_str(v.as_str()).with_context(|| format!("{column} = {v}")))
		.transpose()
}

pub fn block(row: &Row, column: &str) -> anyhow::Result<u64> {
	let value: Numeric = row.try_get(column)?;
	value.as_str().parse().with_context(|| format!("{column} = {value}"))
}

impl Inventory {
	fn from_row(row: &Row) -> anyhow::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			chain: row.try_get("chain")?,
			solver: row.try_get("solver")?,
			token_address: row.try_get("token_address")?,
			balance: int(row, "balance")?,
			wallet: int(row, "wallet")?,
			vaults: int(row, "vaults")?,
			vault_shares: int(row, "vault_shares")?,
			block_number: block(row, "block_number")?,
			last_log_index: row.try_get("last_log_index")?,
			observed_at: row.try_get("observed_at")?,
			refreshed_at: row.try_get("refreshed_at")?,
			genesis_block: block(row, "genesis_block")?,
			last_read_block: block(row, "last_read_block")?,
			last_reconciled_at: row.try_get("last_reconciled_at")?,
			last_reconciled_drift: opt_int(row, "last_reconciled_drift")?,
			discovered_by: row.try_get("discovered_by")?,
		})
	}
}

/** The current version of `id`: the one whose range is open. */
pub async fn load_inventory(db: &Transaction<'_>, id: &str) -> anyhow::Result<Option<Inventory>> {
	let row = db
		.query_opt(
			&format!(
				"SELECT {COLUMNS} FROM app.solver_inventories
				 WHERE id = $1 AND upper_inf(_block_range)
				 ORDER BY lower(_block_range) DESC LIMIT 1"
			),
			&[&id],
		)
		.await?;
	row.as_ref().map(Inventory::from_row).transpose()
}

/** The version timestamp of a write at `at`: SubQuery's historical unit, milliseconds. */
pub fn version_of(at: NaiveDateTime) -> i64 {
	at.and_utc().timestamp_millis()
}

/**
 * Saves `row` as of block time `at`, the way SubQuery's historical store does: a new id opens
 * `[at, ∞)`; a save at the open version's own timestamp replaces it in place; a later save
 * closes it at `at` and opens a new one. Every save makes a version, even when nothing changed.
 * A save earlier than the open version is an ordering bug and an error.
 */
pub async fn save_inventory(
	db: &Transaction<'_>,
	row: &Inventory,
	at: NaiveDateTime,
) -> anyhow::Result<()> {
	let version = version_of(at);
	let open = db
		.query_opt(
			"SELECT _id::text, lower(_block_range) FROM app.solver_inventories
			 WHERE id = $1 AND upper_inf(_block_range)
			 ORDER BY lower(_block_range) DESC LIMIT 1 FOR UPDATE",
			&[&row.id],
		)
		.await?
		.map(|r| (r.get::<_, String>(0), r.get::<_, i64>(1)));
	let values: [&(dyn tokio_postgres::types::ToSql + Sync); 17] = [
		&row.id,
		&row.chain,
		&row.solver,
		&row.token_address,
		&numeric(row.balance),
		&numeric(row.wallet),
		&numeric(row.vaults),
		&numeric(row.vault_shares),
		&numeric(row.block_number),
		&row.last_log_index,
		&row.observed_at,
		&row.refreshed_at,
		&numeric(row.genesis_block),
		&numeric(row.last_read_block),
		&row.last_reconciled_at,
		&row.last_reconciled_drift.map(numeric),
		&row.discovered_by,
	];
	match open {
		Some((uid, lower)) if lower == version => {
			let mut params = values.to_vec();
			params.push(&uid);
			db.execute(
				"UPDATE app.solver_inventories SET id = $1, chain = $2, solver = $3,
				   token_address = $4, balance = $5, wallet = $6, vaults = $7, vault_shares = $8,
				   block_number = $9, last_log_index = $10, observed_at = $11, refreshed_at = $12,
				   genesis_block = $13, last_read_block = $14, last_reconciled_at = $15,
				   last_reconciled_drift = $16, discovered_by = $17
				 WHERE _id = $18::text::uuid",
				&params,
			)
			.await?;
			return Ok(());
		},
		Some((_, lower)) if lower > version => bail!(
			"solver_inventories {}: a write at {version} precedes the version at {lower}",
			row.id
		),
		Some(_) => {
			db.execute(
				"UPDATE app.solver_inventories SET _block_range = int8range(lower(_block_range), $2)
				 WHERE id = $1 AND upper_inf(_block_range)",
				&[&row.id, &version],
			)
			.await?;
		},
		None => {},
	}
	let mut params = values.to_vec();
	params.push(&version);
	db.execute(
		&format!(
			"INSERT INTO app.solver_inventories ({COLUMNS}, _id, _block_range)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
			   gen_random_uuid(), int8range($18, NULL))"
		),
		&params,
	)
	.await?;
	Ok(())
}
