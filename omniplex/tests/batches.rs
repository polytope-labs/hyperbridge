//! A batch commits block by block. Set `INDEXER_TEST_DATABASE_URL` to run.

// Only `database` is used here.
#[allow(dead_code)]
mod common;

use std::{
	collections::BTreeMap,
	sync::{
		atomic::{AtomicU64, AtomicUsize, Ordering},
		Arc, Mutex,
	},
};

use alloy::{
	primitives::{Address, Bytes, LogData, B256, U256, U64},
	providers::{Provider, ProviderBuilder},
	sol,
	sol_types::SolEvent,
};
use async_trait::async_trait;
use omniplex::{
	chains::Chain,
	config::{ChainConfig, ChainKind, Config, Contracts, Environment},
	errors::Retry,
	registry::{
		process_batch, BlockContext, BlockHook, ChainRoutes, EventContext, EventHandler, Source,
		Subscription,
	},
	services::prices::Orderbook,
	Indexer,
};
use rindexer::{
	event::callback_registry::{EventResult, LogFoundInRequest, TxInformation},
	PostgresClient,
};

sol! {
	event Ping();
}

const HOST: Address = Address::repeat_byte(0x11);

/** Records the logs it handles, and fails the first log of `fail_at` while it is set. */
#[derive(Clone, Default)]
struct Recorder {
	handled: Arc<Mutex<Vec<(u64, u64)>>>,
	fail_at: Arc<AtomicU64>,
}

#[async_trait]
impl EventHandler for Recorder {
	type Event = Ping;

	async fn handle(&self, ctx: &EventContext<'_>, _: Ping) -> anyhow::Result<()> {
		let position = (ctx.log.block_number, ctx.log.log_index);
		self.handled.lock().unwrap().push(position);
		if self.fail_at.load(Ordering::SeqCst) == ctx.log.block_number {
			return Err(Retry("the RPC failed".into()).into());
		}
		Ok(())
	}
}

/** Counts its runs, and keeps the blocks of the last. */
#[derive(Clone, Default)]
struct Counter {
	runs: Arc<AtomicUsize>,
	block: Arc<AtomicU64>,
	previous_block: Arc<Mutex<Option<u64>>>,
}

#[async_trait]
impl BlockHook for Counter {
	fn applies_to(&self, _: &ChainConfig) -> bool {
		true
	}

	async fn after_batch(&self, ctx: &BlockContext<'_>) -> anyhow::Result<()> {
		self.runs.fetch_add(1, Ordering::SeqCst);
		self.block.store(ctx.block_number, Ordering::SeqCst);
		*self.previous_block.lock().unwrap() = ctx.previous_block;
		Ok(())
	}
}

/** A log of `block`, from a fetch of `range`. */
fn result(network: &str, range: (u64, u64), block: u64, log_index: u64) -> EventResult {
	let mut log = alloy::rpc::types::Log::default();
	log.inner = alloy::primitives::Log {
		address: HOST,
		data: LogData::new_unchecked(vec![Ping::SIGNATURE_HASH], Bytes::new()),
	};
	EventResult {
		log,
		decoded_data: Arc::new(()),
		tx_information: TxInformation {
			chain_id: 1,
			network: network.to_string(),
			address: HOST,
			block_hash: B256::repeat_byte(block as u8),
			block_number: block,
			block_timestamp: Some(U256::from(1_750_000_000 + block)),
			transaction_hash: B256::repeat_byte(0xaa),
			log_index: U256::from(log_index),
			transaction_index: 0,
		},
		found_in_request: LogFoundInRequest {
			from_block: U64::from(range.0),
			to_block: U64::from(range.1),
		},
	}
}

/** An indexer of one EVM chain named `network`, on `INDEXER_TEST_DATABASE_URL`. */
async fn test_indexer(network: &str) -> (Indexer, ChainConfig) {
	// As in `main`: rindexer connects to `DATABASE_URL`, over rustls.
	let _ = rustls::crypto::ring::default_provider().install_default();
	std::env::set_var("DATABASE_URL", std::env::var("INDEXER_TEST_DATABASE_URL").unwrap());
	let chain_config = ChainConfig {
		kind: ChainKind::Evm,
		chain_id: 1,
		state_machine_id: "EVM-1".into(),
		start_block: 0,
		reorg_safe_distance: 0,
		max_block_range: None,
		op_stack: false,
		contracts: Contracts { ethereum_host: Some(HOST), ..Default::default() },
	};
	let chain = Chain {
		name: network.to_string(),
		config: chain_config.clone(),
		rpc_urls: vec!["http://127.0.0.1:9".into()],
		provider: ProviderBuilder::new()
			.connect_http("http://127.0.0.1:9".parse().unwrap())
			.erased(),
		substrate: None,
	};
	let http = reqwest::Client::new();
	let indexer = Indexer {
		config: Config {
			environment: Environment::Testnet,
			chains: BTreeMap::from([(network.to_string(), chain_config.clone())]),
		},
		db: Arc::new(PostgresClient::new().await.unwrap()),
		chains: BTreeMap::from([(network.to_string(), chain)]),
		orderbook: Orderbook::from_env(http.clone()),
		http,
	};
	(indexer, chain_config)
}

/**
 * Writes a row for each log. At `lasting_at` it then fails for good, and at `nul_at` it writes
 * text Postgres refuses: neither would succeed on a retry.
 */
#[derive(Clone)]
struct Writer {
	lasting_at: u64,
	nul_at: u64,
}

