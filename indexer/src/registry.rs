use std::{
	collections::{BTreeSet, HashMap},
	path::Path,
	sync::Arc,
	time::Duration,
};

use alloy::{
	primitives::{Address, Log, B256},
	providers::Provider,
	sol_types::SolEvent,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use futures::FutureExt;
use rindexer::{
	blockclock::BlockClock,
	event::{
		callback_registry::{
			noop_decoder, EventCallbackRegistry, EventCallbackRegistryInformation, EventResult,
		},
		contract_setup::{ContractInformation, NetworkContract},
	},
	generate_random_id,
	manifest::core::Manifest,
	provider::ChainProvider,
};
use tokio_postgres::Transaction;

use crate::{
	chains::Chain,
	config::{ChainConfig, ChainKind},
	Indexer,
};

/** Where an event comes from: an EVM contract or a Substrate pallet behind substrate-eth-rpc. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
	EthereumHost,
	Erc6160Ext20,
	IntentGatewayV3,
	/** The same gateway address, read with the pre-quote `OrderPlaced` ABI. */
	IntentGatewayV3Legacy,
	/** Every ERC-4626 vault in `contracts.yield_vaults`. */
	YieldVaults,
	/** Every token in `contracts.yield_vaults`, for solver balance transfers. */
	SupportedTokens,
	Pallet(&'static str),
}

impl Source {
	pub fn name(&self) -> String {
		match self {
			Source::EthereumHost => "EthereumHost".into(),
			Source::Erc6160Ext20 => "Erc6160Ext20".into(),
			Source::IntentGatewayV3 => "IntentGatewayV3".into(),
			Source::IntentGatewayV3Legacy => "IntentGatewayV3Legacy".into(),
			Source::YieldVaults => "YieldVaults".into(),
			Source::SupportedTokens => "SupportedTokens".into(),
			Source::Pallet(key) => format!("Pallet_{key}"),
		}
	}

	/** The ABI file rindexer reads for this source, relative to the crate root. */
	pub fn abi(&self) -> String {
		match self {
			Source::EthereumHost => "abis/EthereumHost.abi.json".into(),
			Source::Erc6160Ext20 => "abis/ERC6160Ext20.abi.json".into(),
			Source::IntentGatewayV3 => "abis/IntentGatewayV3.abi.json".into(),
			Source::IntentGatewayV3Legacy => "abis/IntentGatewayV3Legacy.abi.json".into(),
			Source::YieldVaults => "abis/Erc4626.abi.json".into(),
			Source::SupportedTokens => "abis/Erc20.abi.json".into(),
			Source::Pallet(key) =>
				format!("../modules/utils/substrate-eth-rpc/abis/{key}.abi.json"),
		}
	}

	/** The addresses to watch on a chain. Empty means the source doesn't exist there. */
	pub fn addresses(&self, chain: &ChainConfig) -> Vec<Address> {
		let contracts = &chain.contracts;
		match (self, chain.kind) {
			(Source::Pallet(key), ChainKind::Substrate) => vec![pallet_address(key)],
			(Source::Pallet(_), ChainKind::Evm) => vec![],
			(_, ChainKind::Substrate) => vec![],
			(Source::EthereumHost, _) => contracts.ethereum_host.into_iter().collect(),
			(Source::Erc6160Ext20, _) => contracts.erc6160ext20.into_iter().collect(),
			(Source::IntentGatewayV3 | Source::IntentGatewayV3Legacy, _) =>
				contracts.intent_gateway_v3.into_iter().collect(),
			(Source::YieldVaults, _) =>
				contracts.yield_vaults.values().flat_map(|v| v.vaults.iter().copied()).collect(),
			(Source::SupportedTokens, _) => contracts.yield_vaults.keys().copied().collect(),
		}
	}
}

/** The synthetic address substrate-eth-rpc gives a pallet's logs. */
pub fn pallet_address(key: &str) -> Address {
	let preimage = format!("substrate-pallet:{key}");
	Address::from_slice(&alloy::primitives::keccak256(preimage.as_bytes())[12..])
}

/** Where a log sits on its chain. */
#[derive(Debug, Clone)]
pub struct LogMeta {
	pub address: Address,
	pub block_number: u64,
	pub block_hash: B256,
	/** Block timestamp in seconds. */
	pub block_timestamp: u64,
	pub transaction_hash: B256,
	pub transaction_index: u64,
	pub log_index: u64,
}

/** What a handler sees for one log: shared services, its chain, and the batch's transaction. */
pub struct EventContext<'a> {
	pub indexer: &'a Indexer,
	pub chain: &'a Chain,
	pub db: &'a Transaction<'a>,
	pub log: &'a LogMeta,
}

