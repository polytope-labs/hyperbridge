//! Solver inventory, event-sourced from each chain's own Transfer events
//! (`solverInventory.service.ts`).
//!
//! A solver is tracked once it fills an order on the chain or the orderbook's watchlist asks for
//! it. Tracking starts with a storage read pinned to a block (every supported token's balance,
//! every vault's shares, the account's code), positioned after every log of that block. From then
//! on the totals move only with Transfer logs that come after a row's position.

use std::collections::HashSet;

use alloy::primitives::{Address, Bytes, I256};
use chrono::NaiveDateTime;
use tokio_postgres::{Row, Transaction};

use super::{
	history::{block, int, load_inventory, save_inventory, Inventory},
	parse_delegation,
	reads::{self, read_contracts, ContractRead, Reads, Settled},
	unsigned, DelegationReading, SolverChain, ZERO_ADDRESS,
};
use crate::{
	handlers::abi::{Erc20, Erc4626},
	services::solvers::{queue_solver, Discovery},
	store::{numeric, timestamp_to_date, SolverDiscoveryTrigger, TrackedSolverStatus},
};

/** The log position of a storage read: after every log of its block. */
pub const AFTER_EVERY_LOG: i32 = i32::MAX;
/** Genesis reads per block. */
pub const MAX_GENESIS_PER_BLOCK: i64 = 20;
/** Block time between head advances. */
pub const HEAD_INTERVAL_SECS: i64 = 30;
/** How often a solver's vault shares are revalued. */
pub const REVALUE_INTERVAL_SECS: i64 = 3_600;
/** How often a solver's balances are re-read and its delegation re-checked. */
pub const RECONCILE_INTERVAL_SECS: i64 = 86_400;
/** Tracked solvers examined per head advance. */
pub const REFRESH_PAGE_SIZE: i64 = 25;
/** Past this many unseen watchlist versions, one scan of the chain's requests. */
const FULL_SCAN_VERSION_GAP: i32 = 20;

fn tracked_solver_id(chain: &str, solver: &str) -> String {
	format!("{chain}-{solver}")
}
fn inventory_id(chain: &str, token: &str, solver: &str) -> String {
	format!("{chain}-{token}-{solver}")
}
fn vault_shares_id(chain: &str, vault: &str, solver: &str) -> String {
	format!("{chain}-{vault}-{solver}")
}

/** `isAfter`: whether a log at (block, log index) comes after the row's position. */
pub fn is_after(block_number: u64, log_index: u64, row_block: u64, row_log_index: i32) -> bool {
	block_number > row_block ||
		(block_number == row_block && log_index as i64 > row_log_index as i64)
}

fn later(a: NaiveDateTime, b: NaiveDateTime) -> NaiveDateTime {
	if b > a {
		b
	} else {
		a
	}
}

/** `elapsedSecs(from, to) >= secs`, where a missing `from` has elapsed forever. */
fn elapsed_at_least(from: Option<NaiveDateTime>, to: NaiveDateTime, secs: i64) -> bool {
	from.is_none_or(|from| (to - from).num_milliseconds() >= secs * 1000)
}

fn parse_address(address: &str) -> anyhow::Result<Address> {
	Ok(address.parse()?)
}

fn is_db_error(err: &anyhow::Error) -> bool {
	err.chain().any(|cause| cause.is::<tokio_postgres::Error>())
}

// ─── Rows ───────────────────────────────────────────────────────────────────────────────────────

/** One `solver_vault_shares` row. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultShares {
	pub id: String,
	pub chain: String,
	pub solver: String,
	pub vault: String,
	pub token_address: String,
	pub shares: I256,
	pub assets: I256,
	pub block_number: u64,
	pub last_log_index: i32,
	pub observed_at: NaiveDateTime,
}

async fn load_vault_shares(db: &Transaction<'_>, id: &str) -> anyhow::Result<Option<VaultShares>> {
	let Some(row) = db
		.query_opt(
			"SELECT id, chain, solver, vault, token_address, shares, assets, block_number,
			   last_log_index, observed_at
			 FROM app.solver_vault_shares WHERE id = $1",
			&[&id],
		)
		.await?
	else {
		return Ok(None);
	};
	Ok(Some(VaultShares {
		id: row.try_get("id")?,
		chain: row.try_get("chain")?,
		solver: row.try_get("solver")?,
		vault: row.try_get("vault")?,
		token_address: row.try_get("token_address")?,
		shares: int(&row, "shares")?,
		assets: int(&row, "assets")?,
		block_number: block(&row, "block_number")?,
		last_log_index: row.try_get("last_log_index")?,
		observed_at: row.try_get("observed_at")?,
	}))
}

async fn save_vault_shares(db: &Transaction<'_>, row: &VaultShares) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO app.solver_vault_shares
		   (id, chain, solver, vault, token_address, shares, assets, block_number, last_log_index, observed_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		 ON CONFLICT (id) DO UPDATE SET chain = EXCLUDED.chain, solver = EXCLUDED.solver,
		   vault = EXCLUDED.vault, token_address = EXCLUDED.token_address, shares = EXCLUDED.shares,
		   assets = EXCLUDED.assets, block_number = EXCLUDED.block_number,
		   last_log_index = EXCLUDED.last_log_index, observed_at = EXCLUDED.observed_at",
		&[
			&row.id,
			&row.chain,
			&row.solver,
			&row.vault,
			&row.token_address,
			&numeric(row.shares),
			&numeric(row.assets),
			&numeric(row.block_number),
			&row.last_log_index,
			&row.observed_at,
		],
	)
	.await?;
	Ok(())
}

/** The `tracked_solvers` fields this module reads. */
#[derive(Debug, Clone)]
pub struct TrackedSolver {
	pub id: String,
	pub solver: String,
	pub discovered_by: SolverDiscoveryTrigger,
	pub revalued_at: Option<NaiveDateTime>,
	pub reconciled_at: Option<NaiveDateTime>,
}

