//! Solver inventory, solver discovery and yield vault accounting, ported from
//! `solverInventory.service.ts`, `solverWatchlist.service.ts` and `yieldVault.service.ts`.
//!
//! Every log of a chain arrives on one stream in block order, and a block hook runs after a
//! batch's logs, at the last block the batch applied. A storage read pinned to that block already
//! includes every applied log, so it is positioned after all of them, as SubQuery's block handler
//! read was positioned after its block's logs.

pub mod history;
pub mod inventory;
pub mod reads;
pub mod vaults;
pub mod watchlist;

use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, Mutex},
};

use alloy::primitives::{Address, I256, U256};
use anyhow::anyhow;
use async_trait::async_trait;

use crate::{
	config::ChainConfig,
	handlers::{
		abi::{Erc20, Erc4626},
		Handlers,
	},
	registry::{BlockContext, BlockHook, EventContext, EventHandler, Source, Subscription},
	store::VaultLedgerEventType,
	Indexer,
};

pub use reads::Reads;

pub const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
const DELEGATION_INDICATOR_PREFIX: &str = "0xef0100";

/** A token whose inventory is tracked, with the ERC-4626 vaults that wrap it. Lowercase. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedToken {
	pub token: String,
	pub vaults: Vec<String>,
}

/**
 * What this area knows about one chain: its supported tokens and SolverAccounts, plus the
 * process state SubQuery kept in its cache. `id` is the state machine id, e.g. `EVM-8453`.
 */
pub struct SolverChain {
	pub id: String,
	pub tokens: Vec<SupportedToken>,
	pub solver_accounts: Vec<String>,
	/** The TRACKED solvers, a filter in front of the row lookups. None until first loaded. */
	tracked: Mutex<Option<HashSet<String>>>,
	/** LPs seen with contract code. A contract can never take an EIP-7702 delegation. */
	contracts: Mutex<HashSet<String>>,
	/** Set once Multicall3 has been found; an absence is probed again. */
	multicall3: Mutex<bool>,
}

impl SolverChain {
	pub fn new(id: &str, tokens: Vec<SupportedToken>, solver_accounts: Vec<String>) -> Self {
		Self {
			id: id.to_string(),
			tokens,
			solver_accounts,
			tracked: Mutex::new(None),
			contracts: Mutex::new(HashSet::new()),
			multicall3: Mutex::new(false),
		}
	}

	pub fn from_config(config: &ChainConfig) -> Self {
		let tokens = config
			.contracts
			.yield_vaults
			.iter()
			.map(|(token, vaults)| SupportedToken {
				token: lower(*token),
				vaults: vaults.vaults.iter().map(|vault| lower(*vault)).collect(),
			})
			.collect();
		let accounts = config.contracts.solver_accounts.iter().map(|a| lower(*a)).collect();
		Self::new(&config.state_machine_id, tokens, accounts)
	}

	/** `tokenForVault` / `underlyingTokenFor`: the token whose vault list holds `vault`. */
	pub fn token_for_vault(&self, vault: &str) -> Option<&str> {
		let vault = vault.to_lowercase();
		self.tokens.iter().find(|t| t.vaults.contains(&vault)).map(|t| t.token.as_str())
	}

	/** `configuredVaults`: every vault with its underlying token, in config order. */
	pub fn vaults(&self) -> impl Iterator<Item = (&str, &str)> {
		self.tokens
			.iter()
			.flat_map(|t| t.vaults.iter().map(move |v| (v.as_str(), t.token.as_str())))
	}

	pub fn is_solver_account(&self, address: &str) -> bool {
		self.solver_accounts.iter().any(|account| account == address)
	}
}

/** The per-chain state, built from each chain's config the first time a handler needs it. */
#[derive(Default)]
pub struct Solvers {
	chains: Mutex<HashMap<String, Arc<SolverChain>>>,
}

impl Solvers {
	pub fn chain(&self, config: &ChainConfig) -> Arc<SolverChain> {
		self.chains
			.lock()
			.unwrap()
			.entry(config.state_machine_id.clone())
			.or_insert_with(|| Arc::new(SolverChain::from_config(config)))
			.clone()
	}
}

/** An address as this area stores it: lowercase `0x` hex. */
pub fn lower(address: Address) -> String {
	format!("{address:#x}")
}

