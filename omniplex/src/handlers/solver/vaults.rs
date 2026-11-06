//! Yield vault accounting for our solvers' ERC-4626 positions (`yieldVault.service.ts` and
//! `vaultAccounting.ts`): a ledger of capital movements folded into each LP's position, and a
//! daily snapshot of each position's value against its net principal.

use alloy::{
	primitives::{I256, U256},
	rpc::types::{Filter, Log},
	sol_types::SolEvent,
};
use anyhow::{anyhow, bail, Context};
use chrono::NaiveDateTime;
use tokio_postgres::{Row, Transaction};

use super::{
	code_delegates_to_solver_account,
	history::{int, opt_int},
	lower,
	reads::{map_concurrently, Reads},
	signed, unsigned, SolverChain, DELEGATION_INDICATOR_PREFIX, ZERO_ADDRESS,
};
use crate::{
	handlers::abi::Erc4626,
	store::{numeric, timestamp_to_date, VaultLedgerEventType},
};

const SECONDS_PER_DAY: u64 = 86_400;
/** LP positions read per page during a snapshot. */
const LP_PAGE_SIZE: i64 = 100;
/** SubQuery ran the snapshot every 50th block. */
pub const SNAPSHOT_MODULO: u64 = 50;

/** `isOrdinaryVaultTransfer`: mints, burns, self and zero-share transfers move no capital. */
pub fn is_ordinary_vault_transfer(from: &str, to: &str, shares: I256) -> bool {
	let (from, to) = (from.to_lowercase(), to.to_lowercase());
	from != ZERO_ADDRESS && to != ZERO_ADDRESS && from != to && !shares.is_zero()
}

/** One LP's share change in a block, from a Deposit, Withdraw or ordinary Transfer. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movement {
	pub transaction_hash: String,
	pub log_index: u64,
	pub lp: String,
	pub shares: I256,
}

/** `vaultCapitalMovements`: the share changes one vault log represents. */
pub fn capital_movements(
	log: &alloy::primitives::Log,
	transaction_hash: &str,
	log_index: u64,
) -> anyhow::Result<Vec<Movement>> {
	let movement = |lp: alloy::primitives::Address, shares: I256| Movement {
		transaction_hash: transaction_hash.to_lowercase(),
		log_index,
		lp: lower(lp),
		shares,
	};
	let topic = log.topics().first().copied().unwrap_or_default();
	if topic == Erc4626::Deposit::SIGNATURE_HASH {
		let event = Erc4626::Deposit::decode_log(log)?;
		return Ok(vec![movement(event.owner, signed(event.shares)?)]);
	}
	if topic == Erc4626::Withdraw::SIGNATURE_HASH {
		let event = Erc4626::Withdraw::decode_log(log)?;
		return Ok(vec![movement(event.owner, -signed(event.shares)?)]);
	}
	let event = Erc4626::Transfer::decode_log(log)?;
	let shares = signed(event.value)?;
	if !is_ordinary_vault_transfer(&lower(event.from), &lower(event.to), shares) {
		return Ok(vec![]);
	}
	Ok(vec![movement(event.from, -shares), movement(event.to, shares)])
}

/**
 * `readVaultBlockMovements`'s checks over one block's vault logs: each must be the vault's, of
 * that block, not removed, and have a log index seen once.
 */
pub fn block_movements(vault: &str, block: u64, logs: &[Log]) -> anyhow::Result<Vec<Movement>> {
	let mut seen = std::collections::HashSet::new();
	let mut movements = vec![];
	for log in logs {
		let log_index = log.log_index.ok_or_else(|| anyhow!("a vault log has no index"))?;
		// A node that answers like this is broken or behind, and another answer can be right.
		if log.removed ||
			log.block_number != Some(block) ||
			lower(log.inner.address) != vault.to_lowercase()
		{
			return Err(anyhow!(crate::errors::Retry(format!(
				"Unexpected vault log for {vault}:{block}"
			))));
		}
		if !seen.insert(log_index) {
			let duplicate = format!("Duplicate vault log for {vault}:{block}:{log_index}");
			return Err(anyhow!(crate::errors::Retry(duplicate)));
		}
		let hash = log.transaction_hash.map(|h| format!("{h:#x}")).unwrap_or_default();
		movements.extend(capital_movements(&log.inner, &hash, log_index)?);
	}
	Ok(movements)
}

