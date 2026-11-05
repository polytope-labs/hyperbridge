//! Solver inventory, solver discovery and yield vault accounting against the real `app` schema,
//! ported from `solverInventory.service.test.ts`, `solverWatchlist.service.test.ts` and
//! `yieldVault.service.test.ts`. Chain reads are answered by an in-memory model. Set
//! `INDEXER_TEST_DATABASE_URL` to run. Each test uses its own chain id, so concurrent tests never
//! wait on each other's uncommitted rows.

mod common;

use std::{
	collections::{HashMap, HashSet},
	sync::{
		atomic::{AtomicBool, AtomicUsize, Ordering},
		Mutex,
	},
};

use alloy::{
	primitives::{Address, Bytes, LogData, B256, I256, U256},
	rpc::types::{Filter, Log},
	sol_types::{SolCall, SolEvent},
};
use async_trait::async_trait;
use common::{database, text};
use omniplex::{
	config::ChainConfig,
	handlers::{
		abi::Erc4626,
		solver::{
			history::{load_inventory, save_inventory, Inventory},
			inventory::{
				self, apply_token_transfer, apply_vault_share_transfer, index_block, Transfer,
				AFTER_EVERY_LOG, HEAD_INTERVAL_SECS, RECONCILE_INTERVAL_SECS,
				REVALUE_INTERVAL_SECS,
			},
			lower,
			reads::{Multicall3, Reads, MULTICALL3_ADDRESS},
			vaults::{self, LedgerInput, TransferInput},
			watchlist::{queue_requests, Poller},
			SolverChain,
		},
	},
	services::solvers::discover_solver_from_fill,
	store::{timestamp_secs, SolverDiscoveryTrigger, VaultLedgerEventType as Type},
};
use tokio_postgres::Transaction;

const USDC: &str = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913";
const CNGN: &str = "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f";
const VAULT: &str = "0xc768c589647798a6ee01a91fde98ef2ed046dbd6";
const SOLVER: &str = "0xce319986ca4d5d0893751a628d0db3dc8fc91d62";
const OTHER: &str = "0x13e41cde1d55880cbe031c69f206c2e9bc3c94c2";
const THIRD: &str = "0x18f23e630077b1da3ed97c0469d0504a93fad9e2";
const ZERO: &str = "0x0000000000000000000000000000000000000000";
const SOLVER_ACCOUNT: &str = "0xaad062555800a97af062795189e32a3cbd045612";
const T0: u64 = 1_750_000_000;

fn addr(value: &str) -> Address {
	value.parse().unwrap()
}

fn int(value: i128) -> I256 {
	I256::try_from(value).unwrap()
}

fn delegated_to(account: &str) -> Bytes {
	format!("0xef0100{}", &account[2..]).parse().unwrap()
}

/** Chain state the pinned reads answer from, and counts of what was read. */
struct Model {
	/** `(contract, holder)` → balance. */
	balances: Mutex<HashMap<(Address, Address), U256>>,
	/** Assets per share, per vault, as (numerator, denominator). */
	rates: Mutex<HashMap<Address, (u128, u128)>>,
	/** Exact convertToAssets answers that override the rate. */
	prices: Mutex<HashMap<U256, U256>>,
	/** Holders whose balanceOf reverts. */
	reverting: Mutex<HashSet<Address>>,
	/** Holders whose balanceOf fails as an RPC error, outside any multicall. */
	failing_holders: Mutex<HashSet<Address>>,
	/** The code of every account but Multicall3. */
	code: Mutex<HashMap<Address, Bytes>>,
	default_code: Mutex<Bytes>,
	code_failures: AtomicUsize,
	convert_failures: AtomicUsize,
	vault_reads_fail: AtomicBool,
	multicall_deployed: AtomicBool,
	logs: Mutex<Vec<Log>>,
	logs_fail: AtomicBool,
	eth_calls: Mutex<Vec<Address>>,
	balance_reads: AtomicUsize,
	convert_reads: AtomicUsize,
	code_reads: AtomicUsize,
}

impl Model {
	fn new() -> Self {
		Self {
			balances: Default::default(),
			rates: Default::default(),
			prices: Default::default(),
			reverting: Default::default(),
			failing_holders: Default::default(),
			code: Default::default(),
			default_code: Mutex::new(Bytes::new()),
			code_failures: AtomicUsize::new(0),
			convert_failures: AtomicUsize::new(0),
			vault_reads_fail: AtomicBool::new(false),
			multicall_deployed: AtomicBool::new(true),
			logs: Default::default(),
			logs_fail: AtomicBool::new(false),
			eth_calls: Default::default(),
			balance_reads: AtomicUsize::new(0),
			convert_reads: AtomicUsize::new(0),
			code_reads: AtomicUsize::new(0),
		}
	}

	fn set_balance(&self, contract: &str, holder: &str, value: u128) {
		self.balances
			.lock()
			.unwrap()
			.insert((addr(contract), addr(holder)), U256::from(value));
	}

	fn set_rate(&self, vault: &str, numerator: u128, denominator: u128) {
		self.rates.lock().unwrap().insert(addr(vault), (numerator, denominator));
	}

	fn eth_call_count(&self) -> usize {
		self.eth_calls.lock().unwrap().len()
	}

	fn reset_counts(&self) {
		self.eth_calls.lock().unwrap().clear();
		self.balance_reads.store(0, Ordering::SeqCst);
		self.convert_reads.store(0, Ordering::SeqCst);
		self.code_reads.store(0, Ordering::SeqCst);
	}

	fn answer(&self, to: Address, data: &[u8]) -> anyhow::Result<Bytes> {
		if to == MULTICALL3_ADDRESS {
			let call = Multicall3::aggregate3Call::abi_decode(data)?;
			let results: Vec<Multicall3::Result> = call
				.calls
				.iter()
				.map(|c| match self.answer(c.target, &c.callData) {
					Ok(data) => Multicall3::Result { success: true, returnData: data },
					Err(_) => Multicall3::Result { success: false, returnData: Bytes::new() },
				})
				.collect();
			return Ok(Multicall3::aggregate3Call::abi_encode_returns(&results).into());
		}
		let selector: [u8; 4] = data[..4].try_into()?;
		if selector == Erc4626::balanceOfCall::SELECTOR {
			let call = Erc4626::balanceOfCall::abi_decode(data)?;
			self.balance_reads.fetch_add(1, Ordering::SeqCst);
			if self.reverting.lock().unwrap().contains(&call.account) {
				anyhow::bail!("execution reverted");
			}
			let balance = self
				.balances
				.lock()
				.unwrap()
				.get(&(to, call.account))
				.copied()
				.unwrap_or_default();
			return Ok(Erc4626::balanceOfCall::abi_encode_returns(&balance).into());
		}
		if selector == Erc4626::convertToAssetsCall::SELECTOR {
			let call = Erc4626::convertToAssetsCall::abi_decode(data)?;
			self.convert_reads.fetch_add(1, Ordering::SeqCst);
			if self
				.convert_failures
				.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
				.is_ok()
			{
				anyhow::bail!("RPC unavailable");
			}
			let assets = match self.prices.lock().unwrap().get(&call.shares) {
				Some(assets) => *assets,
				None => {
					let (num, den) = self.rates.lock().unwrap().get(&to).copied().unwrap_or((1, 1));
					call.shares * U256::from(num) / U256::from(den)
				},
			};
			return Ok(Erc4626::convertToAssetsCall::abi_encode_returns(&assets).into());
		}
		if self.vault_reads_fail.load(Ordering::SeqCst) {
			anyhow::bail!("vault RPC unavailable");
		}
		if selector == Erc4626::totalAssetsCall::SELECTOR ||
			selector == Erc4626::totalSupplyCall::SELECTOR
		{
			return Ok(Erc4626::totalAssetsCall::abi_encode_returns(&U256::from(1_000_000)).into());
		}
		if selector == Erc4626::decimalsCall::SELECTOR {
			return Ok(Erc4626::decimalsCall::abi_encode_returns(&6u8).into());
		}
		anyhow::bail!("unexpected call to {to}")
	}
}

#[async_trait]
impl Reads for Model {
	async fn call(&self, to: Address, data: Bytes, _block: u64) -> anyhow::Result<Bytes> {
		self.eth_calls.lock().unwrap().push(to);
		if to != MULTICALL3_ADDRESS && data.starts_with(&Erc4626::balanceOfCall::SELECTOR) {
			let call = Erc4626::balanceOfCall::abi_decode(&data)?;
			if self.failing_holders.lock().unwrap().contains(&call.account) {
				anyhow::bail!("RPC unavailable");
			}
		}
		self.answer(to, &data)
	}