/** A uint256 as a signed amount. Token amounts never come near the sign bit. */
pub fn signed(value: U256) -> anyhow::Result<I256> {
	I256::try_from(value).map_err(|_| anyhow!("{value} does not fit a signed 256-bit integer"))
}

/**
 * The error of a read of a configured vault that a row can't be written without. Such a vault
 * reverts only on a node whose state trails its header, so the batch retries until the node
 * has caught up.
 */
pub fn vault_read_retry(err: impl std::fmt::Display) -> anyhow::Error {
	anyhow!(crate::errors::Retry(format!("vault read: {err:#}")))
}

/** A non-negative amount as a uint256, for a contract argument. */
pub fn unsigned(value: I256) -> U256 {
	value.unsigned_abs()
}

/** EIP-7702 delegation read from account code. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationReading {
	pub delegated: bool,
	pub delegate: Option<String>,
}

/**
 * `parseDelegation`: `0xef0100 ‖ addr`, exactly, counted only for a known SolverAccount.
 * `delegate` is set for any designator.
 */
pub fn parse_delegation(chain: &SolverChain, code: Option<&str>) -> DelegationReading {
	let lower = code.unwrap_or("0x").to_lowercase();
	if !lower.starts_with(DELEGATION_INDICATOR_PREFIX) ||
		lower.len() != DELEGATION_INDICATOR_PREFIX.len() + 40
	{
		return DelegationReading { delegated: false, delegate: None };
	}
	let delegate = format!("0x{}", &lower[DELEGATION_INDICATOR_PREFIX.len()..]);
	DelegationReading { delegated: chain.is_solver_account(&delegate), delegate: Some(delegate) }
}

/**
 * `isDelegatedSolver`'s check of code already read: a `0xef0100` prefix whose remainder is a
 * SolverAccount. Unlike `parse_delegation` it has no length check, as in the TypeScript.
 */
pub fn code_delegates_to_solver_account(chain: &SolverChain, code: &str) -> bool {
	if chain.solver_accounts.is_empty() {
		return false;
	}
	let lower = code.to_lowercase();
	if code.is_empty() || !lower.starts_with(DELEGATION_INDICATOR_PREFIX) {
		return false;
	}
	let delegated_to = format!("0x{}", &lower[DELEGATION_INDICATOR_PREFIX.len()..]);
	chain.is_solver_account(&delegated_to)
}

/** Registers the token and vault handlers and the inventory and snapshot hooks. */
pub fn register(handlers: &mut Handlers) {
	let solvers = Arc::new(Solvers::default());
	handlers.subscriptions.extend([
		Subscription::new(Source::SupportedTokens, TokenTransfer(solvers.clone())),
		Subscription::new(Source::YieldVaults, VaultTransfer(solvers.clone())),
		Subscription::new(Source::YieldVaults, VaultDeposit(solvers.clone())),
		Subscription::new(Source::YieldVaults, VaultWithdraw(solvers.clone())),
	]);
	handlers.hooks.push(Arc::new(InventoryHook(solvers.clone())));
	handlers.hooks.push(Arc::new(SnapshotHook(solvers)));
}

/** The solver watchlist poll. */
pub fn spawn_tasks(indexer: &Arc<Indexer>) {
	watchlist::spawn_poller(indexer.clone());
}

/** `handleSolverTokenTransferEvent`. */
struct TokenTransfer(Arc<Solvers>);

#[async_trait]
impl EventHandler for TokenTransfer {
	type Event = Erc20::Transfer;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let chain = self.0.chain(&ctx.chain.config);
		inventory::apply_token_transfer(
			ctx.db,
			&chain,
			&inventory::Transfer {
				contract: lower(ctx.log.address),
				from: lower(event.from),
				to: lower(event.to),
				value: signed(event.value)?,
				block_number: ctx.log.block_number,
				log_index: ctx.log.log_index,
				timestamp: ctx.log.block_timestamp,
			},
		)
		.await
	}
}

/** `handleVaultTransferEvent`: the inventory's share movement, then the yield ledger. */
struct VaultTransfer(Arc<Solvers>);