impl TrackedSolver {
	fn from_row(row: &Row) -> anyhow::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			solver: row.try_get("solver")?,
			discovered_by: row.try_get("discovered_by")?,
			revalued_at: row.try_get("revalued_at")?,
			reconciled_at: row.try_get("reconciled_at")?,
		})
	}
}

/** The chain's solvers with `status`, in id order. */
async fn tracked_page(
	db: &Transaction<'_>,
	chain: &str,
	status: TrackedSolverStatus,
	limit: i64,
	offset: i64,
) -> anyhow::Result<Vec<TrackedSolver>> {
	db.query(
		"SELECT id, solver, discovered_by, revalued_at, reconciled_at FROM app.tracked_solvers
		 WHERE chain = $1 AND status = $2 ORDER BY id ASC LIMIT $3 OFFSET $4",
		&[&chain, &status, &limit, &offset],
	)
	.await?
	.iter()
	.map(TrackedSolver::from_row)
	.collect()
}

// ─── Tracked-solver filter ──────────────────────────────────────────────────────────────────────
//
// Supported-token Transfers arrive for every holder of the token, so nearly all of them must be
// dropped without touching the database. The set is only a filter: an address it admits still
// needs its row. SubQuery kept it in its cross-worker cache; one process needs only memory.

/** `refreshTrackedSolvers`: unions the TRACKED rows into the set, never dropping a member. */
pub async fn refresh_tracked_solvers(
	db: &Transaction<'_>,
	chain: &SolverChain,
) -> anyhow::Result<HashSet<String>> {
	let rows = db
		.query(
			"SELECT solver FROM app.tracked_solvers WHERE chain = $1 AND status = $2 ORDER BY id ASC",
			&[&chain.id, &TrackedSolverStatus::Tracked],
		)
		.await?;
	let mut tracked = chain.tracked.lock().unwrap();
	let set = tracked.get_or_insert_with(HashSet::new);
	set.extend(rows.iter().map(|row| row.get::<_, String>(0)));
	Ok(set.clone())
}

async fn tracked_parties(
	db: &Transaction<'_>,
	chain: &SolverChain,
	candidates: [&str; 2],
	exclude_zero: bool,
) -> anyhow::Result<Vec<String>> {
	let filter = |set: &HashSet<String>| -> Vec<String> {
		candidates
			.iter()
			.filter(|a| !(exclude_zero && **a == ZERO_ADDRESS) && set.contains(**a))
			.map(|a| a.to_string())
			.collect()
	};
	if let Some(set) = chain.tracked.lock().unwrap().as_ref() {
		return Ok(filter(set));
	}
	Ok(filter(&refresh_tracked_solvers(db, chain).await?))
}

/** `cacheTrackedSolver`: publishes a newly seeded solver. */
async fn cache_tracked_solver(
	db: &Transaction<'_>,
	chain: &SolverChain,
	solver: &str,
) -> anyhow::Result<()> {
	if let Some(set) = chain.tracked.lock().unwrap().as_mut() {
		set.insert(solver.to_string());
		return Ok(());
	}
	refresh_tracked_solvers(db, chain).await?;
	Ok(())
}

// ─── Discovery ──────────────────────────────────────────────────────────────────────────────────

/**
 * `consumeWatchlist`: turns the requests queued for this chain into PENDING solvers. SubQuery
 * read the watchlist through its historical filter, which hid a version written after the
 * block's time, so a chain still in its history doesn't consume one yet.
 */