	async fn code(&self, address: Address, _block: u64) -> anyhow::Result<Bytes> {
		if address == MULTICALL3_ADDRESS {
			return Ok(if self.multicall_deployed.load(Ordering::SeqCst) {
				"0x6080604052".parse().unwrap()
			} else {
				Bytes::new()
			});
		}
		self.code_reads.fetch_add(1, Ordering::SeqCst);
		if self
			.code_failures
			.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
			.is_ok()
		{
			anyhow::bail!("rpc down");
		}
		let code = self.code.lock().unwrap().get(&address).cloned();
		Ok(code.unwrap_or_else(|| self.default_code.lock().unwrap().clone()))
	}

	async fn logs(&self, _filter: &Filter) -> anyhow::Result<Vec<Log>> {
		if self.logs_fail.load(Ordering::SeqCst) {
			anyhow::bail!("rate limited");
		}
		Ok(self.logs.lock().unwrap().clone())
	}
}

/** A Base-like chain: USDC with one vault, and cNGN with none. */
fn chain_config(id: &str, accounts: &[&str]) -> ChainConfig {
	let accounts = accounts.iter().map(|a| format!("\"{a}\"")).collect::<Vec<_>>().join(", ");
	toml::from_str(&format!(
		r#"
		kind = "evm"
		chain_id = 8453
		state_machine_id = "{id}"
		start_block = 0
		reorg_safe_distance = 0
		[contracts]
		solver_accounts = [{accounts}]
		[contracts.yield_vaults."{USDC}"]
		description = "USDC"
		vaults = ["0xC768c589647798a6EE01A91FdE98EF2ed046DBD6"]
		[contracts.yield_vaults."{CNGN}"]
		description = "cNGN"
		vaults = []
		"#
	))
	.unwrap()
}

/** One test's chain, its model, and the database transaction it never commits. */
struct Inventories {
	config: ChainConfig,
	chain: SolverChain,
	model: Model,
}

impl Inventories {
	fn new(id: &str) -> Self {
		let config = chain_config(id, &[SOLVER_ACCOUNT]);
		let chain = SolverChain::from_config(&config);
		let model = Model::new();
		model.set_rate(VAULT, 2, 1);
		*model.default_code.lock().unwrap() = delegated_to(SOLVER_ACCOUNT);
		Self { config, chain, model }
	}

	fn id(&self) -> &str {
		&self.chain.id
	}

	async fn fill(&self, db: &Transaction<'_>, block: u64, solver: &str) {
		discover_solver_from_fill(db, &self.config, solver, "0xFILL", block, T0)
			.await
			.unwrap();
	}

	async fn block(&self, db: &Transaction<'_>, block: u64, timestamp: u64) {
		index_block(db, &self.chain, &self.model, block, timestamp).await.unwrap();
	}

	/** Discovers SOLVER by a fill at block 100 and seeds it at block 101. */
	async fn track(&self, db: &Transaction<'_>, wallet: u128, shares: u128) {
		self.model.set_balance(USDC, SOLVER, wallet);
		self.model.set_balance(VAULT, SOLVER, shares);
		self.fill(db, 100, SOLVER).await;
		self.block(db, 101, T0).await;
	}

	fn transfer(&self, from: &str, to: &str, value: i128, block: u64, log_index: u64) -> Transfer {
		Transfer {
			contract: USDC.into(),
			from: from.into(),
			to: to.into(),
			value: int(value),
			block_number: block,
			log_index,
			timestamp: T0,
		}
	}

	async fn send(&self, db: &Transaction<'_>, transfer: Transfer) {
		apply_token_transfer(db, &self.chain, &transfer).await.unwrap();
	}

	async fn shares(&self, db: &Transaction<'_>, from: &str, to: &str, value: i128, block: u64) {
		let transfer =
			Transfer { contract: VAULT.into(), ..self.transfer(from, to, value, block, 0) };
		apply_vault_share_transfer(db, &self.chain, &self.model, &transfer)
			.await
			.unwrap();
	}

	async fn inventory(
		&self,
		db: &Transaction<'_>,
		token: &str,
		solver: &str,
	) -> Option<Inventory> {
		load_inventory(db, &format!("{}-{token}-{solver}", self.id())).await.unwrap()
	}

	async fn usdc(&self, db: &Transaction<'_>) -> Inventory {
		self.inventory(db, USDC, SOLVER).await.unwrap()
	}

	async fn status(&self, db: &Transaction<'_>, solver: &str) -> Option<String> {
		text(
			db,
			&format!(
				"SELECT status::text || ' ' || coalesce(genesis_block::text, '-') FROM app.tracked_solvers
				 WHERE id = '{}-{solver}'",
				self.id()
			),
		)
		.await
	}

	async fn vault_shares(&self, db: &Transaction<'_>, solver: &str) -> Option<String> {
		text(
			db,
			&format!(
				"SELECT shares || ' ' || assets || ' ' || token_address FROM app.solver_vault_shares
				 WHERE id = '{}-{VAULT}-{solver}'",
				self.id()
			),
		)
		.await
	}

	async fn head(&self, db: &Transaction<'_>) -> Option<String> {
		text(
			db,
			&format!(
				"SELECT block_number || ' ' || refresh_offset FROM app.solver_inventory_heads WHERE id = '{}'",
				self.id()
			),
		)
		.await
	}

	async fn queue_watchlist(&self, db: &Transaction<'_>, version: i32, solvers: &[&str]) {
		db.execute(
			"INSERT INTO app.solver_watchlists (id, chain, version, request_count, block_number, updated_at)
			 VALUES ($1, $1, $2, $2, 1, '1970-01-01')
			 ON CONFLICT (id) DO UPDATE SET version = EXCLUDED.version",
			&[&self.id(), &version],
		)
		.await
		.unwrap();
		for solver in solvers {
			db.execute(
				"INSERT INTO app.solver_discovery_requests (id, chain, solver, version, block_number, requested_at)
				 VALUES ($1 || '-' || $2, $1, $2, $3, 1, '1970-01-01')",
				&[&self.id(), solver, &version],
			)
			.await
			.unwrap();
		}
	}

	/** Backdates a solver's reconcile clock. */
	async fn reconciled_at(&self, db: &Transaction<'_>, solver: &str, at: u64) {
		db.execute(
			"UPDATE app.tracked_solvers SET reconciled_at = $2 WHERE id = $1",
			&[&format!("{}-{solver}", self.id()), &timestamp_secs(at)],
		)
		.await
		.unwrap();
	}
}

fn solvers(base: u32, count: u32) -> Vec<String> {
	(0..count).map(|i| format!("0x{:040x}", base + i)).collect()
}

// ─── Discovery ──────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_fill_queues_its_filler_once_without_reading_the_chain() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T01");
	t.fill(&db, 100, &SOLVER.to_uppercase().replace("0X", "0x")).await;
	t.fill(&db, 100, SOLVER).await;
	let row = text(
		&db,
		"SELECT count(*) || ' ' || min(status::text) || ' ' || min(discovered_by::text) || ' ' ||
		   min(discovered_by_fill) || ' ' || min(discovery_block)
		 FROM app.tracked_solvers WHERE chain = 'EVM-T01'",
	)
	.await;
	assert_eq!(row.as_deref(), Some("1 PENDING FILL 0xfill 100"));
	assert_eq!(t.model.eth_call_count(), 0);
	assert_eq!(t.model.code_reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn watchlist_requests_become_tracked_solvers_one_version_at_a_time() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T02");
	t.queue_watchlist(&db, 1, &[SOLVER]).await;
	t.block(&db, 101, T0).await;
	assert_eq!(t.status(&db, SOLVER).await.as_deref(), Some("TRACKED 101"));
	let by = text(&db, &format!("SELECT discovered_by::text || ' ' || discovery_block FROM app.tracked_solvers WHERE id = 'EVM-T02-{SOLVER}'")).await;
	assert_eq!(by.as_deref(), Some("WATCHLIST 101"));
	let cursor =
		text(&db, "SELECT version::text FROM app.solver_watchlist_cursors WHERE id = 'EVM-T02'")
			.await;
	assert_eq!(cursor.as_deref(), Some("1"));

	t.queue_watchlist(&db, 2, &[OTHER]).await;
	// A request of a version the watchlist row doesn't name yet waits for it.
	db.execute(
		"INSERT INTO app.solver_discovery_requests (id, chain, solver, version, block_number, requested_at)
		 VALUES ('EVM-T02-' || $1, 'EVM-T02', $1, 3, 1, '1970-01-01')",
		&[&THIRD],
	)
	.await
	.unwrap();
	t.block(&db, 102, T0 + 2).await;
	assert_eq!(t.status(&db, OTHER).await.as_deref(), Some("TRACKED 102"));
	assert_eq!(t.status(&db, THIRD).await, None);
	let cursor =
		text(&db, "SELECT version::text FROM app.solver_watchlist_cursors WHERE id = 'EVM-T02'")
			.await;
	assert_eq!(cursor.as_deref(), Some("2"));
}