#[async_trait]
impl EventHandler for VaultTransfer {
	type Event = Erc4626::Transfer;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let chain = self.0.chain(&ctx.chain.config);
		let transfer = inventory::Transfer {
			contract: lower(ctx.log.address),
			from: lower(event.from),
			to: lower(event.to),
			value: signed(event.value)?,
			block_number: ctx.log.block_number,
			log_index: ctx.log.log_index,
			timestamp: ctx.log.block_timestamp,
		};
		inventory::apply_vault_share_transfer(ctx.db, &chain, &ctx.chain.provider, &transfer)
			.await?;
		if vaults::is_ordinary_vault_transfer(&transfer.from, &transfer.to, transfer.value) {
			vaults::record_transfer(
				ctx.db,
				&chain,
				&ctx.chain.provider,
				&vaults::TransferInput {
					vault: transfer.contract.clone(),
					from: transfer.from.clone(),
					to: transfer.to.clone(),
					shares: transfer.value,
					block_number: ctx.log.block_number,
					transaction_hash: format!("{:#x}", ctx.log.transaction_hash),
					log_index: ctx.log.log_index,
					timestamp: ctx.log.block_timestamp,
				},
			)
			.await?;
		}
		Ok(())
	}
}

/** `handleVaultDepositEvent`. */
struct VaultDeposit(Arc<Solvers>);

#[async_trait]
impl EventHandler for VaultDeposit {
	type Event = Erc4626::Deposit;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let chain = self.0.chain(&ctx.chain.config);
		vaults::record_ledger(
			ctx.db,
			&chain,
			&ctx.chain.provider,
			&vaults::LedgerInput {
				vault: lower(ctx.log.address),
				lp: lower(event.owner),
				caller: lower(event.sender),
				receiver: None,
				assets: signed(event.assets)?,
				shares: signed(event.shares)?,
				event_type: VaultLedgerEventType::Deposit,
				block_number: ctx.log.block_number,
				transaction_hash: format!("{:#x}", ctx.log.transaction_hash),
				log_index: ctx.log.log_index,
				timestamp: ctx.log.block_timestamp,
			},
		)
		.await
	}
}

/** `handleVaultWithdrawEvent`. */
struct VaultWithdraw(Arc<Solvers>);

#[async_trait]
impl EventHandler for VaultWithdraw {
	type Event = Erc4626::Withdraw;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let chain = self.0.chain(&ctx.chain.config);
		vaults::record_ledger(
			ctx.db,
			&chain,
			&ctx.chain.provider,
			&vaults::LedgerInput {
				vault: lower(ctx.log.address),
				lp: lower(event.owner),
				caller: lower(event.sender),
				receiver: Some(lower(event.receiver)),
				assets: signed(event.assets)?,
				shares: signed(event.shares)?,
				event_type: VaultLedgerEventType::Withdraw,
				block_number: ctx.log.block_number,
				transaction_hash: format!("{:#x}", ctx.log.transaction_hash),
				log_index: ctx.log.log_index,
				timestamp: ctx.log.block_timestamp,
			},
		)
		.await
	}
}

/** `handleSolverInventoryBlock`, after every batch on chains with supported tokens. */
struct InventoryHook(Arc<Solvers>);

#[async_trait]
impl BlockHook for InventoryHook {
	fn applies_to(&self, chain: &ChainConfig) -> bool {
		!chain.contracts.yield_vaults.is_empty()
	}

	async fn after_batch(&self, ctx: &BlockContext<'_>) -> anyhow::Result<()> {
		let chain = self.0.chain(&ctx.chain.config);
		inventory::index_block(
			ctx.db,
			&chain,
			&ctx.chain.provider,
			ctx.block_number,
			ctx.block_timestamp,
		)
		.await
	}
}

/** `handleVaultSnapshotIndexing`: SubQuery's `modulo: 50`, on chains with at least one vault. */
struct SnapshotHook(Arc<Solvers>);

#[async_trait]
impl BlockHook for SnapshotHook {
	fn applies_to(&self, chain: &ChainConfig) -> bool {
		chain.contracts.yield_vaults.values().any(|v| !v.vaults.is_empty())
	}