pub async fn consume_watchlist(
	db: &Transaction<'_>,
	chain: &SolverChain,
	block_number: u64,
	timestamp: u64,
) -> anyhow::Result<()> {
	let Some(watchlist) = db
		.query_opt(
			"SELECT version FROM app.solver_watchlists WHERE chain = $1 AND updated_at <= $2 LIMIT 1",
			&[&chain.id, &timestamp_to_date(timestamp)],
		)
		.await?
	else {
		return Ok(());
	};
	let version: i32 = watchlist.get(0);
	let cursor: Option<i32> = db
		.query_opt("SELECT version FROM app.solver_watchlist_cursors WHERE id = $1", &[&chain.id])
		.await?
		.map(|row| row.get(0));
	let consumed = cursor.unwrap_or(0);
	if version <= consumed {
		return Ok(());
	}

	let mut requests: Vec<(String, i32)> = vec![];
	if cursor.is_none() || version - consumed > FULL_SCAN_VERSION_GAP {
		for row in db
			.query(
				"SELECT solver, version FROM app.solver_discovery_requests WHERE chain = $1 ORDER BY id ASC",
				&[&chain.id],
			)
			.await?
		{
			requests.push((row.get(0), row.get(1)));
		}
	} else {
		for v in consumed + 1..=version {
			for row in db
				.query(
					"SELECT solver, version FROM app.solver_discovery_requests
					 WHERE chain = $1 AND version = $2 ORDER BY id ASC",
					&[&chain.id, &v],
				)
				.await?
			{
				requests.push((row.get(0), row.get(1)));
			}
		}
	}

	for (solver, request_version) in requests {
		// Written after the watchlist row was read.
		if request_version > version {
			continue;
		}
		queue_solver(
			db,
			&chain.id,
			&solver,
			Discovery {
				by: SolverDiscoveryTrigger::Watchlist,
				fill_transaction: None,
				block_number,
				block_timestamp: timestamp,
			},
		)
		.await?;
	}
	db.execute(
		"INSERT INTO app.solver_watchlist_cursors (id, chain, version) VALUES ($1, $1, $2)
		 ON CONFLICT (id) DO UPDATE SET chain = EXCLUDED.chain, version = EXCLUDED.version",
		&[&chain.id, &version],
	)
	.await?;
	Ok(())
}

// ─── Storage reads ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct VaultReading {
	pub vault: String,
	pub shares: I256,
	pub assets: I256,
	/** On a reconciliation, the read shares minus the tracked ones, valued at the block. */
	pub drift: I256,
}

#[derive(Debug, Clone)]
pub struct TokenReading {
	pub token: String,
	pub wallet: I256,
	pub vaults: Vec<VaultReading>,
}

#[derive(Debug, Clone)]
pub struct SolverReading {
	pub delegation: DelegationReading,
	pub tokens: Vec<TokenReading>,
}

/**
 * `valueShares`: signed share amounts valued in assets at `block`, in one batch. Zero costs no
 * read, and a negative amount is valued by its magnitude.
 */
pub async fn value_shares(
	chain: &SolverChain,
	reads: &dyn Reads,
	amounts: &[(String, I256)],
	block: u64,
) -> anyhow::Result<Vec<Settled<I256>>> {
	let mut batch = vec![];
	for (vault, shares) in amounts.iter().filter(|(_, shares)| !shares.is_zero()) {
		batch.push(ContractRead::new(
			parse_address(vault)?,
			Erc4626::convertToAssetsCall { shares: unsigned(*shares) },
		));
	}
	let answers = read_contracts(chain, reads, batch.clone(), block).await?;
	let mut next = 0;
	Ok(amounts
		.iter()
		.map(|(_, shares)| {
			if shares.is_zero() {
				return Ok(I256::ZERO);
			}
			let read = &batch[next];
			let answer = &answers[next];
			next += 1;
			let assets = read.decode::<Erc4626::convertToAssetsCall>(answer)?;
			let assets = super::signed(assets).map_err(|e| e.to_string())?;
			Ok(if shares.is_negative() { -assets } else { assets })
		})
		.collect())
}

struct VaultDraft {
	vault: String,
	shares: I256,
	slot: usize,
	drift_slot: Option<usize>,
}

struct TokenDraft {
	token: String,
	wallet: I256,
	vaults: Vec<VaultDraft>,
}

struct SolverDraft {
	code: Bytes,
	tokens: Vec<TokenDraft>,
}

/**
 * `readSolvers`: every read tracking needs for each of `solvers`, pinned to `block`. Every
 * token's and vault's balanceOf in one batch alongside each solver's getCode, then every vault
 * balance valued in a second. A reconciliation also values each vault's drift from the tracked
 * position. A solver with any failed read comes back as an error; a failed batch is an error.
 */