#[tokio::test]
async fn a_watchlist_version_waits_until_the_chain_reaches_its_time() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T23");
	t.queue_watchlist(&db, 1, &[SOLVER]).await;
	// A poll at the head wrote it, while this chain was still in its history.
	db.execute(
		"UPDATE app.solver_watchlists SET updated_at = $2 WHERE id = $1",
		&[&t.id(), &timestamp_secs(T0 + 60)],
	)
	.await
	.unwrap();
	t.block(&db, 101, T0).await;
	assert_eq!(t.status(&db, SOLVER).await, None);
	t.block(&db, 102, T0 + 60).await;
	assert_eq!(t.status(&db, SOLVER).await.as_deref(), Some("TRACKED 102"));
}

// ─── Genesis ────────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn genesis_reads_every_token_vault_and_the_delegation_pinned_to_the_block() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T03");
	t.track(&db, 1_000, 50).await;

	let usdc = t.usdc(&db).await;
	assert_eq!(
		(usdc.wallet, usdc.vault_shares, usdc.vaults, usdc.balance),
		(int(1_000), int(50), int(100), int(1_100))
	);
	assert_eq!((usdc.genesis_block, usdc.last_read_block, usdc.block_number), (101, 101, 101));
	assert_eq!(usdc.last_log_index, AFTER_EVERY_LOG);
	assert_eq!(usdc.discovered_by, SolverDiscoveryTrigger::Fill);
	// A token the solver holds none of is still a row: zero is an answer.
	let cngn = t.inventory(&db, CNGN, SOLVER).await.unwrap();
	assert_eq!((cngn.wallet, cngn.vaults, cngn.balance), (I256::ZERO, I256::ZERO, I256::ZERO));
	assert_eq!(t.vault_shares(&db, SOLVER).await, Some(format!("50 100 {USDC}")));
	let delegation = text(
		&db,
		&format!("SELECT delegated || ' ' || delegate || ' ' || block_number FROM app.solver_delegations WHERE id = 'EVM-T03-{SOLVER}'"),
	)
	.await;
	assert_eq!(delegation, Some(format!("true {SOLVER_ACCOUNT} 101")));
	assert_eq!(t.status(&db, SOLVER).await.as_deref(), Some("TRACKED 101"));
}

#[tokio::test]
async fn an_undelegated_solver_is_not_checked_again_on_the_next_block() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T04");
	*t.model.default_code.lock().unwrap() = Bytes::new();
	t.track(&db, 1_000, 0).await;
	let delegated = text(
		&db,
		&format!(
			"SELECT delegated::text FROM app.solver_delegations WHERE id = 'EVM-T04-{SOLVER}'"
		),
	)
	.await;
	assert_eq!(delegated.as_deref(), Some("false"));
	t.block(&db, 102, T0 + 2).await;
	assert_eq!(t.model.code_reads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_failed_read_leaves_the_solver_pending_and_a_later_block_reads_again() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T05");
	t.model.code_failures.store(1, Ordering::SeqCst);
	t.model.set_balance(USDC, SOLVER, 1_000);
	t.fill(&db, 100, SOLVER).await;

	t.block(&db, 101, T0).await;
	assert_eq!(t.status(&db, SOLVER).await.as_deref(), Some("PENDING -"));
	assert!(t.inventory(&db, USDC, SOLVER).await.is_none());
	let delegations =
		text(&db, "SELECT count(*)::text FROM app.solver_delegations WHERE chain = 'EVM-T05'")
			.await;
	assert_eq!(delegations.as_deref(), Some("0"));

	t.block(&db, 102, T0 + 2).await;
	assert_eq!(t.status(&db, SOLVER).await.as_deref(), Some("TRACKED 102"));
	assert_eq!(t.usdc(&db).await.wallet, int(1_000));
}

#[tokio::test]
async fn the_discovering_fills_block_is_counted_once() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T06");
	t.fill(&db, 100, SOLVER).await;
	// The solver is pending, so a transfer in its fill block is not applied…
	t.send(&db, t.transfer(OTHER, SOLVER, 40, 100, 7)).await;
	assert!(t.inventory(&db, USDC, SOLVER).await.is_none());

	// …because the genesis read after the batch already holds it.
	t.model.set_balance(USDC, SOLVER, 1_040);
	t.block(&db, 101, T0 + 2).await;
	assert_eq!(t.usdc(&db).await.wallet, int(1_040));

	t.send(&db, Transfer { timestamp: T0 + 2, ..t.transfer(OTHER, SOLVER, 25, 101, 3) })
		.await;
	assert_eq!(t.usdc(&db).await.wallet, int(1_040));

	t.send(&db, Transfer { timestamp: T0 + 4, ..t.transfer(OTHER, SOLVER, 25, 102, 0) })
		.await;
	let usdc = t.usdc(&db).await;
	assert_eq!(
		(usdc.wallet, usdc.balance, usdc.block_number, usdc.last_log_index),
		(int(1_065), int(1_065), 102, 0)
	);
}

#[tokio::test]
async fn a_burst_of_solvers_is_read_in_two_multicalls_plus_one_get_code_each() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T07");
	let burst = solvers(0xa000, 20);
	for (i, solver) in burst.iter().enumerate() {
		t.model.set_balance(USDC, solver, 1_000 + i as u128);
		t.model.set_balance(VAULT, solver, i as u128);
	}
	let refs: Vec<&str> = burst.iter().map(String::as_str).collect();
	t.queue_watchlist(&db, 1, &refs).await;

	t.block(&db, 101, T0).await;

	let calls = t.model.eth_calls.lock().unwrap().clone();
	assert_eq!(calls.len(), 2);
	assert!(calls.iter().all(|to| *to == MULTICALL3_ADDRESS));
	assert_eq!(t.model.code_reads.load(Ordering::SeqCst), 20);
	for (i, solver) in burst.iter().enumerate() {
		assert_eq!(t.status(&db, solver).await.as_deref(), Some("TRACKED 101"));
		let row = t.inventory(&db, USDC, solver).await.unwrap();
		assert_eq!(
			(row.wallet, row.vault_shares, row.vaults),
			(int(1_000 + i as i128), int(i as i128), int(2 * i as i128))
		);
	}
}

#[tokio::test]
async fn a_read_that_fails_for_one_solver_leaves_only_that_solver_pending() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T08");
	t.model.set_balance(USDC, SOLVER, 1_000);
	t.model.set_balance(USDC, OTHER, 700);
	t.model.reverting.lock().unwrap().insert(addr(SOLVER));
	t.fill(&db, 100, SOLVER).await;
	t.fill(&db, 100, OTHER).await;

	t.block(&db, 101, T0).await;
	assert_eq!(t.status(&db, SOLVER).await.as_deref(), Some("PENDING -"));
	assert!(t.inventory(&db, USDC, SOLVER).await.is_none());
	assert_eq!(t.status(&db, OTHER).await.as_deref(), Some("TRACKED 101"));
	assert_eq!(t.inventory(&db, USDC, OTHER).await.unwrap().wallet, int(700));

	t.model.reverting.lock().unwrap().clear();
	t.block(&db, 102, T0 + 2).await;
	assert_eq!(t.status(&db, SOLVER).await.as_deref(), Some("TRACKED 102"));
	assert_eq!(t.usdc(&db).await.wallet, int(1_000));
}

#[tokio::test]
async fn a_chain_without_multicall3_makes_the_same_reads_as_single_calls() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T09");
	t.model.multicall_deployed.store(false, Ordering::SeqCst);
	t.track(&db, 1_000, 50).await;
	let usdc = t.usdc(&db).await;
	assert_eq!(
		(usdc.wallet, usdc.vault_shares, usdc.vaults, usdc.balance),
		(int(1_000), int(50), int(100), int(1_100))
	);
	// USDC's, its vault's and cNGN's balanceOf, then the vault shares' convertToAssets.
	let calls = t.model.eth_calls.lock().unwrap().clone();
	assert_eq!(calls.len(), 4);
	assert!(!calls.contains(&MULTICALL3_ADDRESS));
}