#[async_trait]
impl EventHandler for Writer {
	type Event = Ping;

	async fn handle(&self, ctx: &EventContext<'_>, _: Ping) -> anyhow::Result<()> {
		let block = ctx.log.block_number;
		let note = if block == self.nul_at { "a\0b" } else { "written" };
		ctx.db
			.execute(
				"INSERT INTO public.batch_writes (network, block, note) VALUES ($1, $2, $3)",
				&[&ctx.chain.name, &(block as i64), &note],
			)
			.await?;
		if block == self.lasting_at {
			anyhow::bail!("this log's data can never apply");
		}
		Ok(())
	}
}

/** A handler that fails for good loses its own writes, not the batch: the chain goes on. */
#[tokio::test]
async fn a_log_that_can_never_apply_is_skipped_with_its_writes() {
	let Some(admin) = common::database().await else { return };
	let network = format!("batch-skip-{}", std::process::id());
	let (indexer, chain_config) = test_indexer(&network).await;
	admin
		.batch_execute(
			"CREATE TABLE IF NOT EXISTS public.batch_writes (network text, block bigint, note text)",
		)
		.await
		.unwrap();
	let writer = Writer { lasting_at: 2, nul_at: 3 };
	let routes =
		ChainRoutes::new(&chain_config, &[Subscription::new(Source::EthereumHost, writer)], &[]);
	let batch =
		[(1, 0), (2, 0), (3, 0), (4, 0)].map(|(block, log)| result(&network, (1, 4), block, log));
	process_batch(&indexer, &network, &routes, batch.into()).await.unwrap();
	let written = admin
		.query(
			"SELECT block FROM public.batch_writes WHERE network = $1 ORDER BY block",
			&[&network],
		)
		.await
		.unwrap()
		.iter()
		.map(|row| row.get::<_, i64>(0))
		.collect::<Vec<_>>();
	assert_eq!(written, [1, 4]);
	let cursor = admin
		.query_one("SELECT block_number FROM indexer.cursors WHERE network = $1", &[&network])
		.await
		.unwrap()
		.get::<_, i64>(0);
	assert_eq!(cursor, 4);
	admin
		.execute("DELETE FROM public.batch_writes WHERE network = $1", &[&network])
		.await
		.unwrap();
	admin
		.execute("DELETE FROM indexer.cursors WHERE network = $1", &[&network])
		.await
		.unwrap();
}

#[tokio::test]
async fn a_failed_block_keeps_the_blocks_before_it_and_the_replay_runs_hooks_once() {
	let Some(admin) = common::database().await else { return };
	let network = format!("batch-test-{}", std::process::id());
	let (indexer, chain_config) = test_indexer(&network).await;
	let recorder = Recorder::default();
	let counter = Counter::default();
	let routes = ChainRoutes::new(
		&chain_config,
		&[Subscription::new(Source::EthereumHost, recorder.clone())],
		&[Arc::new(counter.clone())],
	);
	let batch = || {
		[(1, 0), (1, 1), (2, 0), (3, 0), (4, 0)]
			.map(|(block, log)| result(&network, (1, 4), block, log))
	};
	let cursor = || async {
		let row = admin
			.query_opt(
				"SELECT block_number, log_index FROM indexer.cursors WHERE network = $1",
				&[&network],
			)
			.await
			.unwrap();
		row.map(|row| (row.get::<_, i64>(0), row.get::<_, i64>(1)))
	};
	let handled = || std::mem::take(&mut *recorder.handled.lock().unwrap());

	recorder.fail_at.store(3, Ordering::SeqCst);
	let failed = process_batch(&indexer, &network, &routes, batch().into()).await;
	assert!(failed.is_err());
	assert_eq!(handled(), [(1, 0), (1, 1), (2, 0), (3, 0)]);
	assert_eq!(cursor().await, Some((2, 0)));
	assert_eq!(counter.runs.load(Ordering::SeqCst), 0);

	// rindexer retries the batch: blocks 1 and 2 committed, so only 3 and 4 run.
	recorder.fail_at.store(0, Ordering::SeqCst);
	process_batch(&indexer, &network, &routes, batch().into()).await.unwrap();
	assert_eq!(handled(), [(3, 0), (4, 0)]);
	assert_eq!(cursor().await, Some((4, 0)));
	assert_eq!(counter.runs.load(Ordering::SeqCst), 1);
	assert_eq!(counter.block.load(Ordering::SeqCst), 4);
	// The hooks still see the whole batch, though blocks 1 and 2 committed in the failed run.
	assert_eq!(*counter.previous_block.lock().unwrap(), Some(0));

	// A replay of a batch that committed runs nothing again, hooks included.
	process_batch(&indexer, &network, &routes, batch().into()).await.unwrap();
	assert!(handled().is_empty());
	assert_eq!(counter.runs.load(Ordering::SeqCst), 1);

	// Hooks see where the batch started, though each of its blocks has committed by then.
	let next = [(5, 0), (6, 0), (7, 0)].map(|(block, log)| result(&network, (5, 7), block, log));
	process_batch(&indexer, &network, &routes, next.into()).await.unwrap();
	assert_eq!(handled(), [(5, 0), (6, 0), (7, 0)]);
	assert_eq!(counter.block.load(Ordering::SeqCst), 7);
	assert_eq!(*counter.previous_block.lock().unwrap(), Some(4));

	admin
		.execute("DELETE FROM indexer.cursors WHERE network = $1", &[&network])
		.await
		.unwrap();
}