/** `readVaultBlockMovements`: one block's capital movements in `vault`, never cached. */
pub async fn read_vault_block_movements(
	reads: &dyn Reads,
	chain: &SolverChain,
	vault: &str,
	block: u64,
) -> anyhow::Result<Vec<Movement>> {
	let filter = Filter::new()
		.address(vault.parse::<alloy::primitives::Address>()?)
		.event_signature(vec![
			Erc4626::Deposit::SIGNATURE_HASH,
			Erc4626::Withdraw::SIGNATURE_HASH,
			Erc4626::Transfer::SIGNATURE_HASH,
		])
		.from_block(block)
		.to_block(block);
	let logs = reads.logs(&filter).await.with_context(|| {
		format!("Could not read vault capital movements for {}:{vault}:{block}", chain.id)
	})?;
	block_movements(vault, block, &logs)
}

/** The inputs of one ledger entry. Addresses lowercase. */
#[derive(Debug, Clone)]
pub struct LedgerInput {
	pub vault: String,
	/** The share owner whose principal moved. */
	pub lp: String,
	pub caller: String,
	pub receiver: Option<String>,
	pub assets: I256,
	pub shares: I256,
	pub event_type: VaultLedgerEventType,
	pub block_number: u64,
	pub transaction_hash: String,
	pub log_index: u64,
	/** Block time in seconds. */
	pub timestamp: u64,
}

/** An ordinary vault share Transfer. */
#[derive(Debug, Clone)]
pub struct TransferInput {
	pub vault: String,
	pub from: String,
	pub to: String,
	pub shares: I256,
	pub block_number: u64,
	pub transaction_hash: String,
	pub log_index: u64,
	pub timestamp: u64,
}

fn is_capital_in(event_type: VaultLedgerEventType) -> bool {
	matches!(event_type, VaultLedgerEventType::Deposit | VaultLedgerEventType::TransferIn)
}

fn capital_event_id(
	chain: &str,
	transaction_hash: &str,
	log_index: u64,
	lp: Option<&str>,
) -> String {
	let suffix = lp.map(|lp| format!("-{}", lp.to_lowercase())).unwrap_or_default();
	format!("{chain}-{}-{log_index}{suffix}", transaction_hash.to_lowercase())
}

/**
 * `isDelegatedSolver`: `lp` delegates to one of our SolverAccounts at `block`. Code other than
 * a delegation designator is a contract's, and a contract never delegates, so its code is read
 * once. A wallet can delegate at any block, so its code is read at every event.
 */
pub async fn is_delegated_solver(
	chain: &SolverChain,
	reads: &dyn Reads,
	lp: &str,
	block: u64,
) -> anyhow::Result<bool> {
	if chain.solver_accounts.is_empty() || chain.contracts.lock().unwrap().contains(lp) {
		return Ok(false);
	}
	// A failed RPC is not evidence of an unrelated wallet: the batch is retried.
	let code = reads.code(lp.parse()?, block).await.with_context(|| {
		format!("[yield-vault] Delegation check failed for {lp} on {}", chain.id)
	})?;
	let code = alloy::hex::encode_prefixed(&code);
	if code != "0x" && !code.starts_with(DELEGATION_INDICATOR_PREFIX) {
		chain.contracts.lock().unwrap().insert(lp.to_string());
		return Ok(false);
	}
	Ok(code_delegates_to_solver_account(chain, &code))
}

async fn ledger_exists(db: &Transaction<'_>, id: &str) -> anyhow::Result<bool> {
	Ok(db
		.query_opt("SELECT 1 FROM app.vault_ledger_events WHERE id = $1", &[&id])
		.await?
		.is_some())
}

