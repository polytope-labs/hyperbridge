use std::{collections::HashMap, path::Path, sync::Arc};

use alloy::{
	primitives::{Address, Log, B256},
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
			Source::Pallet(key) => format!("../modules/utils/substrate-eth-rpc/abis/{key}.abi.json"),
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
	fn signature(&self) -> &'static str;
	async fn handle_log(&self, ctx: &EventContext<'_>, log: &Log) -> anyhow::Result<()>;
}

#[async_trait]
impl<H: EventHandler> LogHandler for H {
	fn topic(&self) -> B256 {
		H::Event::SIGNATURE_HASH
	}

	fn signature(&self) -> &'static str {
		H::Event::SIGNATURE
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
	pub fn event_name(&self) -> &'static str {
		let signature = self.handler.signature();
		&signature[..signature.find('(').unwrap_or(signature.len())]
	}

	pub fn topic(&self) -> B256 {
		self.handler.topic()
	}
}

/** The manifest name of a source on one network. Each network gets its own entry, so each can
 * have its own reorg-safe distance. */
pub fn contract_name(source: &Source, network: &str) -> String {
	format!("{}__{}", source.name(), network.replace('-', "_"))
}

/** Registers every subscription on every network rindexer's manifest lists for its source. */
pub fn register_all(
	indexer: &Arc<Indexer>,
	manifest: &Manifest,
	manifest_path: &Path,
	subscriptions: &[Subscription],
	providers: &HashMap<String, Arc<dyn ChainProvider>>,
	registry: &mut EventCallbackRegistry,
) -> anyhow::Result<()> {
	for contract in &manifest.contracts {
		for subscription in subscriptions {
			let [detail] = contract.details.as_slice() else {
				return Err(anyhow!("contract {} must list exactly one network", contract.name));
			};
			if contract.name != contract_name(&subscription.source, &detail.network) {
				continue;
			}
			let provider = providers
				.get(&detail.network)
				.ok_or_else(|| anyhow!("no provider for {}", detail.network))?
				.clone();
			let network = manifest
				.networks
				.iter()
				.find(|n| n.name == detail.network)
				.ok_or_else(|| anyhow!("network {} not in manifest", detail.network))?;

			let network_contract = NetworkContract {
				id: generate_random_id(10),
				network: detail.network.clone(),
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
				disable_logs_bloom_checks: network.disable_logs_bloom_checks.unwrap_or_default(),
			};

			let processor = format!("{}:{}", contract.name, subscription.handler.signature());
			let callback = {
				let indexer = indexer.clone();
				let handler = subscription.handler.clone();
				let processor = processor.clone();
				let network = detail.network.clone();
				Arc::new(move |results: Vec<EventResult>| {
					let indexer = indexer.clone();
					let handler = handler.clone();
					let processor = processor.clone();
					let network = network.clone();
					async move {
						process_batch(&indexer, &processor, &network, handler.as_ref(), results)
							.await
							.map_err(|err| {
								tracing::error!(%processor, "batch failed: {err:#}");
								format!("{err:#}")
							})
					}
					.boxed()
				})
			};

			registry.register_event(EventCallbackRegistryInformation {
				id: generate_random_id(10),
				indexer_name: manifest.name.clone(),
				event_name: subscription.event_name().to_string(),
				index_event_in_order: true,
				topic_id: subscription.topic(),
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
				providers: Arc::new(HashMap::from([(detail.network.clone(), provider)])),
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
	}
	Ok(())
}

/**
 * Runs a batch in one database transaction, together with the processor's cursor. rindexer
 * records its own progress after the callback returns, so a crash can replay a batch; the cursor
 * makes the replay skip logs that already committed.
 */
async fn process_batch(
	indexer: &Indexer,
	processor: &str,
	network: &str,
	handler: &dyn LogHandler,
	mut results: Vec<EventResult>,
) -> anyhow::Result<()> {
	if results.is_empty() {
		return Ok(());
	}
	let chain = indexer.chain(network)?;
	results.sort_by_key(|r| (r.tx_information.block_number, r.tx_information.log_index));

	let mut conn = indexer.db.raw_connection().await?;
	let tx = conn.transaction().await?;
	let cursor = tx
		.query_opt(
			"SELECT block_number, log_index FROM indexer.cursors WHERE processor = $1 FOR UPDATE",
			&[&processor],
		)
		.await?
		.map(|row| (row.get::<_, i64>(0) as u64, row.get::<_, i64>(1) as u64));

	let mut last = cursor;
	for result in &results {
		let info = &result.tx_information;
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
			log_index: info.log_index.to::<u64>(),
		};
		let position = (meta.block_number, meta.log_index);
		if last.is_some_and(|done| position <= done) {
			continue;
		}
		let ctx = EventContext { indexer, chain, db: &tx, log: &meta };
		handler.handle_log(&ctx, &result.log.inner).await?;
		last = Some(position);
	}

	if let Some((block_number, log_index)) = last {
		tx.execute(
			"INSERT INTO indexer.cursors (processor, block_number, log_index) VALUES ($1, $2, $3)
			 ON CONFLICT (processor) DO UPDATE SET block_number = $2, log_index = $3",
			&[&processor, &(block_number as i64), &(log_index as i64)],
		)
		.await?;
	}
	tx.commit().await?;
	Ok(())
}