// ─── Events ─────────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn transfers_between_untracked_addresses_read_nothing_on_chain() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T10");
	t.track(&db, 1_000, 0).await;
	t.model.reset_counts();
	for i in 0..200u32 {
		let from = format!("0x{:040x}", i + 1);
		let to = format!("0x{:040x}", i + 2);
		t.send(&db, t.transfer(&from, &to, 10, 200 + i as u64, 0)).await;
		t.shares(&db, &from, &to, 10, 200 + i as u64).await;
	}
	assert_eq!(t.model.eth_call_count(), 0);
	assert_eq!(t.model.code_reads.load(Ordering::SeqCst), 0);
	assert_eq!(t.usdc(&db).await.wallet, int(1_000));
}

#[tokio::test]
async fn transfers_move_wallet_and_balance_and_observed_at_never_goes_back() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T11");
	t.track(&db, 1_000, 10).await;
	t.send(&db, Transfer { timestamp: T0 + 10, ..t.transfer(OTHER, SOLVER, 300, 102, 0) })
		.await;
	// The TypeScript sent the second log at an earlier time; on one stream block time never goes
	// back, and a write before the open version is refused (see the history test).
	t.send(&db, Transfer { timestamp: T0 + 10, ..t.transfer(SOLVER, OTHER, 100, 103, 0) })
		.await;
	let usdc = t.usdc(&db).await;
	assert_eq!(
		(usdc.wallet, usdc.vaults, usdc.balance, usdc.block_number),
		(int(1_200), int(20), int(1_220), 103)
	);
	assert_eq!(usdc.observed_at, timestamp_secs(T0 + 10));
}

#[tokio::test]
async fn a_wallet_that_would_go_negative_is_clamped_to_zero() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T12");
	t.track(&db, 100, 0).await;
	t.send(&db, t.transfer(SOLVER, OTHER, 250, 102, 0)).await;
	let usdc = t.usdc(&db).await;
	assert_eq!((usdc.wallet, usdc.balance), (I256::ZERO, I256::ZERO));
}

#[tokio::test]
async fn replaying_an_applied_log_changes_nothing() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T13");
	t.track(&db, 1_000, 0).await;
	t.send(&db, t.transfer(OTHER, SOLVER, 5, 102, 4)).await;
	t.send(&db, t.transfer(OTHER, SOLVER, 5, 102, 4)).await;
	t.send(&db, t.transfer(OTHER, SOLVER, 5, 102, 2)).await;
	assert_eq!(t.usdc(&db).await.wallet, int(1_005));
}

#[tokio::test]
async fn vault_mints_transfers_and_burns_move_shares_valued_in_assets() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T14");
	t.track(&db, 1_000, 0).await;

	t.shares(&db, ZERO, SOLVER, 100, 102).await;
	assert_eq!(t.vault_shares(&db, SOLVER).await, Some(format!("100 200 {USDC}")));
	let usdc = t.usdc(&db).await;
	assert_eq!((usdc.vault_shares, usdc.vaults, usdc.balance), (int(100), int(200), int(1_200)));

	t.shares(&db, SOLVER, OTHER, 30, 103).await;
	t.shares(&db, SOLVER, ZERO, 10, 104).await;
	let usdc = t.usdc(&db).await;
	assert_eq!((usdc.vault_shares, usdc.vaults, usdc.balance), (int(60), int(120), int(1_120)));
	assert_eq!(t.vault_shares(&db, OTHER).await, None);
}

// ─── Head and refresh ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn the_head_advances_at_most_every_30_seconds_of_block_time() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T15");
	t.block(&db, 101, T0).await;
	assert_eq!(t.head(&db).await.as_deref(), Some("101 0"));
	t.block(&db, 102, T0 + 10).await;
	assert_eq!(t.head(&db).await.as_deref(), Some("101 0"));
	t.block(&db, 103, T0 + 31).await;
	assert_eq!(t.head(&db).await.as_deref(), Some("103 0"));
	let observed =
		text(&db, "SELECT observed_at::text FROM app.solver_inventory_heads WHERE id = 'EVM-T15'")
			.await;
	assert_eq!(observed, Some(timestamp_secs(T0 + 31).to_string()));
}

#[tokio::test]
async fn vault_shares_are_revalued_hourly_without_reading_wallets_or_code() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T16");
	t.track(&db, 1_000, 50).await;
	t.model.set_rate(VAULT, 3, 1);
	t.model.reset_counts();

	let at = T0 + REVALUE_INTERVAL_SECS as u64 + 5;
	t.block(&db, 500, at).await;

	let usdc = t.usdc(&db).await;
	assert_eq!(
		(usdc.wallet, usdc.vault_shares, usdc.vaults, usdc.balance),
		(int(1_000), int(50), int(150), int(1_150))
	);
	assert_eq!(usdc.refreshed_at, timestamp_secs(at));
	assert_eq!(t.model.balance_reads.load(Ordering::SeqCst), 0);
	assert_eq!(t.model.code_reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn reconciliation_re_anchors_to_a_pinned_read_and_records_drift() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T17");
	t.track(&db, 1_000, 50).await;
	// A Transfer never seen, and shares that moved without one.
	t.model.set_balance(USDC, SOLVER, 900);
	t.model.set_balance(VAULT, SOLVER, 55);
	*t.model.default_code.lock().unwrap() = Bytes::new();
	let at = T0 + RECONCILE_INTERVAL_SECS as u64 + 60;

	t.block(&db, 9_000, at).await;

	let usdc = t.usdc(&db).await;
	assert_eq!(
		(usdc.wallet, usdc.vault_shares, usdc.vaults, usdc.balance),
		(int(900), int(55), int(110), int(1_010))
	);
	assert_eq!((usdc.last_read_block, usdc.block_number, usdc.genesis_block), (9_000, 9_000, 101));
	assert_eq!(usdc.last_log_index, AFTER_EVERY_LOG);
	assert_eq!(usdc.last_reconciled_drift, Some(int(-100 + 10)));
	assert_eq!(usdc.last_reconciled_at, Some(timestamp_secs(at)));
	let delegation = text(&db, &format!("SELECT delegated || ' ' || block_number FROM app.solver_delegations WHERE id = 'EVM-T17-{SOLVER}'")).await;
	assert_eq!(delegation.as_deref(), Some("false 9000"));

	// The read at 9000 already holds that block's logs.
	t.send(&db, t.transfer(OTHER, SOLVER, 50, 9_000, 1)).await;
	assert_eq!(t.usdc(&db).await.wallet, int(900));
}

#[tokio::test]
async fn a_page_of_due_solvers_is_reconciled_and_revalued_in_three_eth_calls() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T18");
	let page = solvers(0xb000, 6);
	for solver in &page {
		t.model.set_balance(USDC, solver, 1_000);
		t.model.set_balance(VAULT, solver, 50);
		t.fill(&db, 100, solver).await;
	}
	t.block(&db, 101, T0).await;
	let at = T0 + RECONCILE_INTERVAL_SECS as u64 + 60;
	// Half the page was reconciled recently, so it is only due a revaluation.
	for solver in &page[..3] {
		t.reconciled_at(&db, solver, at - 60).await;
	}
	t.model.set_rate(VAULT, 3, 1);
	t.model.reset_counts();

	t.block(&db, 9_000, at).await;

	assert_eq!(t.model.eth_call_count(), 3);
	assert_eq!(t.model.code_reads.load(Ordering::SeqCst), 3);
	for solver in &page {
		let row = t.inventory(&db, USDC, solver).await.unwrap();
		assert_eq!((row.wallet, row.vaults, row.balance), (int(1_000), int(150), int(1_150)));
	}
	assert_eq!(t.inventory(&db, USDC, &page[0]).await.unwrap().last_read_block, 101);
	assert_eq!(t.inventory(&db, USDC, &page[5]).await.unwrap().last_read_block, 9_000);
}

