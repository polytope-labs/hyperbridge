pub mod chains;
pub mod config;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod manifest;
pub mod metadata;
pub mod provider;
pub mod registry;
pub mod services;
pub mod store;
pub mod substrate;

use std::{
	collections::{BTreeMap, HashMap},
	path::Path,
	sync::Arc,
};

use anyhow::{anyhow, Context};
use rindexer::{
	event::callback_registry::{EventCallbackRegistry, TraceCallbackRegistry},
	provider::{create_client, ChainProvider},
	start_rindexer, GraphqlOverrideSettings, HeaderMap, IndexingDetails, PostgresClient,
	StartDetails,
};

use crate::{chains::Chain, config::Config};

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

	let http = reqwest::Client::builder().build()?;
	let mut chains = BTreeMap::new();
	for name in indexed {
		chains.insert(name.clone(), Chain::connect(name, &config.chains[name], &http)?);
	}
	let rpc_urls: BTreeMap<String, String> = chains
		.iter()
		.map(|(name, chain)| (name.clone(), chain.rpc_url.clone()))
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

	let (manifest_path, manifest) = manifest::write(run_dir, &config, &rpc_urls, &routes)?;
	db::migrate(&db, &manifest_path, &manifest).await?;

	let mut providers: HashMap<String, Arc<dyn ChainProvider>> = HashMap::new();
	for network in &manifest.networks {
		let inner = create_client(
			&network.rpc,
			network.chain_id,
			network.compute_units_per_second,
			network.max_block_range,
			network.block_poll_frequency,
			HeaderMap::new(),
			None,
			None,
		)
		.await
		.map_err(|e| anyhow!("rpc client for {}: {e}", network.name))?;
		let topics = routes[&network.name].topics().into_iter().collect();
		let distance = chains[&network.name].config.reorg_safe_distance;
		providers.insert(
			network.name.clone(),
			Arc::new(provider::AllTopicsProvider::new(inner, topics, distance)),
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