/** One `vault_lp_positions` row. */
#[derive(Debug, Clone)]
pub struct LpPosition {
	pub id: String,
	pub lp: String,
	pub shares: I256,
	pub opening_shares: Option<I256>,
	pub opening_principal: Option<I256>,
	pub opening_block: Option<u64>,
	pub total_assets_transferred_in: Option<I256>,
	pub total_assets_transferred_out: Option<I256>,
	pub total_assets_deposited: I256,
	pub total_assets_withdrawn: I256,
	pub deposit_count: i32,
	pub withdraw_count: i32,
	pub created_at: NaiveDateTime,
	pub last_updated_at: NaiveDateTime,
}

const POSITION_COLUMNS: &str = "id, lp, shares, opening_shares, opening_principal, opening_block, \
	total_assets_transferred_in, total_assets_transferred_out, total_assets_deposited, \
	total_assets_withdrawn, deposit_count, withdraw_count, created_at, last_updated_at";

impl LpPosition {
	fn from_row(row: &Row) -> anyhow::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			lp: row.try_get("lp")?,
			shares: int(row, "shares")?,
			opening_shares: opt_int(row, "opening_shares")?,
			opening_principal: opt_int(row, "opening_principal")?,
			opening_block: opt_int(row, "opening_block")?.map(|b| b.as_u64()),
			total_assets_transferred_in: opt_int(row, "total_assets_transferred_in")?,
			total_assets_transferred_out: opt_int(row, "total_assets_transferred_out")?,
			total_assets_deposited: int(row, "total_assets_deposited")?,
			total_assets_withdrawn: int(row, "total_assets_withdrawn")?,
			deposit_count: row.try_get("deposit_count")?,
			withdraw_count: row.try_get("withdraw_count")?,
			created_at: row.try_get("created_at")?,
			last_updated_at: row.try_get("last_updated_at")?,
		})
	}
}

async fn load_position(db: &Transaction<'_>, id: &str) -> anyhow::Result<Option<LpPosition>> {
	db.query_opt(
		&format!("SELECT {POSITION_COLUMNS} FROM app.vault_lp_positions WHERE id = $1"),
		&[&id],
	)
	.await?
	.as_ref()
	.map(LpPosition::from_row)
	.transpose()
}

/** `tracksLp`: an LP with a position, or one delegated to a SolverAccount. */
async fn tracks_lp(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	vault: &str,
	lp: &str,
	block: u64,
) -> anyhow::Result<bool> {
	let id = format!("{}-{vault}-{lp}", chain.id);
	Ok(load_position(db, &id).await?.is_some() ||
		is_delegated_solver(chain, reads, lp, block).await?)
}

async fn convert_to_assets(
	reads: &dyn Reads,
	vault: &str,
	shares: I256,
	block: u64,
) -> anyhow::Result<I256> {
	use alloy::sol_types::SolCall;
	let call = Erc4626::convertToAssetsCall { shares: unsigned(shares) };
	let answer = reads.call(vault.parse()?, call.abi_encode().into(), block).await?;
	signed(Erc4626::convertToAssetsCall::abi_decode_returns(&answer)?)
}

async fn balance_of(reads: &dyn Reads, vault: &str, lp: &str, block: u64) -> anyhow::Result<I256> {
	use alloy::sol_types::SolCall;
	let call = Erc4626::balanceOfCall { account: lp.parse()? };
	let answer = reads.call(vault.parse()?, call.abi_encode().into(), block).await?;
	signed(Erc4626::balanceOfCall::abi_decode_returns(&answer)?)
}