#[tokio::test]
async fn a_solver_whose_read_keeps_failing_does_not_pin_the_page() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T19");
	let all = solvers(0xc000, 26);
	for solver in &all {
		t.model.set_balance(USDC, solver, 1_000);
		t.fill(&db, 100, solver).await;
	}
	// Genesis runs 20 solvers a block.
	t.block(&db, 101, T0).await;
	t.block(&db, 102, T0 + 2).await;
	let pending = text(&db, "SELECT count(*)::text FROM app.tracked_solvers WHERE chain = 'EVM-T19' AND status = 'PENDING'").await;
	assert_eq!(pending.as_deref(), Some("0"));
	for solver in &all {
		t.model.set_balance(USDC, solver, 900);
	}
	t.model.reverting.lock().unwrap().insert(addr(&all[0]));
	let at = T0 + RECONCILE_INTERVAL_SECS as u64 + 60;

	t.block(&db, 9_000, at).await;
	assert_eq!(t.head(&db).await.as_deref(), Some("9000 25"));
	assert_eq!(t.inventory(&db, USDC, &all[0]).await.unwrap().wallet, int(1_000));
	assert_eq!(t.inventory(&db, USDC, &all[1]).await.unwrap().wallet, int(900));

	t.block(&db, 9_100, at + 31).await;
	assert_eq!(t.inventory(&db, USDC, &all[25]).await.unwrap().wallet, int(900));
	assert_eq!(t.head(&db).await.as_deref(), Some("9100 0"));

	// Still due, the failed solver is read again when the cycle comes round.
	t.model.reverting.lock().unwrap().clear();
	t.block(&db, 9_200, at + 62).await;
	assert_eq!(t.inventory(&db, USDC, &all[0]).await.unwrap().wallet, int(900));
}

#[tokio::test]
async fn a_failed_refresh_read_writes_nothing_and_resumes_next_time() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T20");
	t.track(&db, 1_000, 50).await;
	t.model.set_balance(USDC, SOLVER, 900);
	t.model.code_failures.store(1, Ordering::SeqCst);
	let at = T0 + RECONCILE_INTERVAL_SECS as u64 + 60;

	t.block(&db, 9_000, at).await;
	let usdc = t.usdc(&db).await;
	assert_eq!((usdc.wallet, usdc.last_read_block), (int(1_000), 101));
	assert_eq!(t.head(&db).await.as_deref(), Some("9000 0"));

	t.block(&db, 9_020, at + 40).await;
	let usdc = t.usdc(&db).await;
	assert_eq!((usdc.wallet, usdc.last_read_block), (int(900), 9_020));
}

#[tokio::test]
async fn a_failed_multicall_leaves_genesis_for_a_later_block() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T21");
	t.model.set_balance(USDC, SOLVER, 1_000);
	t.fill(&db, 100, SOLVER).await;
	t.fill(&db, 100, OTHER).await;
	// Two solvers make a batch for Multicall3; a failing `aggregate3` eth_call fails the batch.
	struct FailingMulticall<'a>(&'a Model);
	#[async_trait]
	impl Reads for FailingMulticall<'_> {
		async fn call(&self, to: Address, data: Bytes, block: u64) -> anyhow::Result<Bytes> {
			anyhow::ensure!(to != MULTICALL3_ADDRESS, "aggregate3 failed");
			self.0.call(to, data, block).await
		}
		async fn code(&self, address: Address, block: u64) -> anyhow::Result<Bytes> {
			self.0.code(address, block).await
		}
		async fn logs(&self, filter: &Filter) -> anyhow::Result<Vec<Log>> {
			self.0.logs(filter).await
		}
	}
	index_block(&db, &t.chain, &FailingMulticall(&t.model), 101, T0).await.unwrap();
	assert_eq!(t.status(&db, SOLVER).await.as_deref(), Some("PENDING -"));
	assert_eq!(t.status(&db, OTHER).await.as_deref(), Some("PENDING -"));
	t.block(&db, 102, T0 + 2).await;
	assert_eq!(t.status(&db, SOLVER).await.as_deref(), Some("TRACKED 102"));
}

// ─── solver_inventories history ─────────────────────────────────────────────────────────────────

/** omnihedron's `blockHeight` lookup: the version whose range contains `height` (ms). */
async fn at_height(db: &Transaction<'_>, id: &str, height: i64) -> Option<String> {
	db.query_opt(
		"SELECT wallet::text FROM app.solver_inventories
		 WHERE id = $1 AND _block_range @> $2::int8 ORDER BY lower(_block_range) DESC LIMIT 1",
		&[&id, &height],
	)
	.await
	.unwrap()
	.map(|row| row.get(0))
}

#[tokio::test]
async fn solver_inventories_keep_one_version_per_block_time() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let t = Inventories::new("EVM-T22");
	t.track(&db, 1_000, 0).await; // T0
	t.send(&db, Transfer { timestamp: T0 + 12, ..t.transfer(OTHER, SOLVER, 100, 102, 0) })
		.await;
	// Two logs in one second: one version, replaced in place.
	t.send(&db, Transfer { timestamp: T0 + 24, ..t.transfer(OTHER, SOLVER, 10, 103, 0) })
		.await;
	t.send(&db, Transfer { timestamp: T0 + 24, ..t.transfer(OTHER, SOLVER, 5, 104, 0) })
		.await;

	let id = format!("EVM-T22-{USDC}-{SOLVER}");
	let ms = |secs: u64| (secs * 1000) as i64;
	let ranges = db
		.query(
			"SELECT _block_range::text, wallet::text FROM app.solver_inventories WHERE id = $1
			 ORDER BY lower(_block_range)",
			&[&id],
		)
		.await
		.unwrap()
		.iter()
		.map(|row| format!("{} {}", row.get::<_, String>(0), row.get::<_, String>(1)))
		.collect::<Vec<_>>();
	assert_eq!(
		ranges,
		vec![
			format!("[{},{}) 1000", ms(T0), ms(T0 + 12)),
			format!("[{},{}) 1100", ms(T0 + 12), ms(T0 + 24)),
			format!("[{},) 1115", ms(T0 + 24)),
		]
	);
	assert_eq!(at_height(&db, &id, ms(T0) - 1).await, None);
	assert_eq!(at_height(&db, &id, ms(T0)).await.as_deref(), Some("1000"));
	assert_eq!(at_height(&db, &id, ms(T0 + 12) - 1).await.as_deref(), Some("1000"));
	assert_eq!(at_height(&db, &id, ms(T0 + 12)).await.as_deref(), Some("1100"));
	assert_eq!(at_height(&db, &id, ms(T0 + 20)).await.as_deref(), Some("1100"));
	assert_eq!(at_height(&db, &id, i64::MAX).await.as_deref(), Some("1115"));

	// simplex's query: every token of one solver as of a block time.
	let simplex = db
		.query(
			"SELECT token_address, balance::text FROM app.solver_inventories
			 WHERE solver = $1 AND chain = 'EVM-T22' AND _block_range @> $2::int8 ORDER BY token_address",
			&[&SOLVER, &ms(T0 + 13)],
		)
		.await
		.unwrap()
		.iter()
		.map(|row| format!("{} {}", row.get::<_, String>(0), row.get::<_, String>(1)))
		.collect::<Vec<_>>();
	assert_eq!(simplex, vec![format!("{CNGN} 0"), format!("{USDC} 1100")]);

	// A write earlier than the open version is an ordering bug.
	let mut row = t.usdc(&db).await;
	row.wallet = int(1);
	let err = save_inventory(&db, &row, timestamp_secs(T0 + 1)).await.unwrap_err();
	assert!(err.to_string().contains("precedes"), "{err}");
}

// ─── Watchlist ──────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn watchlist_requests_are_queued_as_one_version_per_chain() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let at = timestamp_secs(T0);
	let [a, b, c] = ["a", "b", "c"].map(|x| format!("0x{}", x.repeat(40)));
	queue_requests(&db, "EVM-W01", &[a.clone(), b.clone()], 1_000, at)
		.await
		.unwrap();
	queue_requests(&db, "EVM-W02", &[a.clone()], 1_000, at).await.unwrap();
	let watchlist = |chain: &str| {
		format!("SELECT version || ' ' || request_count || ' ' || block_number FROM app.solver_watchlists WHERE id = '{chain}'")
	};
	assert_eq!(text(&db, &watchlist("EVM-W01")).await.as_deref(), Some("1 2 1000"));

	queue_requests(&db, "EVM-W01", &[a.clone(), b.clone(), c.clone()], 1_001, at)
		.await
		.unwrap();
	queue_requests(&db, "EVM-W02", &[a.clone()], 1_001, at).await.unwrap();
	let requests = db
		.query(
			"SELECT solver, version FROM app.solver_discovery_requests WHERE chain = 'EVM-W01' ORDER BY solver",
			&[],
		)
		.await
		.unwrap()
		.iter()
		.map(|row| (row.get::<_, String>(0), row.get::<_, i32>(1)))
		.collect::<Vec<_>>();
	assert_eq!(requests, vec![(a, 1), (b, 1), (c, 2)]);
	assert_eq!(text(&db, &watchlist("EVM-W01")).await.as_deref(), Some("2 3 1001"));
	// Nothing new for this chain, so its version stands.
	assert_eq!(text(&db, &watchlist("EVM-W02")).await.as_deref(), Some("1 1 1000"));
}