pub async fn read_solvers(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	solvers: &[String],
	reconciliation: bool,
	block: u64,
) -> anyhow::Result<Vec<Settled<SolverReading>>> {
	let holdings: Vec<&str> = chain
		.tokens
		.iter()
		.flat_map(|t| std::iter::once(t.token.as_str()).chain(t.vaults.iter().map(String::as_str)))
		.collect();
	let mut batch = vec![];
	for solver in solvers {
		let account = parse_address(solver)?;
		for holding in &holdings {
			batch
				.push(ContractRead::new(parse_address(holding)?, Erc20::balanceOfCall { account }));
		}
	}
	let addresses = solvers.iter().map(|s| parse_address(s)).collect::<anyhow::Result<Vec<_>>>()?;
	let (balances, codes) = futures::join!(
		read_contracts(chain, reads, batch.clone(), block),
		reads::map_concurrently(addresses, |address| reads::code(reads, address, block)),
	);
	let balances = balances?;

	let mut amounts: Vec<(String, I256)> = vec![];
	let mut drafts: Vec<Settled<SolverDraft>> = vec![];
	for (index, code) in codes.into_iter().enumerate() {
		let range = index * holdings.len()..(index + 1) * holdings.len();
		let draft = (|| -> Settled<(Bytes, Vec<I256>)> {
			let code = code?;
			let own = batch[range.clone()]
				.iter()
				.zip(&balances[range.clone()])
				.map(|(read, answer)| {
					let value = read.decode::<Erc20::balanceOfCall>(answer)?;
					super::signed(value).map_err(|e| e.to_string())
				})
				.collect::<Settled<Vec<_>>>()?;
			Ok((code, own))
		})();
		drafts.push(draft.map(|(code, own)| {
			let mut next = own.into_iter();
			SolverDraft {
				code,
				tokens: chain
					.tokens
					.iter()
					.map(|t| TokenDraft {
						token: t.token.clone(),
						wallet: next.next().unwrap_or_default(),
						vaults: t
							.vaults
							.iter()
							.map(|vault| {
								let shares = next.next().unwrap_or_default();
								amounts.push((vault.clone(), shares));
								VaultDraft {
									vault: vault.clone(),
									shares,
									slot: amounts.len() - 1,
									drift_slot: None,
								}
							})
							.collect(),
					})
					.collect(),
			}
		}));
	}
	if reconciliation {
		for (index, draft) in drafts.iter_mut().enumerate() {
			let Ok(draft) = draft else { continue };
			for vault in draft.tokens.iter_mut().flat_map(|t| t.vaults.iter_mut()) {
				let id = vault_shares_id(&chain.id, &vault.vault, &solvers[index]);
				if let Some(position) = load_vault_shares(db, &id).await? {
					if position.shares != vault.shares {
						amounts.push((vault.vault.clone(), vault.shares - position.shares));
						vault.drift_slot = Some(amounts.len() - 1);
					}
				}
			}
		}
	}

	let values = value_shares(chain, reads, &amounts, block).await?;
	Ok(drafts
		.into_iter()
		.map(|draft| {
			let draft = draft?;
			let code = alloy::hex::encode_prefixed(&draft.code);
			Ok(SolverReading {
				delegation: parse_delegation(chain, Some(&code)),
				tokens: draft
					.tokens
					.into_iter()
					.map(|t| {
						Ok(TokenReading {
							token: t.token,
							wallet: t.wallet,
							vaults: t
								.vaults
								.into_iter()
								.map(|v| {
									Ok(VaultReading {
										assets: values[v.slot].clone()?,
										drift: match v.drift_slot {
											Some(slot) => values[slot].clone()?,
											None => I256::ZERO,
										},
										vault: v.vault,
										shares: v.shares,
									})
								})
								.collect::<Settled<Vec<_>>>()?,
						})
					})
					.collect::<Settled<Vec<_>>>()?,
			})
		})
		.collect())
}