/** `recordTransfer`: a share transfer is capital moving between owners, valued at the block. */
pub async fn record_transfer(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	input: &TransferInput,
) -> anyhow::Result<()> {
	let (from, to, vault) =
		(input.from.to_lowercase(), input.to.to_lowercase(), input.vault.to_lowercase());
	if !is_ordinary_vault_transfer(&from, &to, input.shares) {
		return Ok(());
	}
	if chain.token_for_vault(&vault).is_none() {
		return Ok(());
	}
	let mut owners = vec![];
	for lp in [&from, &to] {
		let id = capital_event_id(&chain.id, &input.transaction_hash, input.log_index, Some(lp));
		if ledger_exists(db, &id).await? {
			continue;
		}
		if tracks_lp(db, chain, reads, &vault, lp, input.block_number).await? {
			owners.push(lp.clone());
		}
	}
	if owners.is_empty() {
		return Ok(());
	}
	let assets = convert_to_assets(reads, &vault, input.shares, input.block_number).await?;
	// One log can affect two tracked LPs, each with its own ledger key.
	for lp in owners {
		let event_type = if lp == from {
			VaultLedgerEventType::TransferOut
		} else {
			VaultLedgerEventType::TransferIn
		};
		record_ledger(
			db,
			chain,
			reads,
			&LedgerInput {
				vault: vault.clone(),
				lp,
				caller: from.clone(),
				receiver: Some(to.clone()),
				assets,
				shares: input.shares,
				event_type,
				block_number: input.block_number,
				transaction_hash: input.transaction_hash.clone(),
				log_index: input.log_index,
				timestamp: input.timestamp,
			},
		)
		.await?;
	}
	Ok(())
}

/**
 * `openingBalance`: capital held before tracking began. balanceOf is end-of-block, so every
 * movement of the LP from the triggering log onwards in that block is backed out.
 */
async fn opening_balance(
	chain: &SolverChain,
	reads: &dyn Reads,
	input: &LedgerInput,
) -> anyhow::Result<(I256, I256)> {
	let movements =
		read_vault_block_movements(reads, chain, &input.vault, input.block_number).await?;
	let delta = if is_capital_in(input.event_type) { input.shares } else { -input.shares };
	let hash = input.transaction_hash.to_lowercase();
	if !movements.iter().any(|m| {
		m.lp == input.lp &&
			m.log_index == input.log_index &&
			m.transaction_hash == hash &&
			m.shares == delta
	}) {
		// What a node answers before it has the block: the batch retries until it has.
		return Err(anyhow!(crate::errors::Retry(format!(
			"Vault opening balance is missing the triggering log for {}:{}:{}",
			chain.id, input.vault, input.lp
		))));
	}
	let remaining = movements
		.iter()
		.filter(|m| m.lp == input.lp && m.log_index >= input.log_index)
		.fold(I256::ZERO, |sum, m| sum + m.shares);
	let shares = balance_of(reads, &input.vault, &input.lp, input.block_number).await? - remaining;
	if shares.is_negative() {
		bail!("Negative vault opening shares for {}:{}:{}", chain.id, input.vault, input.lp);
	}
	let assets = if shares.is_zero() {
		I256::ZERO
	} else {
		convert_to_assets(reads, &input.vault, shares, input.block_number).await?
	};
	Ok((shares, assets))
}

/**
 * `recordLedger`: persists one capital movement of one of our solvers and folds it into the
 * LP's position, in the batch's transaction.
 */