/** A one-connection-at-a-time HTTP server answering each request with the next canned reply. */
async fn orderbook(
	replies: Vec<(u16, Option<&'static str>, String)>,
) -> (reqwest::Url, std::sync::Arc<Mutex<Vec<String>>>) {
	use tokio::io::{AsyncReadExt, AsyncWriteExt};
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let url = format!("http://{}/solvers", listener.local_addr().unwrap()).parse().unwrap();
	let seen = std::sync::Arc::new(Mutex::new(vec![]));
	let requests = seen.clone();
	tokio::spawn(async move {
		for (status, etag, body) in replies {
			let (mut socket, _) = listener.accept().await.unwrap();
			let mut buf = vec![0u8; 8192];
			let n = socket.read(&mut buf).await.unwrap();
			requests.lock().unwrap().push(String::from_utf8_lossy(&buf[..n]).to_lowercase());
			let etag = etag.map(|e| format!("etag: {e}\r\n")).unwrap_or_default();
			let reply = format!(
				"HTTP/1.1 {status} X\r\ncontent-type: application/json\r\n{etag}content-length: {}\r\nconnection: close\r\n\r\n{body}",
				body.len()
			);
			socket.write_all(reply.as_bytes()).await.unwrap();
		}
	});
	(url, seen)
}

#[tokio::test]
async fn the_poll_sends_its_etag_treats_304_as_unchanged_and_survives_failures() {
	let Some(mut client) = database().await else { return };
	let chain = format!("EVM-W{}", std::process::id());
	let a = format!("0x{}", "a".repeat(40));
	let body = format!(r#"{{"chains":[{{"chain":"{chain}","solvers":[{{"address":"{a}"}}]}}]}}"#);
	let bad_chain = format!("{chain}\\u0000");
	let unwritable =
		format!(r#"{{"chains":[{{"chain":"{bad_chain}","solvers":[{{"address":"{a}"}}]}}]}}"#);
	let (url, seen) = orderbook(vec![
		(503, None, "{}".into()),
		(200, Some("\"v0\""), unwritable),
		(200, Some("\"v1\""), body),
		(304, None, String::new()),
	])
	.await;
	let http = reqwest::Client::new();
	let tracks = |c: &str| c.starts_with(&chain);
	let mut poller = Poller::default();

	// A 503 is logged, and the next poll retries.
	poller.poll(&mut client, &http, &url, tracks, (1_000, T0)).await.unwrap();
	// A response it fails to write keeps no ETag, so the next poll fetches it in full.
	assert!(poller.poll(&mut client, &http, &url, tracks, (1_001, T0)).await.is_err());
	assert_eq!(poller.etag, None);
	poller.poll(&mut client, &http, &url, tracks, (1_002, T0)).await.unwrap();
	assert_eq!(poller.etag.as_deref(), Some("\"v1\""));
	poller.poll(&mut client, &http, &url, tracks, (1_003, T0)).await.unwrap();

	let seen = seen.lock().unwrap().clone();
	assert!(!seen[2].contains("if-none-match"));
	assert!(seen[3].contains("if-none-match: \"v1\""));
	assert!(seen[3].contains("accept: application/json"));
	let written = client
		.query_one(
			"SELECT (SELECT version || ' ' || block_number FROM app.solver_watchlists WHERE id = $1),
			   (SELECT count(*) FROM app.solver_discovery_requests WHERE chain = $1)",
			&[&chain],
		)
		.await
		.unwrap();
	assert_eq!(written.get::<_, Option<String>>(0).as_deref(), Some("1 1002"));
	assert_eq!(written.get::<_, i64>(1), 1);
	client
		.execute("DELETE FROM app.solver_discovery_requests WHERE chain = $1", &[&chain])
		.await
		.unwrap();
	client
		.execute("DELETE FROM app.solver_watchlists WHERE id = $1", &[&chain])
		.await
		.unwrap();

	// An unreachable orderbook is logged, never an error.
	let closed: reqwest::Url = "http://127.0.0.1:9/solvers".parse().unwrap();
	poller.poll(&mut client, &http, &closed, tracks, (1_004, T0)).await.unwrap();
}

// ─── Yield vaults ───────────────────────────────────────────────────────────────────────────────

/** The Base SolverAccount the TypeScript tests delegate to. */
const BASE_ACCOUNT: &str = "0xd5535d4deb17f050e52b6efda2fde00435f39279";

struct Vaults {
	chain: SolverChain,
	model: Model,
}

impl Vaults {
	fn new(id: &str) -> Self {
		let chain = SolverChain::from_config(&chain_config(id, &[BASE_ACCOUNT]));
		let model = Model::new();
		model.code.lock().unwrap().insert(addr(SOLVER), delegated_to(BASE_ACCOUNT));
		Self { chain, model }
	}

	fn id(&self) -> &str {
		&self.chain.id
	}

	fn ledger(&self) -> LedgerInput {
		LedgerInput {
			vault: VAULT.into(),
			lp: SOLVER.into(),
			caller: SOLVER.into(),
			receiver: None,
			assets: int(20),
			shares: int(20),
			event_type: Type::Deposit,
			block_number: 50,
			transaction_hash: tx(0xab),
			log_index: 2,
			timestamp: 86_400,
		}
	}

	fn transfer(&self) -> TransferInput {
		TransferInput {
			vault: VAULT.into(),
			from: OTHER.into(),
			to: SOLVER.into(),
			shares: int(100),
			block_number: 50,
			transaction_hash: tx(0xde),
			log_index: 3,
			timestamp: 86_400,
		}
	}

	/** The block's vault logs, as `readVaultBlockMovements` reads them. */
	fn block_events(&self, events: &[LedgerInput]) {
		let logs = events
			.iter()
			.map(|e| {
				let lp = addr(&e.lp);
				let shares = e.shares.unsigned_abs();
				let data: LogData = match e.event_type {
					Type::Deposit =>
						Erc4626::Deposit { sender: lp, owner: lp, assets: shares, shares }
							.encode_log_data(),
					Type::Withdraw => Erc4626::Withdraw {
						sender: lp,
						receiver: lp,
						owner: lp,
						assets: shares,
						shares,
					}
					.encode_log_data(),
					Type::TransferIn =>
						Erc4626::Transfer { from: addr(THIRD), to: lp, value: shares }
							.encode_log_data(),
					Type::TransferOut =>
						Erc4626::Transfer { from: lp, to: addr(THIRD), value: shares }
							.encode_log_data(),
				};
				Log {
					inner: alloy::primitives::Log { address: addr(VAULT), data },
					block_number: Some(e.block_number),
					log_index: Some(e.log_index),
					transaction_hash: Some(e.transaction_hash.parse().unwrap()),
					..Default::default()
				}
			})
			.collect();
		*self.model.logs.lock().unwrap() = logs;
	}

	/** A position that predates this indexer, as SubQuery's rows did. */
	async fn legacy(&self, db: &Transaction<'_>, lp: &str, shares: u128, deposited: u128) {
		db.execute(
			"INSERT INTO app.vault_lp_positions
			   (id, chain, vault, underlying_token, lp, shares, total_assets_deposited,
			    total_assets_withdrawn, deposit_count, withdraw_count, created_at, last_updated_at)
			 VALUES ($1 || '-' || $2 || '-' || $3, $1, $2, $4, $3, $5::text::numeric, $6::text::numeric,
			   0, 1, 0, '1970-01-01', '1970-01-01')",
			&[&self.id(), &VAULT, &lp, &USDC, &shares.to_string(), &deposited.to_string()],
		)
		.await
		.unwrap();
		self.model.set_balance(VAULT, lp, shares);
	}

	async fn record(&self, db: &Transaction<'_>, input: LedgerInput) -> anyhow::Result<()> {
		vaults::record_ledger(db, &self.chain, &self.model, &input).await
	}

	async fn record_transfer(
		&self,
		db: &Transaction<'_>,
		input: TransferInput,
	) -> anyhow::Result<()> {
		vaults::record_transfer(db, &self.chain, &self.model, &input).await
	}

	async fn snapshot(&self, db: &Transaction<'_>) {
		vaults::snapshot_chain(db, &self.chain, &self.model, 100, 86_500).await.unwrap();
	}

	async fn position(&self, db: &Transaction<'_>, lp: &str) -> Option<String> {
		text(
			db,
			&format!(
				"SELECT concat_ws(' ', shares, opening_shares, opening_principal, opening_block,
				   total_assets_deposited, total_assets_withdrawn, total_assets_transferred_in,
				   total_assets_transferred_out, deposit_count, withdraw_count)
				 FROM app.vault_lp_positions WHERE id = '{}-{VAULT}-{lp}'",
				self.id()
			),
		)
		.await
	}

	async fn snapshot_of(&self, db: &Transaction<'_>, lp: &str) -> Option<String> {
		text(
			db,
			&format!(
				"SELECT concat_ws(' ', net_principal, asset_value, yield_earned)
				 FROM app.vault_position_snapshots WHERE chain = '{}' AND lp = '{lp}'",
				self.id()
			),
		)
		.await
	}

	async fn count(&self, db: &Transaction<'_>, table: &str) -> i64 {
		db.query_one(&format!("SELECT count(*) FROM app.{table} WHERE chain = $1"), &[&self.id()])
			.await
			.unwrap()
			.get(0)
	}
}

fn tx(byte: u8) -> String {
	format!("{:#x}", B256::repeat_byte(byte))
}

fn set_prices(model: &Model, pairs: &[(u128, u128)]) {
	let mut prices = model.prices.lock().unwrap();
	for (shares, assets) in pairs {
		prices.insert(U256::from(*shares), U256::from(*assets));
	}
}

#[tokio::test]
async fn shares_received_before_delegation_seed_opening_capital() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V01");
	v.model.code.lock().unwrap().clear();
	v.record_transfer(&db, v.transfer()).await.unwrap();
	assert_eq!(v.position(&db, SOLVER).await, None);

	v.model.code.lock().unwrap().insert(addr(SOLVER), delegated_to(BASE_ACCOUNT));
	v.model.set_balance(VAULT, SOLVER, 1_020);
	v.block_events(&[v.ledger()]);
	v.record(&db, v.ledger()).await.unwrap();
	assert_eq!(v.position(&db, SOLVER).await.as_deref(), Some("1020 1000 1000 50 20 0 0 0 1 0"));

	v.model.set_rate(VAULT, 101, 100);
	v.snapshot(&db).await;
	assert_eq!(v.snapshot_of(&db, SOLVER).await.as_deref(), Some("1020 1030 10"));
}

#[tokio::test]
async fn later_same_block_movements_are_backed_out_of_the_opening_balance() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V02");
	let deposit = v.ledger();
	let incoming = LedgerInput {
		event_type: Type::TransferIn,
		shares: int(30),
		assets: int(30),
		log_index: 4,
		..v.ledger()
	};
	let withdrawal = LedgerInput {
		event_type: Type::Withdraw,
		shares: int(10),
		assets: int(10),
		log_index: 6,
		..v.ledger()
	};
	v.model.set_balance(VAULT, SOLVER, 1_040);
	v.block_events(&[deposit.clone(), incoming.clone(), withdrawal.clone()]);
	v.record(&db, deposit).await.unwrap();
	v.record(&db, incoming).await.unwrap();
	v.record(&db, withdrawal).await.unwrap();
	assert_eq!(v.position(&db, SOLVER).await.as_deref(), Some("1040 1000 1000 50 20 10 30 0 1 1"));
	v.snapshot(&db).await;
	assert_eq!(v.snapshot_of(&db, SOLVER).await.as_deref(), Some("1040 1040 0"));
}

