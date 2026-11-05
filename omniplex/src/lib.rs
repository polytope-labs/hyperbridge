pub mod chains;
pub mod config;
pub mod db;
pub mod errors;
pub mod failover;
pub mod handlers;
pub mod manifest;
pub mod metadata;
pub mod provider;
pub mod registry;
pub mod services;
pub mod store;
pub mod subquery;
pub mod substrate;

use std::{
	collections::{BTreeMap, HashMap},
	path::Path,
	sync::Arc,
};

use anyhow::{anyhow, Context};
use rindexer::{
	event::callback_registry::{EventCallbackRegistry, TraceCallbackRegistry},
	provider::{create_client, ChainProvider, RetryClientError},
	start_rindexer, GraphqlOverrideSettings, HeaderMap, IndexingDetails, PostgresClient,
	StartDetails,
};

use crate::{
	chains::Chain,
	config::{env_key, env_urls, ChainKind, Config},
	substrate::SubstrateRpc,
};

/** How long each of a chain's several nodes gets at startup to answer the chain-id check of
 * rindexer's client. */
const CLIENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/** Services shared by every handler. */
pub struct Indexer {
	pub config: Config,
	pub db: Arc<PostgresClient>,
	pub chains: BTreeMap<String, Chain>,
	pub http: reqwest::Client,
	pub orderbook: services::prices::Orderbook,
}

impl Indexer {
	pub fn chain(&self, network: &str) -> anyhow::Result<&Chain> {
		self.chains.get(network).ok_or_else(|| anyhow!("unknown network {network}"))
	}

	/** The chain with this ISMP state machine id, if the indexer follows it. */
	pub fn chain_by_state_machine(&self, state_machine_id: &str) -> Option<&Chain> {
		self.chains.values().find(|c| c.state_machine_id() == state_machine_id)
	}
}

/**
 * Connects, migrates, and indexes `selected` chains (all when empty) until shutdown. rindexer
 * finishes every chain's history before any chain goes live, so one process per chain keeps a
 * slow or stuck chain from holding back the others.
 */