	async fn after_batch(&self, ctx: &BlockContext<'_>) -> anyhow::Result<()> {
		let chain = self.0.chain(&ctx.chain.config);
		if !vaults::snapshot_due(ctx.previous_block, ctx.block_number) {
			return Ok(());
		}
		vaults::snapshot_chain(
			ctx.db,
			&chain,
			&ctx.chain.provider,
			ctx.block_number,
			ctx.block_timestamp,
		)
		.await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const SOLVER_ACCOUNT: &str = "0xaad062555800a97af062795189e32a3cbd045612";
	const OTHER: &str = "0x13e41cde1d55880cbe031c69f206c2e9bc3c94c2";

	fn base() -> SolverChain {
		SolverChain::new("EVM-8453", vec![], vec![SOLVER_ACCOUNT.into()])
	}

	#[test]
	fn parse_delegation_counts_only_an_exact_designator_for_a_known_account() {
		let delegated = format!("0xef0100{}", &SOLVER_ACCOUNT[2..]);
		let reading = |chain: &SolverChain, code: &str| parse_delegation(chain, Some(code));
		assert_eq!(
			reading(&base(), &delegated.to_uppercase().replace("0X", "0x")),
			DelegationReading { delegated: true, delegate: Some(SOLVER_ACCOUNT.into()) }
		);
		assert_eq!(
			reading(&base(), &format!("0xef0100{}", &OTHER[2..])),
			DelegationReading { delegated: false, delegate: Some(OTHER.into()) }
		);
		assert_eq!(
			reading(&base(), &format!("{delegated}00")),
			DelegationReading { delegated: false, delegate: None }
		);
		assert_eq!(
			reading(&base(), "0x6080604052"),
			DelegationReading { delegated: false, delegate: None }
		);
		let ethereum = SolverChain::new("EVM-1", vec![], vec![]);
		assert_eq!(
			reading(&ethereum, &delegated),
			DelegationReading { delegated: false, delegate: Some(SOLVER_ACCOUNT.into()) }
		);
		assert!(!parse_delegation(&base(), None).delegated);
	}

	#[test]
	fn the_ledger_delegation_check_rejects_unrelated_or_malformed_code() {
		let chain = base();
		for code in ["0x", "0x60006000", &format!("0xef0100{}", "ab".repeat(20)), "0xef0100"] {
			assert!(!code_delegates_to_solver_account(&chain, code), "{code}");
		}
		let upper = format!("0xEF0100{}", SOLVER_ACCOUNT[2..].to_uppercase());
		assert!(code_delegates_to_solver_account(&chain, &upper));
		let ethereum = SolverChain::new("EVM-999", vec![], vec![]);
		assert!(!code_delegates_to_solver_account(&ethereum, &upper));
	}

	#[test]
	fn mainnet_routes_token_and_vault_logs_and_hooks_only_where_configured() {
		use crate::{config::Config, registry::ChainRoutes};
		let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config/mainnet.toml");
		let config = Config::load(&path).unwrap();
		let mut handlers = Handlers::default();
		register(&mut handlers);
		let base = &config.chains["base-mainnet"];
		let routes = ChainRoutes::new(base, &handlers.subscriptions, &handlers.hooks);
		let usdc: Address = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913".parse().unwrap();
		let vault: Address = "0xC768c589647798a6EE01A91FdE98EF2ed046DBD6".parse().unwrap();
		assert!(routes.addresses().contains(&usdc) && routes.addresses().contains(&vault));
		assert!(InventoryHook(Default::default()).applies_to(base));
		assert!(SnapshotHook(Default::default()).applies_to(base));
		let hub = &config.chains["polkadot-hub-mainnet"];
		assert!(InventoryHook(Default::default()).applies_to(hub));
		assert!(!SnapshotHook(Default::default()).applies_to(hub));
		let nexus = &config.chains["hyperbridge-nexus"];
		assert!(!InventoryHook(Default::default()).applies_to(nexus));
	}

	#[test]
	fn vaults_resolve_their_token_case_insensitively() {
		let token = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913";
		let vault = "0xc768c589647798a6ee01a91fde98ef2ed046dbd6";
		let chain = SolverChain::new(
			"EVM-8453",
			vec![SupportedToken { token: token.into(), vaults: vec![vault.into()] }],
			vec![],
		);
		assert_eq!(
			chain.token_for_vault("0xC768c589647798a6EE01A91FdE98EF2ed046DBD6"),
			Some(token)
		);
		assert_eq!(chain.token_for_vault(OTHER), None);
		assert_eq!(chain.vaults().collect::<Vec<_>>(), vec![(vault, token)]);
	}
}