#[tokio::test]
async fn a_first_event_that_withdraws_everything_still_opens_the_position() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V03");
	let withdrawal = LedgerInput {
		event_type: Type::Withdraw,
		assets: int(110),
		shares: int(100),
		..v.ledger()
	};
	v.model.set_rate(VAULT, 110, 100);
	v.block_events(&[withdrawal.clone()]);
	v.record(&db, withdrawal).await.unwrap();
	assert_eq!(v.position(&db, SOLVER).await.as_deref(), Some("0 100 110 50 0 110 0 0 0 1"));
	v.snapshot(&db).await;
	assert_eq!(v.snapshot_of(&db, SOLVER).await.as_deref(), Some("0 0 0"));
}

#[tokio::test]
async fn an_impossible_opening_balance_records_nothing() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V04");
	v.block_events(&[v.ledger()]);
	let err = v.record(&db, v.ledger()).await.unwrap_err();
	assert!(err.to_string().contains("Negative vault opening shares"), "{err}");
	assert_eq!(v.count(&db, "vault_ledger_events").await, 0);
	assert_eq!(v.position(&db, SOLVER).await, None);
}

#[tokio::test]
async fn transferred_principal_stays_out_of_yield_for_the_reported_base_wallet() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V05");
	v.legacy(&db, SOLVER, 0, 0).await;
	set_prices(
		&v.model,
		&[
			(174_220_559_508, 199_506_902_673),
			(57_243_722_070, 65_552_073_822),
			(231_481_948_014, 265_087_704_647),
		],
	);
	v.record_transfer(&db, TransferInput { shares: int(174_220_559_508), ..v.transfer() })
		.await
		.unwrap();
	v.record_transfer(
		&db,
		TransferInput {
			from: THIRD.into(),
			shares: int(57_243_722_070),
			log_index: 4,
			..v.transfer()
		},
	)
	.await
	.unwrap();
	v.record(
		&db,
		LedgerInput {
			assets: int(20_230_586),
			shares: int(17_666_436),
			log_index: 5,
			..v.ledger()
		},
	)
	.await
	.unwrap();
	v.model.set_balance(VAULT, SOLVER, 231_481_948_014);
	v.snapshot(&db).await;
	let position = v.position(&db, SOLVER).await.unwrap();
	assert!(position.starts_with("231481948014 "), "{position}");
	assert!(position.contains(" 20230586 0 265058976495 "), "{position}");
	assert_eq!(
		v.snapshot_of(&db, SOLVER).await.as_deref(),
		Some("265079207081 265087704647 8497566")
	);
}

#[tokio::test]
async fn both_tracked_owners_are_counted_once_and_earned_yield_survives_a_full_exit() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V06");
	v.legacy(&db, OTHER, 100, 100).await;
	v.model.set_rate(VAULT, 110, 100);
	v.model.set_balance(VAULT, OTHER, 0);
	v.model.set_balance(VAULT, SOLVER, 100);
	let event = v.transfer();
	// One Transfer log, OTHER → SOLVER.
	*v.model.logs.lock().unwrap() = vec![Log {
		inner: alloy::primitives::Log {
			address: addr(VAULT),
			data: Erc4626::Transfer { from: addr(OTHER), to: addr(SOLVER), value: U256::from(100) }
				.encode_log_data(),
		},
		block_number: Some(50),
		log_index: Some(3),
		transaction_hash: Some(event.transaction_hash.parse().unwrap()),
		..Default::default()
	}];
	v.record_transfer(&db, event.clone()).await.unwrap();
	v.record_transfer(&db, event).await.unwrap();
	assert_eq!(v.count(&db, "vault_ledger_events").await, 2);
	assert_eq!(v.position(&db, OTHER).await.as_deref(), Some("0 100 0 110 1 0"));
	assert_eq!(v.position(&db, SOLVER).await.as_deref(), Some("100 0 0 50 0 0 110 0 0 0"));
	v.snapshot(&db).await;
	assert_eq!(v.snapshot_of(&db, OTHER).await.as_deref(), Some("-10 0 10"));
	assert_eq!(v.snapshot_of(&db, SOLVER).await.as_deref(), Some("110 110 0"));
}