pub async fn run(config_path: &Path, run_dir: &Path, selected: &[String]) -> anyhow::Result<()> {
	let config = Config::load(config_path)?;
	for name in selected {
		if !config.chains.contains_key(name) {
			return Err(anyhow!("--chains: {name} is not in {}", config_path.display()));
		}
	}
	let indexed: Vec<&String> = config
		.chains
		.keys()
		.filter(|n| selected.is_empty() || selected.contains(n))
		.collect();

	// Each chain's batch holds a connection for its transaction, and rindexer's default pool
	// is 10. Leave room for each chain's upkeep, the flusher and the watchlist.
	if std::env::var_os("DATABASE_POOL_SIZE").is_none() {
		std::env::set_var("DATABASE_POOL_SIZE", (indexed.len() * 2 + 4).max(10).to_string());
	}
	let db = Arc::new(PostgresClient::new().await.context("connecting to DATABASE_URL")?);
	// A chain rindexer has no progress for, as after a SubQuery import, starts at its cursor.
	// rindexer checks the manifest's start block against the node's head before it reads its
	// own progress, so a chain with progress keeps the config's, far behind any head.
	let cursors = {
		let mut conn = db.raw_connection().await?;
		db::apply_migrations(&mut conn).await?;
		let cursors = db::cursor_blocks(&conn).await?;
		let tx = conn.transaction().await?;
		let mut unsynced = BTreeMap::new();
		for (network, block) in cursors {
			if registry::rindexer_progress(&tx, &network).await?.is_none() {
				unsynced.insert(network, block);
			}
		}
		unsynced
	};

	let http = reqwest::Client::builder().build()?;
	let mut chains = BTreeMap::new();
	for name in indexed {
		chains.insert(name.clone(), Chain::connect(name, &config.chains[name], &http)?);
	}
	// rindexer's manifest names one RPC per chain. It builds no client from it: every call goes
	// through `AllTopicsProvider`, which fails over between all of them.
	let rpc_urls: BTreeMap<String, String> = chains
		.iter()
		.map(|(name, chain)| (name.clone(), chain.rpc_urls[0].clone()))
		.collect();

	let handlers = handlers::all();
	let routes: HashMap<String, Arc<registry::ChainRoutes>> = config
		.chains
		.iter()
		.filter(|(name, _)| chains.contains_key(*name))
		.map(|(name, chain)| {
			let routes =
				registry::ChainRoutes::new(chain, &handlers.subscriptions, &handlers.hooks);
			(name.clone(), Arc::new(routes))
		})
		.collect();

	let (manifest_path, manifest) =
		manifest::write(run_dir, &config, &rpc_urls, &cursors, &routes)?;
	db::migrate(&db, &manifest_path, &manifest).await?;

	let mut providers: HashMap<String, Arc<dyn ChainProvider>> = HashMap::new();
	for network in &manifest.networks {
		let chain = &chains[&network.name];
		let mut clients = vec![];
		let mut failure = None;
		for (position, url) in chain.rpc_urls.iter().enumerate() {
			let client = create_client(
				url,
				network.chain_id,
				network.compute_units_per_second,
				network.max_block_range,
				network.block_poll_frequency,
				HeaderMap::new(),
				None,
				None,
			);
			// rindexer's client checks the node's chain id when it's built, and retries a 503
			// for an hour. With several URLs, a node that doesn't answer in time is left out
			// until the next start; with one, the start waits for it. A node of another chain
			// is a mistake in the config.
			let client = if chain.rpc_urls.len() > 1 {
				tokio::time::timeout(CLIENT_TIMEOUT, client).await
			} else {
				Ok(client.await)
			};
			match client {
				Ok(Ok(client)) => clients.push((position, client)),
				Ok(Err(e @ RetryClientError::InvalidClientChainId(..))) =>
					return Err(anyhow!("rpc client for {}: {e}", network.name)),
				Ok(Err(e)) => {
					tracing::warn!(network = %network.name, rpc = position, "RPC left out: {e}");
					failure = Some(e.to_string());
				},
				Err(_) => {
					let e = format!("no answer in {CLIENT_TIMEOUT:?}");
					tracing::warn!(network = %network.name, rpc = position, "RPC left out: {e}");
					failure = Some(e);
				},
			}
		}
		if clients.is_empty() {
			let failure = failure.unwrap_or_default();
			return Err(anyhow!("rpc client for {}: {failure}", network.name));
		}
		let topics = routes[&network.name].topics().into_iter().collect();
		let distance = chain.config.reorg_safe_distance;
		providers.insert(
			network.name.clone(),
			Arc::new(provider::AllTopicsProvider::new(clients, topics, distance)),
		);
	}

	let indexer = Arc::new(Indexer {
		config,
		db,
		chains,
		orderbook: services::prices::Orderbook::from_env(http.clone()),
		http,
	});
	handlers::spawn_tasks(&indexer);
	let mut registry = EventCallbackRegistry::new();
	registry::register_all(
		&indexer,
		&manifest,
		&manifest_path,
		&routes,
		&providers,
		&mut registry,
	)?;
	registry::spawn_upkeep(indexer.clone(), &routes);

	start_rindexer(StartDetails {
		manifest_path: &manifest_path,
		indexing_details: Some(IndexingDetails {
			registry,
			trace_registry: TraceCallbackRegistry { events: vec![], on_reorg: vec![] },
			event_stream: None,
		}),
		graphql_details: GraphqlOverrideSettings { enabled: false, override_port: None },
		cron_scheduler_handle: None,
		watch: false,
	})
	.await
	.map_err(|e| anyhow!("rindexer: {e:?}"))
}

/**
 * Turns the copy of SubQuery's database at `DATABASE_URL` into this indexer's, so each chain
 * continues after the last block SubQuery indexed. `new_chains` are the chains SubQuery never
 * indexed, which start at their start block. A Substrate chain's node, at `<CHAIN>_SUBSTRATE`,
 * gives the name SubQuery's metadata knows it by.
 */
pub async fn import_subquery(config_path: &Path, new_chains: &[String]) -> anyhow::Result<()> {
	let config = Config::load(config_path)?;
	let http = reqwest::Client::builder().build()?;
	let mut names = BTreeMap::new();
	for (network, chain) in &config.chains {
		let node = match chain.kind {
			ChainKind::Substrate => {
				let urls = env_urls(&format!("{}_SUBSTRATE", env_key(network)))?;
				Some(SubstrateRpc::new(&urls, http.clone()))
			},
			ChainKind::Evm => None,
		};
		let name = metadata::name(chain, node.as_ref()).await.with_context(|| network.clone())?;
		names.insert(network.clone(), name);
	}
	// A pool of one, so every other session the import sees is another client's.
	std::env::set_var("DATABASE_POOL_SIZE", "1");
	let db = PostgresClient::new().await.context("connecting to DATABASE_URL")?;
	let mut conn = db.raw_connection().await?;
	let imported = subquery::import(&mut conn, &config, &names, new_chains).await?;
	for (table, rows) in &imported.rows {
		tracing::info!(table, rows, "copied");
	}
	for (network, block) in &imported.cursors {
		tracing::info!(%network, block, "continues after SubQuery's last block");
	}
	Ok(())
}