async fn record_delegation(
	db: &Transaction<'_>,
	chain: &SolverChain,
	solver: &str,
	reading: &DelegationReading,
	block_number: u64,
	at: NaiveDateTime,
) -> anyhow::Result<()> {
	let id = tracked_solver_id(&chain.id, solver);
	let existing = db
		.query_opt(
			"SELECT delegated, delegate, observed_at FROM app.solver_delegations WHERE id = $1",
			&[&id],
		)
		.await?
		.map(|row| {
			(row.get::<_, bool>(0), row.get::<_, Option<String>>(1), row.get::<_, NaiveDateTime>(2))
		});
	let observed_at = match existing {
		Some((delegated, delegate, observed_at))
			if delegated == reading.delegated && delegate == reading.delegate =>
			observed_at,
		_ => at,
	};
	db.execute(
		"INSERT INTO app.solver_delegations
		   (id, chain, solver, delegated, delegate, block_number, observed_at, refreshed_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
		 ON CONFLICT (id) DO UPDATE SET chain = EXCLUDED.chain, solver = EXCLUDED.solver,
		   delegated = EXCLUDED.delegated, delegate = EXCLUDED.delegate,
		   block_number = EXCLUDED.block_number, observed_at = EXCLUDED.observed_at,
		   refreshed_at = EXCLUDED.refreshed_at",
		&[
			&id,
			&chain.id,
			&solver,
			&reading.delegated,
			&reading.delegate,
			&numeric(block_number),
			&observed_at,
			&at,
		],
	)
	.await?;
	Ok(())
}

/**
 * `applyReading`: writes a storage read over the solver's rows, positioned after `block`. A
 * reconciliation also records how far the running totals had drifted from the chain.
 */
async fn apply_reading(
	db: &Transaction<'_>,
	chain: &SolverChain,
	tracked: &TrackedSolver,
	reading: &SolverReading,
	block_number: u64,
	at: NaiveDateTime,
	reconciliation: bool,
) -> anyhow::Result<()> {
	let solver = &tracked.solver;
	record_delegation(db, chain, solver, &reading.delegation, block_number, at).await?;

	for token in &reading.tokens {
		let mut vaults = I256::ZERO;
		let mut vault_shares = I256::ZERO;
		let mut drift = I256::ZERO;
		for vault in &token.vaults {
			let id = vault_shares_id(&chain.id, &vault.vault, solver);
			let position = load_vault_shares(db, &id).await?;
			let observed_at = match &position {
				None => at,
				Some(p) if p.shares != vault.shares || p.assets != vault.assets =>
					later(p.observed_at, at),
				Some(p) => p.observed_at,
			};
			save_vault_shares(
				db,
				&VaultShares {
					id,
					chain: chain.id.clone(),
					solver: solver.clone(),
					vault: vault.vault.clone(),
					token_address: token.token.clone(),
					shares: vault.shares,
					assets: vault.assets,
					block_number,
					last_log_index: AFTER_EVERY_LOG,
					observed_at,
				},
			)
			.await?;
			vaults += vault.assets;
			vault_shares += vault.shares;
			drift += vault.drift;
		}

		let id = inventory_id(&chain.id, &token.token, solver);
		let existing = load_inventory(db, &id).await?;
		let balance = token.wallet + vaults;
		if let Some(existing) = &existing {
			drift += token.wallet - existing.wallet;
		}
		if reconciliation && !drift.is_zero() {
			tracing::warn!(
				"[solver-inventory] Reconciliation drift {drift} for {solver} in {} on {} at block {block_number}",
				token.token,
				chain.id
			);
		}
		let observed_at = match &existing {
			None => at,
			Some(e) if e.balance != balance => later(e.observed_at, at),
			Some(e) => e.observed_at,
		};
		let row = Inventory {
			id,
			chain: chain.id.clone(),
			solver: solver.clone(),
			token_address: token.token.clone(),
			balance,
			wallet: token.wallet,
			vaults,
			vault_shares,
			block_number,
			last_log_index: AFTER_EVERY_LOG,
			observed_at,
			refreshed_at: at,
			// A token added to the config after the solver was tracked gets its genesis here.
			genesis_block: existing.as_ref().map_or(block_number, |e| e.genesis_block),
			last_read_block: block_number,
			last_reconciled_at: if reconciliation {
				Some(at)
			} else {
				existing.as_ref().and_then(|e| e.last_reconciled_at)
			},
			last_reconciled_drift: if reconciliation {
				Some(drift)
			} else {
				existing.as_ref().and_then(|e| e.last_reconciled_drift)
			},
			discovered_by: tracked.discovered_by,
		};
		save_inventory(db, &row, at).await?;
	}
	Ok(())
}

/**
 * `seedPendingSolvers`: the genesis read for up to 20 PENDING solvers in id order. A failed
 * read leaves its solver PENDING with nothing written, and a later block reads it again.
 */
pub async fn seed_pending_solvers(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	block_number: u64,
	at: NaiveDateTime,
) -> anyhow::Result<()> {
	let pending =
		tracked_page(db, &chain.id, TrackedSolverStatus::Pending, MAX_GENESIS_PER_BLOCK, 0).await?;
	if pending.is_empty() {
		return Ok(());
	}
	let solvers: Vec<String> = pending.iter().map(|t| t.solver.clone()).collect();
	let readings = match read_solvers(db, chain, reads, &solvers, false, block_number).await {
		Ok(readings) => readings,
		Err(err) if is_db_error(&err) => return Err(err),
		Err(err) => {
			tracing::warn!(
				"[solver-inventory] Genesis reads failed on {}, retrying next block: {err:#}",
				chain.id
			);
			return Ok(());
		},
	};
	for (tracked, reading) in pending.iter().zip(readings) {
		let reading = match reading {
			Ok(reading) => reading,
			Err(err) => {
				tracing::warn!(
					"[solver-inventory] Genesis read failed for {} on {}, retrying next block: {err}",
					tracked.solver,
					chain.id
				);
				continue;
			},
		};
		apply_reading(db, chain, tracked, &reading, block_number, at, false).await?;
		db.execute(
			"UPDATE app.tracked_solvers SET status = $2, genesis_block = $3, revalued_at = $4,
			   reconciled_at = $4 WHERE id = $1",
			&[&tracked.id, &TrackedSolverStatus::Tracked, &numeric(block_number), &at],
		)
		.await?;
		cache_tracked_solver(db, chain, &tracked.solver).await?;
	}
	Ok(())
}

// ─── Events ─────────────────────────────────────────────────────────────────────────────────────

/** A Transfer log of a supported token or a vault. Addresses are lowercase. */
#[derive(Debug, Clone)]
pub struct Transfer {
	/** The token or vault that emitted it. */
	pub contract: String,
	pub from: String,
	pub to: String,
	pub value: I256,
	pub block_number: u64,
	pub log_index: u64,
	/** Block time in seconds. */
	pub timestamp: u64,
}

/** `applyTokenTransfer`: moves the wallet of whichever side is a tracked solver. */
pub async fn apply_token_transfer(
	db: &Transaction<'_>,
	chain: &SolverChain,
	transfer: &Transfer,
) -> anyhow::Result<()> {
	if transfer.from == transfer.to || transfer.value.is_zero() {
		return Ok(());
	}
	let parties = tracked_parties(db, chain, [&transfer.from, &transfer.to], false).await?;
	if parties.is_empty() {
		return Ok(());
	}
	let at = timestamp_to_date(transfer.timestamp);
	for solver in parties {
		let id = inventory_id(&chain.id, &transfer.contract, &solver);
		let Some(mut row) = load_inventory(db, &id).await? else { continue };
		if !is_after(
			transfer.block_number,
			transfer.log_index,
			row.block_number,
			row.last_log_index,
		) {
			continue;
		}
		let mut wallet = if solver == transfer.from {
			row.wallet - transfer.value
		} else {
			row.wallet + transfer.value
		};
		if wallet.is_negative() {
			tracing::error!(
				"[solver-inventory] {solver}'s {} wallet on {} went negative at block {}; clamping to zero until reconciliation",
				transfer.contract,
				chain.id,
				transfer.block_number
			);
			wallet = I256::ZERO;
		}
		row.wallet = wallet;
		row.balance = wallet + row.vaults;
		row.block_number = transfer.block_number;
		row.last_log_index = transfer.log_index as i32;
		row.observed_at = later(row.observed_at, at);
		save_inventory(db, &row, at).await?;
	}
	Ok(())
}

/**
 * `applyVaultShareTransfer`: moves the vault shares of whichever side is a tracked solver.
 * Mints and burns count: a deposit mints to the owner, a withdrawal burns from it.
 */
pub async fn apply_vault_share_transfer(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	transfer: &Transfer,
) -> anyhow::Result<()> {
	if transfer.from == transfer.to || transfer.value.is_zero() {
		return Ok(());
	}
	let parties = tracked_parties(db, chain, [&transfer.from, &transfer.to], true).await?;
	if parties.is_empty() {
		return Ok(());
	}
	let vault = &transfer.contract;
	let Some(token) = chain.token_for_vault(vault).map(str::to_string) else { return Ok(()) };
	let at = timestamp_to_date(transfer.timestamp);
	for solver in parties {
		let id = vault_shares_id(&chain.id, vault, &solver);
		let Some(mut position) = load_vault_shares(db, &id).await? else { continue };
		if !is_after(
			transfer.block_number,
			transfer.log_index,
			position.block_number,
			position.last_log_index,
		) {
			continue;
		}
		let mut shares = if solver == transfer.from {
			position.shares - transfer.value
		} else {
			position.shares + transfer.value
		};
		if shares.is_negative() {
			tracing::error!(
				"[solver-inventory] {solver}'s shares in {vault} on {} went negative at block {}; clamping to zero until reconciliation",
				chain.id,
				transfer.block_number
			);
			shares = I256::ZERO;
		}
		let valued =
			value_shares(chain, reads, &[(vault.clone(), shares)], transfer.block_number).await?;
		position.shares = shares;
		position.assets = valued[0].clone().map_err(anyhow::Error::msg)?;
		position.block_number = transfer.block_number;
		position.last_log_index = transfer.log_index as i32;
		position.observed_at = later(position.observed_at, at);
		save_vault_shares(db, &position).await?;
		refold_vaults(db, chain, &token, &solver, at).await?;
	}
	Ok(())
}

/** `refoldVaults`: recomputes the inventory's vault totals from the solver's positions. */
async fn refold_vaults(
	db: &Transaction<'_>,
	chain: &SolverChain,
	token: &str,
	solver: &str,
	at: NaiveDateTime,
) -> anyhow::Result<()> {
	let Some(mut inventory) = load_inventory(db, &inventory_id(&chain.id, token, solver)).await?
	else {
		return Ok(());
	};
	let mut vaults = I256::ZERO;
	let mut vault_shares = I256::ZERO;
	let token_vaults = chain.tokens.iter().find(|t| t.token == token).map(|t| t.vaults.clone());
	for vault in token_vaults.unwrap_or_default() {
		if let Some(position) =
			load_vault_shares(db, &vault_shares_id(&chain.id, &vault, solver)).await?
		{
			vaults += position.assets;
			vault_shares += position.shares;
		}
	}
	let balance = inventory.wallet + vaults;
	if balance != inventory.balance {
		inventory.observed_at = later(inventory.observed_at, at);
	}
	inventory.vaults = vaults;
	inventory.vault_shares = vault_shares;
	inventory.balance = balance;
	save_inventory(db, &inventory, at).await
}

// ─── Head and refresh ───────────────────────────────────────────────────────────────────────────

struct Revaluation {
	token: String,
	positions: Vec<(VaultShares, I256)>,
}

/** `readRevaluations`: every solver's vault positions valued at `block`, in one batch. */
async fn read_revaluations(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	solvers: &[TrackedSolver],
	block: u64,
) -> anyhow::Result<Vec<Settled<Vec<Revaluation>>>> {
	let mut amounts = vec![];
	let mut plans = vec![];
	for tracked in solvers {
		let mut plan = vec![];
		for token in &chain.tokens {
			let mut positions = vec![];
			for vault in &token.vaults {
				let id = vault_shares_id(&chain.id, vault, &tracked.solver);
				if let Some(position) = load_vault_shares(db, &id).await? {
					amounts.push((vault.clone(), position.shares));
					positions.push((position, amounts.len() - 1));
				}
			}
			if !positions.is_empty() {
				plan.push((token.token.clone(), positions));
			}
		}
		plans.push(plan);
	}
	let values = value_shares(chain, reads, &amounts, block).await?;
	Ok(plans
		.into_iter()
		.map(|plan| {
			plan.into_iter()
				.map(|(token, positions)| {
					Ok(Revaluation {
						token,
						positions: positions
							.into_iter()
							.map(|(position, slot)| Ok((position, values[slot].clone()?)))
							.collect::<Settled<Vec<_>>>()?,
					})
				})
				.collect::<Settled<Vec<_>>>()
		})
		.collect())
}

/** `applyRevaluation`: writes revalued vault shares. Shares don't move; the wallet is left
 * alone. */
async fn apply_revaluation(
	db: &Transaction<'_>,
	chain: &SolverChain,
	tracked: &TrackedSolver,
	revalued: Vec<Revaluation>,
	at: NaiveDateTime,
) -> anyhow::Result<()> {
	for Revaluation { token, positions } in revalued {
		for (mut position, assets) in positions {
			if position.assets == assets {
				continue;
			}
			position.assets = assets;
			position.observed_at = later(position.observed_at, at);
			save_vault_shares(db, &position).await?;
		}
		refold_vaults(db, chain, &token, &tracked.solver, at).await?;
		let id = inventory_id(&chain.id, &token, &tracked.solver);
		let Some(mut inventory) = load_inventory(db, &id).await? else { continue };
		inventory.refreshed_at = at;
		save_inventory(db, &inventory, at).await?;
	}
	Ok(())
}

/**
 * `refreshPage`: one page of the chain's tracked solvers, each reconciled (daily) or revalued
 * (hourly) when due. A per-solver failure skips only that solver and the offset still advances;
 * a failed batch keeps the offset for the next advance. Returns the offset to resume from.
 */
async fn refresh_page(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	block_number: u64,
	at: NaiveDateTime,
	offset: i32,
) -> anyhow::Result<i32> {
	let page =
		tracked_page(db, &chain.id, TrackedSolverStatus::Tracked, REFRESH_PAGE_SIZE, offset as i64)
			.await?;
	let mut reconciling = vec![];
	let mut revaluing = vec![];
	for tracked in &page {
		if elapsed_at_least(tracked.reconciled_at, at, RECONCILE_INTERVAL_SECS) {
			reconciling.push(tracked.clone());
		} else if elapsed_at_least(tracked.revalued_at, at, REVALUE_INTERVAL_SECS) {
			revaluing.push(tracked.clone());
		}
	}

	let solvers: Vec<String> = reconciling.iter().map(|t| t.solver.clone()).collect();
	let reads_done = futures::try_join!(
		read_solvers(db, chain, reads, &solvers, true, block_number),
		read_revaluations(db, chain, reads, &revaluing, block_number),
	);
	let (readings, revaluations) = match reads_done {
		Ok(done) => done,
		Err(err) if is_db_error(&err) => return Err(err),
		Err(err) => {
			tracing::warn!(
				"[solver-inventory] Refresh reads failed on {}, retrying next pass: {err:#}",
				chain.id
			);
			return Ok(offset);
		},
	};
	let skip = |tracked: &TrackedSolver, err: &str| {
		tracing::warn!(
			"[solver-inventory] Refresh failed for {} on {}, retrying when its page comes round: {err}",
			tracked.solver,
			chain.id
		)
	};
	for (tracked, reading) in reconciling.iter().zip(readings) {
		let reading = match reading {
			Ok(reading) => reading,
			Err(err) => {
				skip(tracked, &err);
				continue;
			},
		};
		apply_reading(db, chain, tracked, &reading, block_number, at, true).await?;
		db.execute(
			"UPDATE app.tracked_solvers SET reconciled_at = $2, revalued_at = $2 WHERE id = $1",
			&[&tracked.id, &at],
		)
		.await?;
	}
	for (tracked, revaluation) in revaluing.iter().zip(revaluations) {
		let revaluation = match revaluation {
			Ok(revaluation) => revaluation,
			Err(err) => {
				skip(tracked, &err);
				continue;
			},
		};
		apply_revaluation(db, chain, tracked, revaluation, at).await?;
		db.execute(
			"UPDATE app.tracked_solvers SET revalued_at = $2 WHERE id = $1",
			&[&tracked.id, &at],
		)
		.await?;
	}
	Ok(if (page.len() as i64) < REFRESH_PAGE_SIZE { 0 } else { offset + page.len() as i32 })
}

/**
 * `advanceHead`: at most every HEAD_INTERVAL_SECS of block time, rebuilds the tracked filter,
 * runs one refresh page, and moves the chain's head to `block`.
 */
pub async fn advance_head(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	block_number: u64,
	at: NaiveDateTime,
) -> anyhow::Result<()> {
	let head = db
		.query_opt(
			"SELECT observed_at, refresh_offset FROM app.solver_inventory_heads WHERE id = $1",
			&[&chain.id],
		)
		.await?
		.map(|row| (row.get::<_, NaiveDateTime>(0), row.get::<_, i32>(1)));
	if let Some((observed_at, _)) = head {
		if !elapsed_at_least(Some(observed_at), at, HEAD_INTERVAL_SECS) {
			return Ok(());
		}
	}
	refresh_tracked_solvers(db, chain).await?;
	let offset = head.map_or(0, |(_, offset)| offset);
	let offset = refresh_page(db, chain, reads, block_number, at, offset).await?;
	db.execute(
		"INSERT INTO app.solver_inventory_heads (id, block_number, observed_at, refresh_offset)
		 VALUES ($1, $2, $3, $4)
		 ON CONFLICT (id) DO UPDATE SET block_number = EXCLUDED.block_number,
		   observed_at = EXCLUDED.observed_at, refresh_offset = EXCLUDED.refresh_offset",
		&[&chain.id, &numeric(block_number), &at, &offset],
	)
	.await?;
	Ok(())
}

/**
 * `indexSolverInventoryBlock`, run after a batch at its last block: every log up to `block` is
 * applied, so reads pinned to it are positioned after all of them.
 */
pub async fn index_block(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	block_number: u64,
	timestamp: u64,
) -> anyhow::Result<()> {
	if chain.tokens.is_empty() {
		return Ok(());
	}
	let at = timestamp_to_date(timestamp);
	consume_watchlist(db, chain, block_number, timestamp).await?;
	seed_pending_solvers(db, chain, reads, block_number, at).await?;
	advance_head(db, chain, reads, block_number, at).await
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_log_applies_only_after_the_rows_position() {
		assert!(is_after(102, 0, 101, AFTER_EVERY_LOG));
		assert!(!is_after(101, 3, 101, AFTER_EVERY_LOG));
		assert!(is_after(102, 5, 102, 4));
		assert!(!is_after(102, 4, 102, 4));
		assert!(!is_after(102, 2, 102, 4));
		assert!(!is_after(100, 7, 101, AFTER_EVERY_LOG));
	}

	#[test]
	fn elapsed_time_treats_a_missing_clock_as_due() {
		let t0 = timestamp_to_date(1_750_000_000);
		assert!(elapsed_at_least(None, t0, RECONCILE_INTERVAL_SECS));
		assert!(!elapsed_at_least(Some(t0), timestamp_to_date(1_750_000_029), 30));
		assert!(elapsed_at_least(Some(t0), timestamp_to_date(1_750_000_030), 30));
	}
}