#[tokio::test]
async fn mint_burn_self_and_zero_share_transfers_are_not_capital() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V07");
	for input in [
		TransferInput { from: ZERO.into(), ..v.transfer() },
		TransferInput { to: ZERO.into(), ..v.transfer() },
		TransferInput { from: SOLVER.into(), ..v.transfer() },
		TransferInput { shares: I256::ZERO, ..v.transfer() },
	] {
		v.record_transfer(&db, input).await.unwrap();
	}
	assert_eq!(v.count(&db, "vault_ledger_events").await, 0);
	assert_eq!(v.model.convert_reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_deposit_and_a_withdrawal_are_each_counted_once() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V08");
	v.model.set_balance(VAULT, SOLVER, 20);
	v.block_events(&[v.ledger()]);
	v.record_transfer(&db, TransferInput { from: ZERO.into(), shares: int(20), ..v.transfer() })
		.await
		.unwrap();
	v.record(&db, v.ledger()).await.unwrap();
	v.record(&db, v.ledger()).await.unwrap();
	v.record_transfer(
		&db,
		TransferInput { from: SOLVER.into(), to: ZERO.into(), shares: int(20), ..v.transfer() },
	)
	.await
	.unwrap();
	v.record(&db, LedgerInput { event_type: Type::Withdraw, log_index: 10, ..v.ledger() })
		.await
		.unwrap();
	assert_eq!(v.position(&db, SOLVER).await.as_deref(), Some("0 0 0 50 20 20 0 0 1 1"));
}

#[tokio::test]
async fn a_missing_triggering_log_records_nothing_and_a_retry_succeeds() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V09");
	v.model.set_balance(VAULT, SOLVER, 20);
	let err = v.record(&db, v.ledger()).await.unwrap_err();
	assert!(err.to_string().contains("missing the triggering log"), "{err}");
	assert_eq!(v.count(&db, "vault_ledger_events").await, 0);
	v.block_events(&[v.ledger()]);
	v.record(&db, v.ledger()).await.unwrap();
	assert!(v.position(&db, SOLVER).await.unwrap().starts_with("20 "));
}

/** A contract can never delegate, so its code is read once; a wallet's is read at each event. */
#[tokio::test]
async fn a_contract_lp_has_its_code_read_once() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V19");
	let contract: Bytes = "0x6080604052".parse().unwrap();
	v.model.code.lock().unwrap().insert(addr(OTHER), contract);
	for log_index in [3, 4] {
		let transfer =
			TransferInput { from: OTHER.into(), to: THIRD.into(), log_index, ..v.transfer() };
		v.record_transfer(&db, transfer).await.unwrap();
	}
	assert_eq!(v.count(&db, "vault_ledger_events").await, 0);
	assert_eq!(v.model.code_reads.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn a_failed_delegation_check_is_an_error_not_a_skip() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V10");
	v.model.code_failures.store(usize::MAX, Ordering::SeqCst);
	assert!(v.record_transfer(&db, v.transfer()).await.is_err());
	assert_eq!(v.count(&db, "vault_ledger_events").await, 0);
}

#[tokio::test]
async fn an_unexplained_legacy_balance_withholds_only_that_snapshot() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V11");
	v.legacy(&db, SOLVER, 17_666_436, 20_230_586).await;
	v.legacy(&db, OTHER, 100, 100).await;
	v.model.set_balance(VAULT, SOLVER, 231_481_948_014);
	v.snapshot(&db).await;
	assert_eq!(v.snapshot_of(&db, SOLVER).await, None);
	assert!(v.snapshot_of(&db, OTHER).await.is_some());
	assert_eq!(v.count(&db, "vault_snapshots").await, 1);
}

#[tokio::test]
async fn a_failed_lp_read_retries_without_rewriting_completed_lps() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V12");
	v.legacy(&db, SOLVER, 100, 100).await;
	v.legacy(&db, OTHER, 100, 100).await;
	v.model.failing_holders.lock().unwrap().insert(addr(OTHER));
	v.snapshot(&db).await;
	assert!(v.snapshot_of(&db, SOLVER).await.is_some());
	assert_eq!(v.snapshot_of(&db, OTHER).await, None);
	assert_eq!(v.count(&db, "vault_snapshots").await, 0);

	v.model.failing_holders.lock().unwrap().clear();
	v.model.set_rate(VAULT, 2, 1);
	v.snapshot(&db).await;
	assert_eq!(v.snapshot_of(&db, SOLVER).await.as_deref(), Some("100 100 0"));
	assert_eq!(v.snapshot_of(&db, OTHER).await.as_deref(), Some("100 200 100"));
	assert_eq!(v.count(&db, "vault_snapshots").await, 1);
}

#[tokio::test]
async fn genuine_losses_are_kept_negative() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V13");
	v.legacy(&db, SOLVER, 100, 100).await;
	v.model.set_rate(VAULT, 9, 10);
	v.snapshot(&db).await;
	assert_eq!(v.snapshot_of(&db, SOLVER).await.as_deref(), Some("100 90 -10"));
}

#[tokio::test]
async fn a_transfer_already_in_both_ledgers_is_not_valued_again() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V14");
	v.legacy(&db, OTHER, 100, 100).await;
	v.legacy(&db, SOLVER, 0, 0).await;
	v.record_transfer(&db, v.transfer()).await.unwrap();
	v.model.convert_failures.store(1, Ordering::SeqCst);
	v.record_transfer(&db, v.transfer()).await.unwrap();
	assert_eq!(v.count(&db, "vault_ledger_events").await, 2);
}

#[tokio::test]
async fn unknown_vaults_are_ignored_without_reading_balances() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V15");
	v.record(&db, LedgerInput { vault: OTHER.into(), ..v.ledger() }).await.unwrap();
	v.record_transfer(&db, TransferInput { vault: OTHER.into(), ..v.transfer() })
		.await
		.unwrap();
	assert_eq!(v.count(&db, "vault_ledger_events").await, 0);
	assert_eq!(v.model.balance_reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_vault_read_failure_leaves_the_days_gate_open() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V16");
	v.legacy(&db, SOLVER, 100, 100).await;
	v.model.vault_reads_fail.store(true, Ordering::SeqCst);
	v.snapshot(&db).await;
	assert_eq!(v.snapshot_of(&db, SOLVER).await, None);
	assert_eq!(v.count(&db, "vault_snapshots").await, 0);
	v.model.vault_reads_fail.store(false, Ordering::SeqCst);
	v.snapshot(&db).await;
	assert!(v.snapshot_of(&db, SOLVER).await.is_some());
}

#[tokio::test]
async fn every_lp_is_snapshotted_across_the_page_boundary() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V17");
	for lp in solvers(0xd000, 205) {
		v.legacy(&db, &lp, 100, 100).await;
	}
	v.snapshot(&db).await;
	assert_eq!(v.count(&db, "vault_position_snapshots").await, 205);
	assert_eq!(v.count(&db, "vault_snapshots").await, 1);
}

#[tokio::test]
async fn each_utc_day_gets_its_own_snapshot_once() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let v = Vaults::new("EVM-V18");
	v.legacy(&db, SOLVER, 100, 100).await;
	for (block, timestamp) in [(100, 86_399), (150, 86_400), (200, 86_500), (250, 172_800)] {
		vaults::snapshot_chain(&db, &v.chain, &v.model, block, timestamp).await.unwrap();
	}
	let days = db
		.query(
			"SELECT day_start_timestamp::text, block_number::text FROM app.vault_snapshots WHERE chain = 'EVM-V18' ORDER BY snapshot_time",
			&[],
		)
		.await
		.unwrap()
		.iter()
		.map(|row| format!("{} {}", row.get::<_, String>(0), row.get::<_, String>(1)))
		.collect::<Vec<_>>();
	assert_eq!(days, vec!["0 100", "86400 150", "172800 250"]);
	assert_eq!(v.count(&db, "vault_position_snapshots").await, 3);
}

/** Each batch ends at a block; the one before it ended at the previous. */
#[test]
fn the_snapshot_runs_once_per_multiple_of_50_reached() {
	let ends = [101, 120, 149, 150, 151, 199, 260, 261];
	let due: Vec<u64> = ends
		.iter()
		.enumerate()
		.filter(|(index, end)| vaults::snapshot_due(index.checked_sub(1).map(|i| ends[i]), **end))
		.map(|(_, end)| *end)
		.collect();
	assert_eq!(due, vec![101, 150, 260]);
	// A block that rolled back is retried from the same previous block, so it is due again.
	assert!(vaults::snapshot_due(Some(149), 150) && vaults::snapshot_due(Some(149), 150));
}

#[test]
fn chain_config_lowercases_tokens_and_vaults() {
	let chain = SolverChain::from_config(&chain_config("EVM-V20", &[BASE_ACCOUNT]));
	assert_eq!(chain.token_for_vault(VAULT), Some(USDC));
	assert_eq!(chain.solver_accounts, vec![BASE_ACCOUNT.to_string()]);
	assert_eq!(lower(addr(VAULT)), VAULT);
	let _ = inventory::MAX_GENESIS_PER_BLOCK + HEAD_INTERVAL_SECS;
}