pub async fn record_ledger(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	input: &LedgerInput,
) -> anyhow::Result<()> {
	let vault = input.vault.to_lowercase();
	let lp = input.lp.to_lowercase();
	let Some(underlying_token) = chain.token_for_vault(&vault).map(str::to_string) else {
		tracing::warn!(
			"[yield-vault] Unconfigured vault {vault} on {}, skipping ledger event",
			chain.id
		);
		return Ok(());
	};
	let position_id = format!("{}-{vault}-{lp}", chain.id);
	let existing = load_position(db, &position_id).await?;
	if existing.is_none() && !is_delegated_solver(chain, reads, &lp, input.block_number).await? {
		return Ok(());
	}

	let is_transfer = matches!(
		input.event_type,
		VaultLedgerEventType::TransferIn | VaultLedgerEventType::TransferOut
	);
	let ledger_id = capital_event_id(
		&chain.id,
		&input.transaction_hash,
		input.log_index,
		is_transfer.then_some(lp.as_str()),
	);
	if ledger_exists(db, &ledger_id).await? {
		tracing::debug!("[yield-vault] Ledger event {ledger_id} already recorded, skipping");
		return Ok(());
	}

	let event_time = timestamp_to_date(input.timestamp);
	let input = LedgerInput { vault: vault.clone(), lp: lp.clone(), ..input.clone() };
	let opening = match existing {
		Some(_) => None,
		None => Some(opening_balance(chain, reads, &input).await?),
	};

	db.execute(
		"INSERT INTO app.vault_ledger_events
		   (id, chain, vault, underlying_token, lp, caller, receiver, event_type, assets, shares,
		    block_number, transaction_hash, timestamp)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
		&[
			&ledger_id,
			&chain.id,
			&vault,
			&underlying_token,
			&lp,
			&input.caller.to_lowercase(),
			&input.receiver.as_ref().map(|r| r.to_lowercase()),
			&input.event_type,
			&numeric(input.assets),
			&numeric(input.shares),
			&numeric(input.block_number),
			&input.transaction_hash.to_lowercase(),
			&event_time,
		],
	)
	.await?;

	let mut position = existing.unwrap_or_else(|| {
		let (shares, assets) = opening.expect("opening balance of a new position");
		LpPosition {
			id: position_id.clone(),
			lp: lp.clone(),
			shares,
			opening_shares: Some(shares),
			opening_principal: Some(assets),
			opening_block: Some(input.block_number),
			total_assets_transferred_in: Some(I256::ZERO),
			total_assets_transferred_out: Some(I256::ZERO),
			total_assets_deposited: I256::ZERO,
			total_assets_withdrawn: I256::ZERO,
			deposit_count: 0,
			withdraw_count: 0,
			created_at: event_time,
			last_updated_at: event_time,
		}
	});
	match input.event_type {
		VaultLedgerEventType::Deposit => {
			position.total_assets_deposited += input.assets;
			position.shares += input.shares;
			position.deposit_count += 1;
		},
		VaultLedgerEventType::Withdraw => {
			position.total_assets_withdrawn += input.assets;
			position.shares -= input.shares;
			position.withdraw_count += 1;
		},
		VaultLedgerEventType::TransferIn => {
			position.total_assets_transferred_in =
				Some(position.total_assets_transferred_in.unwrap_or_default() + input.assets);
			position.shares += input.shares;
		},
		VaultLedgerEventType::TransferOut => {
			position.total_assets_transferred_out =
				Some(position.total_assets_transferred_out.unwrap_or_default() + input.assets);
			position.shares -= input.shares;
		},
	}
	position.last_updated_at = event_time;

	db.execute(
		&format!(
			"INSERT INTO app.vault_lp_positions ({POSITION_COLUMNS}, chain, vault, underlying_token)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
			 ON CONFLICT (id) DO UPDATE SET lp = EXCLUDED.lp, shares = EXCLUDED.shares,
			   opening_shares = EXCLUDED.opening_shares, opening_principal = EXCLUDED.opening_principal,
			   opening_block = EXCLUDED.opening_block,
			   total_assets_transferred_in = EXCLUDED.total_assets_transferred_in,
			   total_assets_transferred_out = EXCLUDED.total_assets_transferred_out,
			   total_assets_deposited = EXCLUDED.total_assets_deposited,
			   total_assets_withdrawn = EXCLUDED.total_assets_withdrawn,
			   deposit_count = EXCLUDED.deposit_count, withdraw_count = EXCLUDED.withdraw_count,
			   created_at = EXCLUDED.created_at, last_updated_at = EXCLUDED.last_updated_at,
			   chain = EXCLUDED.chain, vault = EXCLUDED.vault,
			   underlying_token = EXCLUDED.underlying_token"
		),
		&[
			&position.id,
			&position.lp,
			&numeric(position.shares),
			&position.opening_shares.map(numeric),
			&position.opening_principal.map(numeric),
			&position.opening_block.map(numeric),
			&position.total_assets_transferred_in.map(numeric),
			&position.total_assets_transferred_out.map(numeric),
			&numeric(position.total_assets_deposited),
			&numeric(position.total_assets_withdrawn),
			&position.deposit_count,
			&position.withdraw_count,
			&position.created_at,
			&position.last_updated_at,
			&chain.id,
			&vault,
			&underlying_token,
		],
	)
	.await?;
	Ok(())
}