/** Handles one decoded event. Writes go through `ctx.db`, which commits with the batch. */
#[async_trait]
pub trait EventHandler: Send + Sync + 'static {
	type Event: SolEvent + Send + Sync + 'static;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()>;
}

#[async_trait]
trait LogHandler: Send + Sync {
	fn topic(&self) -> B256;
	async fn handle_log(&self, ctx: &EventContext<'_>, log: &Log) -> anyhow::Result<()>;
}

#[async_trait]
impl<H: EventHandler> LogHandler for H {
	fn topic(&self) -> B256 {
		H::Event::SIGNATURE_HASH
	}

	async fn handle_log(&self, ctx: &EventContext<'_>, log: &Log) -> anyhow::Result<()> {
		let event = H::Event::decode_log(log)
			.with_context(|| format!("decoding {} at {:?}", H::Event::SIGNATURE, ctx.log))?;
		self.handle(ctx, event.data).await
	}
}

/** One event from one source, and the handler that indexes it. */
#[derive(Clone)]
pub struct Subscription {
	pub source: Source,
	handler: Arc<dyn LogHandler>,
}

impl Subscription {
	pub fn new(source: Source, handler: impl EventHandler) -> Self {
		Self { source, handler: Arc::new(handler) }
	}

	/** The event name rindexer matches against the ABI, e.g. `PostRequestEvent`. */
	pub fn topic(&self) -> B256 {
		self.handler.topic()
	}
}

/** The one event each chain's rindexer stream is registered under. `AllTopicsProvider` widens
 * its `eth_getLogs` to every topic the chain's handlers subscribe to. */
pub const STREAM_EVENT: &str = "AllEvents";

pub fn stream_topic() -> B256 {
	alloy::primitives::keccak256(format!("{STREAM_EVENT}()"))
}

/** The manifest contract name of a chain's stream. */
pub fn stream_name(network: &str) -> String {
	format!("Chain__{}", network.replace('-', "_"))
}

/** The ABI rindexer is given for every stream: one event with no inputs. */
pub fn stream_abi() -> String {
	format!(r#"[{{"type":"event","name":"{STREAM_EVENT}","inputs":[],"anonymous":false}}]"#)
}

/** Where a chain's stream stands at the end of a batch: every log up to this block is applied. */
pub struct BlockContext<'a> {
	pub indexer: &'a Indexer,
	pub chain: &'a Chain,
	pub db: &'a Transaction<'a>,
	pub block_number: u64,
	/** Block timestamp in seconds. */
	pub block_timestamp: u64,
}

/**
 * Work SubQuery ran as a block handler. It runs after each batch's logs, in the batch's
 * transaction, at the last block the batch applied. rindexer only calls back for ranges with
 * logs, so a hook runs as often as its chain has logs we subscribe to.
 */
#[async_trait]
pub trait BlockHook: Send + Sync + 'static {
	/** Whether the hook runs on this chain. */
	fn applies_to(&self, chain: &ChainConfig) -> bool;

	async fn after_batch(&self, ctx: &BlockContext<'_>) -> anyhow::Result<()>;
}

/** A chain's handlers, keyed by the address and topic of the logs they take. */
#[derive(Default)]
pub struct ChainRoutes {
	routes: HashMap<(Address, B256), Vec<Arc<dyn LogHandler>>>,
	hooks: Vec<Arc<dyn BlockHook>>,
	/** Held while a batch or an idle hook run writes, so the two never interleave. */
	writing: tokio::sync::Mutex<()>,
}

impl ChainRoutes {
	pub fn new(
		chain: &ChainConfig,
		subscriptions: &[Subscription],
		hooks: &[Arc<dyn BlockHook>],
	) -> Self {
		let mut routes: HashMap<(Address, B256), Vec<Arc<dyn LogHandler>>> = HashMap::new();
		for subscription in subscriptions {
			for address in subscription.source.addresses(chain) {
				routes
					.entry((address, subscription.topic()))
					.or_default()
					.push(subscription.handler.clone());
			}
		}
		let hooks = hooks.iter().filter(|hook| hook.applies_to(chain)).cloned().collect();
		Self { routes, hooks, writing: Default::default() }
	}

	pub fn addresses(&self) -> BTreeSet<Address> {
		self.routes.keys().map(|(address, _)| *address).collect()
	}

	pub fn topics(&self) -> BTreeSet<B256> {
		self.routes.keys().map(|(_, topic)| *topic).collect()
	}

