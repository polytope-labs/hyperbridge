//! The Hyperbridge side of solver discovery (`solverWatchlist.service.ts`): poll the HyperFX
//! orderbook's watchlist and queue every solver it has not asked about before, for the chain's
//! inventory hook to start tracking. This closes the cold start fill-based discovery leaves open:
//! a solver that has never filled has no inventory row, so the orderbook cannot validate its
//! orders.

use std::{
	collections::HashSet,
	sync::Arc,
	time::{Duration, Instant},
};

use alloy::{eips::BlockNumberOrTag, providers::Provider};
use chrono::NaiveDateTime;
use reqwest::{header, StatusCode};
use serde_json::Value;
use tokio_postgres::Transaction;

use crate::{
	services::prices::orderbook_endpoint,
	store::{numeric, timestamp_to_date},
	Indexer,
};

/** How often the poll runs: one Hyperbridge block. SubQuery polled on every block. */
pub const POLL_INTERVAL: Duration = Duration::from_secs(6);
/** A slow orderbook must not hold a poll up for long. */
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
/** Solvers accepted per chain per response; the orderbook caps its own list the same way. */
pub const MAX_WATCHLIST_PER_CHAIN: usize = 5_000;
const SKIP_REPORT_INTERVAL: Duration = Duration::from_secs(60);