/**
 * Whether the snapshot hook runs at `block`: SubQuery ran it at every 50th block, so it runs
 * when a batch reaches a multiple of 50 past `previous_block`, the chain's last committed
 * block. That comes from the cursor, so a block that rolls back is due again when it is
 * retried.
 */
pub fn snapshot_due(previous_block: Option<u64>, block: u64) -> bool {
	previous_block.is_none_or(|previous| block / SNAPSHOT_MODULO > previous / SNAPSHOT_MODULO)
}

enum LpOutcome {
	Complete,
	Retry,
	Unreconciled,
}

/**
 * `snapshotChain`: the daily snapshot of every configured vault and its LPs, idempotent per UTC
 * day. The vault row is the day's completion gate and is written only once no LP needs a retry.
 *
 * It runs after the batch's logs, so every position already holds every log up to `block`:
 * SubQuery's block handler ran before its block's logs and so deferred LPs that moved in it,
 * which this has no need to.
 */
pub async fn snapshot_chain(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	block: u64,
	timestamp: u64,
) -> anyhow::Result<()> {
	use alloy::sol_types::SolCall;
	let day_start = timestamp / SECONDS_PER_DAY * SECONDS_PER_DAY;
	let snapshot_time = timestamp_to_date(timestamp);

	for (vault, underlying_token) in chain.vaults() {
		let vault_snapshot_id = format!("{}-{vault}-{day_start}", chain.id);
		if db
			.query_opt("SELECT 1 FROM app.vault_snapshots WHERE id = $1", &[&vault_snapshot_id])
			.await?
			.is_some()
		{
			continue;
		}

		let read_vault = async {
			let target = vault.parse()?;
			let call = |data: Vec<u8>| reads.call(target, data.into(), block);
			let (assets, supply, decimals) = futures::try_join!(
				call(Erc4626::totalAssetsCall {}.abi_encode()),
				call(Erc4626::totalSupplyCall {}.abi_encode()),
				call(Erc4626::decimalsCall {}.abi_encode()),
			)?;
			let decimals = Erc4626::decimalsCall::abi_decode_returns(&decimals)?;
			let one_share = U256::from(10).pow(U256::from(decimals));
			let per_share =
				call(Erc4626::convertToAssetsCall { shares: one_share }.abi_encode()).await?;
			anyhow::Ok((
				signed(Erc4626::totalAssetsCall::abi_decode_returns(&assets)?)?,
				signed(Erc4626::totalSupplyCall::abi_decode_returns(&supply)?)?,
				signed(Erc4626::convertToAssetsCall::abi_decode_returns(&per_share)?)?,
			))
		};
		let (total_assets, total_shares, assets_per_share) = match read_vault.await {
			Ok(read) => read,
			Err(err) => {
				tracing::error!(
					"[yield-vault] Vault read failed for {vault} on {}: {err:#}",
					chain.id
				);
				continue;
			},
		};

		if !snapshot_lp_positions(
			db,
			chain,
			reads,
			vault,
			underlying_token,
			day_start,
			block,
			snapshot_time,
		)
		.await?
		{
			continue;
		}

		db.execute(
			"INSERT INTO app.vault_snapshots
			   (id, chain, vault, underlying_token, day_start_timestamp, total_assets, total_shares,
			    assets_per_share, block_number, snapshot_time)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
			&[
				&vault_snapshot_id,
				&chain.id,
				&vault,
				&underlying_token,
				&numeric(day_start),
				&numeric(total_assets),
				&numeric(total_shares),
				&numeric(assets_per_share),
				&numeric(block),
				&snapshot_time,
			],
		)
		.await?;
	}
	Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn snapshot_lp_positions(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	vault: &str,
	underlying_token: &str,
	day_start: u64,
	block: u64,
	snapshot_time: NaiveDateTime,
) -> anyhow::Result<bool> {
	let mut offset = 0;
	let mut complete = true;
	loop {
		let positions = db
			.query(
				&format!(
					"SELECT {POSITION_COLUMNS} FROM app.vault_lp_positions
					 WHERE chain = $1 AND vault = $2 ORDER BY id ASC LIMIT $3 OFFSET $4"
				),
				&[&chain.id, &vault, &LP_PAGE_SIZE, &offset],
			)
			.await?
			.iter()
			.map(LpPosition::from_row)
			.collect::<anyhow::Result<Vec<_>>>()?;
		if positions.is_empty() {
			break;
		}
		let count = positions.len() as i64;
		let outcomes = map_concurrently(positions, |position| {
			snapshot_lp_position(
				db,
				chain,
				reads,
				vault,
				underlying_token,
				position,
				day_start,
				block,
				snapshot_time,
			)
		})
		.await;
		for outcome in crate::errors::first_failure(outcomes)? {
			if matches!(outcome, LpOutcome::Retry) {
				complete = false;
			}
		}
		if count < LP_PAGE_SIZE {
			break;
		}
		offset += LP_PAGE_SIZE;
	}
	Ok(complete)
}

#[allow(clippy::too_many_arguments)]
async fn snapshot_lp_position(
	db: &Transaction<'_>,
	chain: &SolverChain,
	reads: &dyn Reads,
	vault: &str,
	underlying_token: &str,
	position: LpPosition,
	day_start: u64,
	block: u64,
	snapshot_time: NaiveDateTime,
) -> anyhow::Result<LpOutcome> {
	let snapshot_id = format!("{}-{vault}-{}-{day_start}", chain.id, position.lp);
	if db
		.query_opt("SELECT 1 FROM app.vault_position_snapshots WHERE id = $1", &[&snapshot_id])
		.await?
		.is_some()
	{
		return Ok(LpOutcome::Complete);
	}

	let read = async {
		let shares = balance_of(reads, vault, &position.lp, block).await?;
		let assets = convert_to_assets(reads, vault, shares, block).await?;
		anyhow::Ok((shares, assets))
	};
	let (shares, asset_value) = match read.await {
		Ok(read) => read,
		Err(err) => {
			tracing::error!("[yield-vault] LP read failed for {} on {vault}: {err:#}", position.lp);
			return Ok(LpOutcome::Retry);
		},
	};

	if shares != position.shares {
		tracing::error!(
			"[yield-vault] Withholding yield for {}:{vault}:{}: accounted shares={}, onchain shares={shares}; principal requires reconciliation",
			chain.id,
			position.lp,
			position.shares
		);
		return Ok(LpOutcome::Unreconciled);
	}

	let net_principal = position.opening_principal.unwrap_or_default() +
		position.total_assets_deposited -
		position.total_assets_withdrawn +
		position.total_assets_transferred_in.unwrap_or_default() -
		position.total_assets_transferred_out.unwrap_or_default();
	let yield_earned = asset_value - net_principal;
	if yield_earned.is_negative() {
		tracing::warn!(
			"[yield-vault] Negative yield for {}:{vault}:{} (assetValue={asset_value}, netPrincipal={net_principal})",
			chain.id,
			position.lp
		);
	}

	db.execute(
		"INSERT INTO app.vault_position_snapshots
		   (id, chain, vault, underlying_token, lp, day_start_timestamp, shares, asset_value,
		    net_principal, yield_earned, block_number, snapshot_time)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
		&[
			&snapshot_id,
			&chain.id,
			&vault,
			&underlying_token,
			&position.lp,
			&numeric(day_start),
			&numeric(shares),
			&numeric(asset_value),
			&numeric(net_principal),
			&numeric(yield_earned),
			&numeric(block),
			&snapshot_time,
		],
	)
	.await?;
	Ok(LpOutcome::Complete)
}

#[cfg(test)]
mod tests {
	use alloy::{
		primitives::{address, b256, Address, LogData},
		sol_types::SolEvent,
	};

	use super::*;

	const FROM: Address = address!("1111111111111111111111111111111111111111");
	const TO: Address = address!("2222222222222222222222222222222222222222");
	const VAULT: Address = address!("c768c589647798a6ee01a91fde98ef2ed046dbd6");
	const TX: &str = "0xabababababababababababababababababababababababababababababababab";

	fn log(event: impl SolEvent) -> alloy::primitives::Log {
		alloy::primitives::Log { address: VAULT, data: event.encode_log_data() }
	}

	fn transfer(from: Address, to: Address, value: u64) -> alloy::primitives::Log {
		log(Erc4626::Transfer { from, to, value: U256::from(value) })
	}

	fn rpc_log(inner: alloy::primitives::Log, block: u64, index: u64) -> Log {
		Log {
			inner,
			block_number: Some(block),
			log_index: Some(index),
			transaction_hash: Some(b256!(
				"abababababababababababababababababababababababababababababababab"
			)),
			..Default::default()
		}
	}

	fn shares(value: i64) -> I256 {
		I256::try_from(value).unwrap()
	}

	#[test]
	fn deposits_and_withdrawals_move_the_share_owner() {
		let deposit = log(Erc4626::Deposit {
			sender: FROM,
			owner: TO,
			assets: U256::from(110),
			shares: U256::from(100),
		});
		assert_eq!(
			capital_movements(&deposit, TX, 2).unwrap(),
			vec![Movement {
				transaction_hash: TX.into(),
				log_index: 2,
				lp: lower(TO),
				shares: shares(100)
			}]
		);
		let withdraw = log(Erc4626::Withdraw {
			sender: FROM,
			receiver: FROM,
			owner: TO,
			assets: U256::from(110),
			shares: U256::from(100),
		});
		assert_eq!(capital_movements(&withdraw, TX, 2).unwrap()[0].shares, shares(-100));
		assert_eq!(capital_movements(&withdraw, TX, 2).unwrap()[0].lp, lower(TO));
	}

	#[test]
	fn ordinary_transfers_move_both_sides_and_others_move_nothing() {
		assert_eq!(
			capital_movements(&transfer(FROM, TO, 100), TX, 2).unwrap(),
			vec![
				Movement {
					transaction_hash: TX.into(),
					log_index: 2,
					lp: lower(FROM),
					shares: shares(-100)
				},
				Movement {
					transaction_hash: TX.into(),
					log_index: 2,
					lp: lower(TO),
					shares: shares(100)
				},
			]
		);
		for (from, to, value) in
			[(Address::ZERO, TO, 100), (FROM, Address::ZERO, 100), (FROM, FROM, 100), (FROM, TO, 0)]
		{
			assert!(capital_movements(&transfer(from, to, value), TX, 2).unwrap().is_empty());
		}
	}

	#[test]
	fn block_movements_keep_the_log_index_and_reject_mismatched_logs() {
		let vault = lower(VAULT);
		let ok = rpc_log(transfer(FROM, TO, 100), 50, 10);
		let movements = block_movements(&vault, 50, &[ok.clone()]).unwrap();
		assert_eq!(movements.iter().map(|m| m.log_index).collect::<Vec<_>>(), vec![10, 10]);

		let other_block = rpc_log(transfer(FROM, TO, 100), 49, 2);
		let removed = Log { removed: true, ..rpc_log(transfer(FROM, TO, 100), 50, 2) };
		let other_address =
			rpc_log(alloy::primitives::Log { address: FROM, data: LogData::default() }, 50, 2);
		for bad in [other_block, removed, other_address] {
			assert!(block_movements(&vault, 50, &[bad]).is_err());
		}
		let duplicate = block_movements(&vault, 50, &[ok.clone(), ok]).unwrap_err();
		assert!(duplicate.to_string().contains("Duplicate"));
	}
}