	fn handlers(&self, address: Address, topic: B256) -> &[Arc<dyn LogHandler>] {
		self.routes.get(&(address, topic)).map(Vec::as_slice).unwrap_or_default()
	}
}

/** Registers each chain's stream with rindexer. */
pub fn register_all(
	indexer: &Arc<Indexer>,
	manifest: &Manifest,
	manifest_path: &Path,
	routes: &HashMap<String, Arc<ChainRoutes>>,
	providers: &HashMap<String, Arc<dyn ChainProvider>>,
	registry: &mut EventCallbackRegistry,
) -> anyhow::Result<()> {
	for contract in &manifest.contracts {
		let [detail] = contract.details.as_slice() else {
			return Err(anyhow!("contract {} must list exactly one network", contract.name));
		};
		let network = detail.network.clone();
		let provider = providers
			.get(&network)
			.ok_or_else(|| anyhow!("no provider for {network}"))?
			.clone();
		let chain_routes =
			routes.get(&network).ok_or_else(|| anyhow!("no routes for {network}"))?.clone();

		let network_contract = NetworkContract {
			id: generate_random_id(10),
			network: network.clone(),
			cached_provider: provider.clone(),
			block_clock: BlockClock::new(
				manifest.timestamps,
				manifest.config.timestamp_sample_rate,
				provider.clone(),
			),
			decoder: noop_decoder(),
			indexing_contract_setup: detail.indexing_contract_setup(manifest_path),
			start_block: detail.start_block,
			end_block: detail.end_block,
			// The bloom pre-check only knows the stream's placeholder topic.
			disable_logs_bloom_checks: true,
		};

		let callback = {
			let indexer = indexer.clone();
			let network = network.clone();
			Arc::new(move |results: Vec<EventResult>| {
				let indexer = indexer.clone();
				let network = network.clone();
				let chain_routes = chain_routes.clone();
				async move {
					process_batch(&indexer, &network, &chain_routes, results).await.map_err(|err| {
						tracing::error!(%network, "batch failed: {err:#}");
						format!("{err:#}")
					})
				}
				.boxed()
			})
		};

		registry.register_event(EventCallbackRegistryInformation {
			id: generate_random_id(10),
			indexer_name: manifest.name.clone(),
			event_name: STREAM_EVENT.to_string(),
			index_event_in_order: true,
			topic_id: stream_topic(),
			contract: ContractInformation {
				name: contract.name.clone(),
				details: vec![network_contract],
				abi: contract.abi.clone(),
				reorg_safe_distance: contract.reorg_safe_distance,
			},
			callback,
			tables: Arc::new(vec![]),
			reorg_sender: None,
			streams_clients: Arc::new(None),
			providers: Arc::new(HashMap::from([(network.clone(), provider)])),
			constants: Arc::new(manifest.constants.clone()),
			multicall_addresses: Arc::new(
				manifest
					.networks
					.iter()
					.map(|n| (n.name.clone(), n.multicall3_address.clone()))
					.collect(),
			),
		});
	}
	Ok(())
}

/**
 * Runs a batch of one chain's logs in block order, in one database transaction together with
 * the chain's cursor. rindexer records its own progress after the callback returns, so a crash
 * can replay a batch; the cursor makes the replay skip logs that already committed.
 */
async fn process_batch(
	indexer: &Indexer,
	network: &str,
	routes: &ChainRoutes,
	mut results: Vec<EventResult>,
) -> anyhow::Result<()> {
	if results.is_empty() {
		return Ok(());
	}
	let chain = indexer.chain(network)?;
	results.sort_by_key(|r| (r.tx_information.block_number, r.tx_information.log_index));

	let _writing = routes.writing.lock().await;
	let mut conn = indexer.db.raw_connection().await?;
	let tx = conn.transaction().await?;
	let cursor = tx
		.query_opt(
			"SELECT block_number, log_index FROM indexer.cursors WHERE network = $1 FOR UPDATE",
			&[&network],
		)
		.await?
		.map(|row| (row.get::<_, i64>(0) as u64, row.get::<_, i64>(1) as u64));

	let mut last = cursor;
	let mut applied_to = None;
	for result in &results {
		let info = &result.tx_information;
		let position = (info.block_number, info.log_index.to::<u64>());
		if last.is_some_and(|done| position <= done) {
			continue;
		}
		last = Some(position);
		let Some(topic) = result.log.inner.topics().first().copied() else { continue };
		// The request is every address crossed with every topic, so some logs match no route.
		let handlers = routes.handlers(info.address, topic);
		if handlers.is_empty() {
			continue;
		}
		let meta = LogMeta {
			address: info.address,
			block_number: info.block_number,
			block_hash: info.block_hash,
			block_timestamp: info
				.block_timestamp
				.map(|t| t.to::<u64>())
				.ok_or_else(|| anyhow!("log at block {} has no timestamp", info.block_number))?,
			transaction_hash: info.transaction_hash,
			transaction_index: info.transaction_index,
			log_index: position.1,
		};
		let ctx = EventContext { indexer, chain, db: &tx, log: &meta };
		for handler in handlers {
			handler.handle_log(&ctx, &result.log.inner).await?;
		}
		applied_to = Some((meta.block_number, meta.block_timestamp));
	}

	if let Some((block_number, block_timestamp)) = applied_to {
		let ctx = BlockContext { indexer, chain, db: &tx, block_number, block_timestamp };
		for hook in &routes.hooks {
			hook.after_batch(&ctx).await?;
		}
	}

	if let Some((block_number, log_index)) = last {
		tx.execute(
			"INSERT INTO indexer.cursors (network, block_number, log_index) VALUES ($1, $2, $3)
			 ON CONFLICT (network) DO UPDATE SET block_number = $2, log_index = $3",
			&[&network, &(block_number as i64), &(log_index as i64)],
		)
		.await?;
	}
	tx.commit().await?;
	Ok(())
}

/** How often a chain's hooks catch up when its stream has had no logs. */
const IDLE_HOOK_INTERVAL: Duration = Duration::from_secs(15);

/**
 * rindexer only calls back for ranges that have logs, so on a quiet chain the hooks would stop.
 * This runs them every [`IDLE_HOOK_INTERVAL`] at the block rindexer has synced to, when that is
 * past the chain's cursor. rindexer records that block only after the callbacks for every range
 * up to it returned, so every log up to it is applied.
 */
pub fn spawn_idle_hooks(indexer: Arc<Indexer>, routes: &HashMap<String, Arc<ChainRoutes>>) {
	for (network, chain_routes) in routes {
		if chain_routes.hooks.is_empty() {
			continue;
		}
		let (indexer, network, chain_routes) =
			(indexer.clone(), network.clone(), chain_routes.clone());
		tokio::spawn(async move {
			let mut interval = tokio::time::interval(IDLE_HOOK_INTERVAL);
			loop {
				interval.tick().await;
				if let Err(err) = run_idle_hooks(&indexer, &network, &chain_routes).await {
					tracing::warn!(%network, "idle hooks failed: {err:#}");
				}
			}
		});
	}
}

/** rindexer's progress table for a chain's stream (`rindexer_internal.<indexer>_<contract>_<event>`). */
fn progress_table(network: &str) -> String {
	format!("hyperbridge_indexer_{}_all_events", stream_name(network).to_lowercase())
}

async fn run_idle_hooks(indexer: &Indexer, network: &str, routes: &ChainRoutes) -> anyhow::Result<()> {
	let chain = indexer.chain(network)?;
	let _writing = routes.writing.lock().await;
	let mut conn = indexer.db.raw_connection().await?;
	let tx = conn.transaction().await?;
	let table = format!("rindexer_internal.{}", progress_table(network));
	let exists: bool = tx
		.query_one("SELECT to_regclass($1) IS NOT NULL", &[&table])
		.await?
		.get(0);
	if !exists {
		// Before rindexer's first sync, or if a rindexer upgrade renamed the table.
		tracing::debug!(%network, "no progress table {table} yet");
		return Ok(());
	}
	let synced: Option<String> = tx
		.query_opt(&format!("SELECT last_synced_block::text FROM {table} WHERE network = $1"), &[&network])
		.await?
		.and_then(|row| row.get(0));
	let Some(synced) = synced.and_then(|s| s.parse::<u64>().ok()).filter(|b| *b > 0) else {
		return Ok(());
	};
	let cursor: Option<i64> = tx
		.query_opt("SELECT block_number FROM indexer.cursors WHERE network = $1 FOR UPDATE", &[&network])
		.await?
		.map(|row| row.get(0));
	if cursor.is_some_and(|block| block as u64 >= synced) {
		return Ok(());
	}
	let block = chain
		.provider
		.get_block_by_number(synced.into())
		.await?
		.ok_or_else(|| anyhow!("{network}: block {synced} not found"))?;
	let ctx = BlockContext {
		indexer,
		chain,
		db: &tx,
		block_number: synced,
		block_timestamp: block.header.timestamp,
	};
	for hook in &routes.hooks {
		hook.after_batch(&ctx).await?;
	}
	// The whole block is applied, so its log index is the largest there is.
	tx.execute(
		"INSERT INTO indexer.cursors (network, block_number, log_index) VALUES ($1, $2, $3)
		 ON CONFLICT (network) DO UPDATE SET block_number = $2, log_index = $3",
		&[&network, &(synced as i64), &i64::MAX],
	)
	.await?;
	tx.commit().await?;
	Ok(())
}