fn is_address(value: &str) -> bool {
	value.len() == 42 &&
		value.starts_with("0x") &&
		value[2..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/**
 * `parseWatchlist`: `{ chains: [{ chain, solvers: [{ address }] }] }` as lowercase,
 * de-duplicated addresses per chain `tracks` accepts, capped per chain. None when the body is
 * not that shape. A chain listed twice keeps its first position and its last list.
 */
pub fn parse_watchlist(
	body: &Value,
	tracks: impl Fn(&str) -> bool,
) -> Option<Vec<(String, Vec<String>)>> {
	let chains = body.get("chains")?.as_array()?;
	let mut parsed: Vec<(String, Vec<String>)> = vec![];
	for entry in chains {
		let chain = entry.get("chain").and_then(Value::as_str)?;
		let solvers = entry.get("solvers").and_then(Value::as_array)?;
		if !tracks(chain) {
			continue;
		}
		let mut seen = HashSet::new();
		let mut addresses = vec![];
		for solver in solvers {
			let Some(address) = solver.get("address").and_then(Value::as_str) else { continue };
			let address = address.to_lowercase();
			if is_address(&address) && seen.insert(address.clone()) {
				addresses.push(address);
			}
		}
		if addresses.len() > MAX_WATCHLIST_PER_CHAIN {
			tracing::warn!(
				"[solver-watchlist] {chain} lists {} solvers; ignoring all past the first {MAX_WATCHLIST_PER_CHAIN}",
				addresses.len()
			);
			addresses.truncate(MAX_WATCHLIST_PER_CHAIN);
		}
		match parsed.iter_mut().find(|(c, _)| c == chain) {
			Some(slot) => slot.1 = addresses,
			None => parsed.push((chain.to_string(), addresses)),
		}
	}
	Some(parsed)
}

/** `queueRequests`: the solvers not yet requested for `chain`, as one new watchlist version. */
pub async fn queue_requests(
	db: &Transaction<'_>,
	chain: &str,
	solvers: &[String],
	block_number: u64,
	at: NaiveDateTime,
) -> anyhow::Result<()> {
	let known: HashSet<String> = db
		.query(
			"SELECT solver FROM app.solver_discovery_requests WHERE chain = $1 ORDER BY id ASC",
			&[&chain],
		)
		.await?
		.iter()
		.map(|row| row.get(0))
		.collect();
	let fresh: Vec<&String> = solvers.iter().filter(|s| !known.contains(*s)).collect();
	if fresh.is_empty() {
		return Ok(());
	}
	let version = db
		.query_opt("SELECT version FROM app.solver_watchlists WHERE id = $1", &[&chain])
		.await?
		.map_or(0, |row| row.get::<_, i32>(0)) +
		1;
	for solver in &fresh {
		db.execute(
			"INSERT INTO app.solver_discovery_requests (id, chain, solver, version, block_number, requested_at)
			 VALUES ($1, $2, $3, $4, $5, $6)
			 ON CONFLICT (id) DO UPDATE SET chain = EXCLUDED.chain, solver = EXCLUDED.solver,
			   version = EXCLUDED.version, block_number = EXCLUDED.block_number,
			   requested_at = EXCLUDED.requested_at",
			&[&format!("{chain}-{solver}"), &chain, solver, &version, &numeric(block_number), &at],
		)
		.await?;
	}
	db.execute(
		"INSERT INTO app.solver_watchlists (id, chain, version, request_count, block_number, updated_at)
		 VALUES ($1, $1, $2, $3, $4, $5)
		 ON CONFLICT (id) DO UPDATE SET chain = EXCLUDED.chain, version = EXCLUDED.version,
		   request_count = EXCLUDED.request_count, block_number = EXCLUDED.block_number,
		   updated_at = EXCLUDED.updated_at",
		&[&chain, &version, &((known.len() + fresh.len()) as i32), &numeric(block_number), &at],
	)
	.await?;
	Ok(())
}

/** What one fetch of the watchlist returned. */
#[derive(Debug, PartialEq, Eq)]
pub enum Fetched {
	Unchanged,
	/** Parsed chains, and the response's ETag. */
	Watchlist(Vec<(String, Vec<String>)>, Option<String>),
	/** Logged; the next poll retries. */
	Failed,
}

/** The poll's state: the ETag of the last response fully applied. */
#[derive(Default)]
pub struct Poller {
	pub etag: Option<String>,
	last_skip_report: Option<Instant>,
}

impl Poller {
	/** Says why a poll did nothing, at most once a minute. */
	fn report_skip(&mut self, reason: &str) {
		if self.last_skip_report.is_some_and(|at| at.elapsed() < SKIP_REPORT_INTERVAL) {
			return;
		}
		self.last_skip_report = Some(Instant::now());
		tracing::info!("[solver-watchlist] Not polling: {reason}");
	}

	/** One GET of the watchlist with the last ETag. Never fails for the orderbook's sake. */
	pub async fn fetch(
		&self,
		http: &reqwest::Client,
		url: &reqwest::Url,
		tracks: impl Fn(&str) -> bool,
	) -> Fetched {
		let mut request = http
			.get(url.clone())
			.header(header::ACCEPT, "application/json")
			.timeout(FETCH_TIMEOUT);
		if let Some(etag) = &self.etag {
			request = request.header(header::IF_NONE_MATCH, etag);
		}
		let response = match request.send().await {
			Ok(response) => response,
			Err(err) => {
				tracing::warn!(
					"[solver-watchlist] Fetching {url} failed; retrying next block: {err}"
				);
				return Fetched::Failed;
			},
		};
		if response.status() == StatusCode::NOT_MODIFIED {
			return Fetched::Unchanged;
		}
		if !response.status().is_success() {
			tracing::warn!(
				"[solver-watchlist] {url} answered {}; retrying next block",
				response.status().as_u16()
			);
			return Fetched::Failed;
		}
		let etag = response
			.headers()
			.get(header::ETAG)
			.and_then(|v| v.to_str().ok())
			.map(str::to_string);
		let body: Value = match response.json().await {
			Ok(body) => body,
			Err(err) => {
				tracing::warn!(
					"[solver-watchlist] Fetching {url} failed; retrying next block: {err}"
				);
				return Fetched::Failed;
			},
		};
		match parse_watchlist(&body, tracks) {
			Some(chains) => Fetched::Watchlist(chains, etag),
			None => {
				tracing::warn!(
					"[solver-watchlist] {url} returned an unrecognised body; retrying next block"
				);
				Fetched::Failed
			},
		}
	}

	/**
	 * `pollSolverWatchlist`: fetches, queues each chain's new solvers in one transaction, and
	 * keeps the ETag only once everything is committed, so a failed write is fetched again.
	 * `block` is the Hyperbridge block and time (seconds) the requests are recorded at.
	 */
	pub async fn poll(
		&mut self,
		conn: &mut tokio_postgres::Client,
		http: &reqwest::Client,
		url: &reqwest::Url,
		tracks: impl Fn(&str) -> bool,
		block: (u64, u64),
	) -> anyhow::Result<()> {
		let (chains, etag) = match self.fetch(http, url, tracks).await {
			Fetched::Watchlist(chains, etag) => (chains, etag),
			Fetched::Unchanged | Fetched::Failed => return Ok(()),
		};
		let at = timestamp_to_date(block.1);
		let tx = conn.transaction().await?;
		for (chain, solvers) in &chains {
			queue_requests(&tx, chain, solvers, block.0, at).await?;
		}
		tx.commit().await?;
		self.etag = etag;
		Ok(())
	}
}

/**
 * Polls the watchlist every POLL_INTERVAL. SubQuery polled from the Hyperbridge node's block
 * handler, recording the block it ran at; this reads Hyperbridge's finalized head for the same
 * fields. Its live-slack guard only stopped a resync from replaying fetches, so it is gone.
 */
pub fn spawn_poller(indexer: Arc<Indexer>) {
	tokio::spawn(async move {
		let mut poller = Poller::default();
		let mut interval = tokio::time::interval(POLL_INTERVAL);
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
		loop {
			interval.tick().await;
			if let Err(err) = poll_once(&indexer, &mut poller).await {
				tracing::warn!("[solver-watchlist] Poll failed; retrying next block: {err:#}");
			}
		}
	});
}

async fn poll_once(indexer: &Indexer, poller: &mut Poller) -> anyhow::Result<()> {
	let configured = std::env::var("HYPERFX_ORDERBOOK_URL").ok().filter(|v| !v.is_empty());
	let Some(configured) = configured else {
		poller.report_skip("HYPERFX_ORDERBOOK_URL is not set");
		return Ok(());
	};
	let Some(url) = orderbook_endpoint(&configured, "solvers") else {
		poller.report_skip(&format!("HYPERFX_ORDERBOOK_URL is not a URL: {configured}"));
		return Ok(());
	};
	let hyperbridge = indexer.config.environment.hyperbridge();
	let Some(chain) = indexer.chain_by_state_machine(hyperbridge) else {
		poller.report_skip(&format!("the indexer does not follow {hyperbridge}"));
		return Ok(());
	};
	let head = chain
		.provider
		.get_block_by_number(BlockNumberOrTag::Finalized)
		.await?
		.ok_or_else(|| anyhow::anyhow!("{hyperbridge} has no finalized block"))?;
	let block = (head.header.number, head.header.timestamp);

	// From config, not the connected chains: a process may index only some of them.
	let tracks =
		|state_machine: &str| {
			indexer.config.chains.values().any(|c| {
				c.state_machine_id == state_machine && !c.contracts.yield_vaults.is_empty()
			})
		};
	let mut conn = indexer.db.raw_connection().await?;
	poller.poll(&mut conn, &indexer.http, &url, tracks, block).await
}

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;

	const A: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
	const B: &str = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

	fn tracks(chain: &str) -> bool {
		chain == "EVM-8453" || chain == "EVM-42161"
	}

	#[test]
	fn parses_tracked_chains_lowercased_and_deduplicated() {
		let body = json!({ "chains": [
			{ "chain": "EVM-8453", "solvers": [{ "address": A }, { "address": B.to_uppercase().replace("0X", "0x") }, { "address": A }] },
			{ "chain": "EVM-999", "solvers": [{ "address": A }] },
		]});
		assert_eq!(
			parse_watchlist(&body, tracks).unwrap(),
			vec![("EVM-8453".to_string(), vec![A.to_string(), B.to_string()])]
		);
	}

	#[test]
	fn drops_malformed_addresses_and_rejects_the_wrong_shape() {
		let body = json!({ "chains": [{ "chain": "EVM-8453", "solvers": [
			{ "address": "0x1234" }, { "address": 42 }, {}, { "address": A },
		]}]});
		assert_eq!(parse_watchlist(&body, tracks).unwrap()[0].1, vec![A.to_string()]);
		assert!(parse_watchlist(&json!({ "solvers": [A] }), tracks).is_none());
		// One malformed entry rejects the whole body, even for a chain that isn't tracked.
		let body = json!({ "chains": [
			{ "chain": "EVM-8453", "solvers": [{ "address": A }] },
			{ "chain": "EVM-999", "solvers": "none" },
		]});
		assert!(parse_watchlist(&body, tracks).is_none());
	}

	#[test]
	fn caps_each_chain() {
		let solvers: Vec<Value> = (0..MAX_WATCHLIST_PER_CHAIN + 3)
			.map(|i| json!({ "address": format!("0x{i:040x}") }))
			.collect();
		let body = json!({ "chains": [{ "chain": "EVM-8453", "solvers": solvers }] });
		assert_eq!(parse_watchlist(&body, tracks).unwrap()[0].1.len(), MAX_WATCHLIST_PER_CHAIN);
	}

	#[test]
	fn a_chain_listed_twice_keeps_the_last_list() {
		let body = json!({ "chains": [
			{ "chain": "EVM-8453", "solvers": [{ "address": A }] },
			{ "chain": "EVM-42161", "solvers": [] },
			{ "chain": "EVM-8453", "solvers": [{ "address": B }] },
		]});
		assert_eq!(
			parse_watchlist(&body, tracks).unwrap(),
			vec![("EVM-8453".to_string(), vec![B.to_string()]), ("EVM-42161".to_string(), vec![]),]
		);
	}
}
