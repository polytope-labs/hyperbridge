//! Token prices from the HyperFX orderbook, Chainlink native prices, and ERC-20 metadata.

use std::{
	collections::HashMap,
	sync::Arc,
	time::{Duration, Instant},
};

use alloy::{
	eips::BlockId,
	primitives::{Address, U256},
	sol,
};
use anyhow::Context;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tokio_postgres::Transaction;

use crate::{
	chains::Chain,
	config::Environment,
	services::decimal::Dec,
	store::{normalize_timestamp, numeric, timestamp_to_date},
};

sol! {
	#[sol(rpc)]
	interface Erc20Metadata {
		function symbol() external view returns (string);
		function decimals() external view returns (uint8);
	}

	#[sol(rpc)]
	interface ChainlinkAggregator {
		function decimals() external view returns (uint8);
		function latestRoundData() external view returns (
			uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound
		);
	}
}

/** Symbols worth exactly $1. The first two are the quote currencies, in the order they're tried. */
const STABLE_QUOTES: [&str; 2] = ["USDC", "USDT"];
const STABLE_ALIASES: [&str; 2] = ["USDT0", "USD₮0"];
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
const PRICE_TTL: Duration = Duration::from_secs(60);
const FAILURE_TTL: Duration = Duration::from_secs(5);
const MAX_SYMBOL_LENGTH: usize = 32;
/** How old a stored `token_prices` row may be before it's priced again. */
const PRICE_REFRESH_INTERVAL_MS: u64 = 600_000;

pub fn is_stable_symbol(symbol: &str) -> bool {
	let upper = symbol.to_uppercase();
	STABLE_QUOTES.contains(&upper.as_str()) || STABLE_ALIASES.contains(&upper.as_str())
}

type Answer = Option<(Option<Dec>, Instant)>;

/** The orderbook's rates, cached per symbol. Callers asking for one symbol share one request. */
pub struct Orderbook {
	url: Option<reqwest::Url>,
	http: reqwest::Client,
	cache: std::sync::Mutex<HashMap<String, Arc<Mutex<Answer>>>>,
}

impl Orderbook {
	/** Reads `HYPERFX_ORDERBOOK_URL`, the host that serves the watchlist and the rates API. */
	pub fn from_env(http: reqwest::Client) -> Self {
		Self::new(std::env::var("HYPERFX_ORDERBOOK_URL").ok(), http)
	}

	pub fn new(configured: Option<String>, http: reqwest::Client) -> Self {
		let url = configured.and_then(|configured| {
			let endpoint = orderbook_endpoint(&configured, "graphql");
			if endpoint.is_none() {
				tracing::warn!("HYPERFX_ORDERBOOK_URL is not a URL: {configured}");
			}
			endpoint
		});
		if url.is_none() {
			tracing::info!(
				"HYPERFX_ORDERBOOK_URL is not set: tokens other than stables go unpriced"
			);
		}
		Self { url, http, cache: Default::default() }
	}

	/** `fetchTokenUsdPrice`: $1 for a stable, else the orderbook's rate, else none. */
	pub async fn token_usd_price(&self, symbol: &str) -> Option<Dec> {
		if is_stable_symbol(symbol) {
			return Some(Dec::one());
		}
		self.orderbook_usd_price(symbol).await
	}

	async fn orderbook_usd_price(&self, symbol: &str) -> Option<Dec> {
		let url = self.url.as_ref()?;
		if symbol.is_empty() || symbol.chars().count() > MAX_SYMBOL_LENGTH {
			return None;
		}
		let slot = self.cache.lock().unwrap().entry(symbol.to_string()).or_default().clone();
		let mut answer = slot.lock().await;
		if let Some((price, expires)) = answer.as_ref() {
			if *expires > Instant::now() {
				return price.clone();
			}
		}
		let (price, failed) = self.fetch(url, symbol).await;
		let ttl = if failed { FAILURE_TTL } else { PRICE_TTL };
		*answer = Some((price.clone(), Instant::now() + ttl));
		price
	}

	/** One request. Returns the price, and whether the orderbook failed to answer at all. */
	async fn fetch(&self, url: &reqwest::Url, symbol: &str) -> (Option<Dec>, bool) {
		let query = format!(
			"query Rates($symbol: String!) {{ {} }}",
			STABLE_QUOTES
				.iter()
				.enumerate()
				.map(|(index, stable)| format!(
					"q{index}: bestRate(tokenIn: $symbol, tokenOut: \"{stable}\") {{ base quote rate }}"
				))
				.collect::<Vec<_>>()
				.join(" ")
		);
		let sent = self
			.http
			.post(url.clone())
			.header("accept", "application/json")
			.timeout(FETCH_TIMEOUT)
			.json(&json!({ "query": query, "variables": { "symbol": symbol } }))
			.send()
			.await;
		let body: Value = match sent {
			Ok(response) if response.status().is_success() => match response.json().await {
				Ok(body) => body,
				Err(err) => {
					tracing::warn!("orderbook: pricing {symbol} failed; retrying later: {err}");
					return (None, true);
				},
			},
			Ok(response) => {
				tracing::warn!(
					"orderbook answered {} pricing {symbol}; retrying later",
					response.status()
				);
				return (None, true);
			},
			Err(err) => {
				tracing::warn!("orderbook: pricing {symbol} failed; retrying later: {err}");
				return (None, true);
			},
		};
		let Some(envelope) =
			body.as_object().filter(|o| o.contains_key("data") || o.contains_key("errors"))
		else {
			tracing::warn!(
				"orderbook returned an unrecognised body pricing {symbol}; retrying later"
			);
			return (None, true);
		};
		for (index, stable) in STABLE_QUOTES.iter().enumerate() {
			let entry = envelope.get("data").and_then(|data| data.get(format!("q{index}")));
			if let Some(price) = entry.and_then(|entry| usd_price(entry, stable)) {
				return (Some(price), false);
			}
		}
		if let Some(errors) = envelope.get("errors").and_then(Value::as_array) {
			let messages: Vec<&str> =
				errors.iter().filter_map(|e| e.get("message").and_then(Value::as_str)).collect();
			if !messages.is_empty() {
				tracing::info!("orderbook priced no {symbol}: {}", messages.join("; "));
			}
		}
		(None, false)
	}
}

/** One `bestRate` answer as USD per whole token. `rate` is quote per base, at 1e18. */
fn usd_price(entry: &Value, stable: &str) -> Option<Dec> {
	let stable_is_quote = entry.get("quote").and_then(Value::as_str) == Some(stable);
	if !stable_is_quote && entry.get("base").and_then(Value::as_str) != Some(stable) {
		return None;
	}
	let rate = match entry.get("rate")? {
		Value::String(text) => Dec::parse(text).ok()?,
		Value::Number(number) => Dec::from_f64(number.as_f64()?).ok()?,
		_ => return None,
	};
	if !rate.is_positive() {
		return None;
	}
	let wad = Dec::pow10(18);
	if stable_is_quote {
		rate.div(&wad).ok()
	} else {
		wad.div(&rate).ok()
	}
}

/** `orderbookEndpoint`: `path` under the configured host, dropping a trailing endpoint segment. */
pub fn orderbook_endpoint(configured: &str, path: &str) -> Option<reqwest::Url> {
	let mut base = reqwest::Url::parse(configured).ok()?;
	let mut segments: Vec<String> =
		base.path().split('/').filter(|s| !s.is_empty()).map(str::to_string).collect();
	if segments.last().is_some_and(|last| last == "solvers" || last == "graphql") {
		segments.pop();
	}
	let directory =
		if segments.is_empty() { "/".to_string() } else { format!("/{}/", segments.join("/")) };
	base.set_path(&directory);
	base.join(path).ok()
}

/**
 * `TokenPriceService.getPrice`: the stored price while it is under 10 minutes old, otherwise a
 * fresh one from the orderbook, falling back to the stale price. 0 on testnets and on errors.
 * `timestamp` is the block's, in seconds or milliseconds.
 */
pub async fn token_price(
	orderbook: &Orderbook,
	environment: Environment,
	db: &Transaction<'_>,
	symbol: &str,
	timestamp: u64,
) -> anyhow::Result<Dec> {
	if environment == Environment::Testnet {
		return Ok(Dec::zero());
	}
	let stored = db
		.query_opt(
			"SELECT price, last_updated_at::text FROM app.token_prices WHERE id = $1",
			&[&symbol],
		)
		.await?
		.map(|row| (row.get::<_, String>(0), row.get::<_, String>(1)));
	let fresh = |last_updated: &str| {
		let last: u64 = last_updated.parse().unwrap_or(0);
		normalize_timestamp(timestamp).saturating_sub(last) < PRICE_REFRESH_INTERVAL_MS
	};
	match stored {
		Some((price, last_updated)) if fresh(&last_updated) => parse_js_number(&price),
		stored => match refresh_token_price(orderbook, db, symbol, timestamp).await? {
			Some(price) => Ok(price),
			None => match stored {
				Some((price, _)) => parse_js_number(&price),
				None => Ok(Dec::zero()),
			},
		},
	}
}

/** `updateTokenPrices([symbol])`: stores the orderbook's price, or nothing when it has none. */
async fn refresh_token_price(
	orderbook: &Orderbook,
	db: &Transaction<'_>,
	symbol: &str,
	timestamp: u64,
) -> anyhow::Result<Option<Dec>> {
	let Some(price) = orderbook.token_usd_price(symbol).await.filter(Dec::is_positive) else {
		tracing::warn!("the orderbook quotes no price for {symbol}");
		return Ok(None);
	};
	// The TypeScript stored `price.toNumber().toString()`.
	let number: f64 = price.to_string().parse()?;
	let text = Dec::from_f64(number)?.to_string();
	let normalized = numeric(normalize_timestamp(timestamp));
	db.execute(
		"INSERT INTO app.token_prices (id, symbol, currency, price, last_updated_at)
		 VALUES ($1, $1, 'USD', $2, $3)
		 ON CONFLICT (id) DO UPDATE SET price = EXCLUDED.price, last_updated_at = EXCLUDED.last_updated_at",
		&[&symbol, &text, &normalized],
	)
	.await?;
	db.execute(
		"INSERT INTO app.token_price_logs (id, symbol, currency, price, provider, timestamp, created_at)
		 VALUES ($1, $2, 'USD', $3, 'HYPERFX', $4, $5)
		 ON CONFLICT (id) DO UPDATE SET price = EXCLUDED.price, timestamp = EXCLUDED.timestamp,
		   created_at = EXCLUDED.created_at",
		&[&format!("{symbol}-{timestamp}"), &symbol, &text, &normalized, &timestamp_to_date(timestamp)],
	)
	.await?;
	parse_js_number(&text).map(Some)
}

/** `parseFloat` of a stored price, carried back into a decimal the way JavaScript would. */
fn parse_js_number(text: &str) -> anyhow::Result<Dec> {
	let number: f64 = text.parse().with_context(|| format!("stored price {text:?}"))?;
	Dec::from_f64(number)
}

/**
 * `PriceHelper.getNativeCurrencyPrice`: USD per native token at 18 decimals, from the chain's
 * Chainlink feed at `block`. 0 when the chain has no feed.
 */
pub async fn native_currency_price(chain: &Chain, block: u64) -> anyhow::Result<U256> {
	let Some(feed) = chain.config.contracts.native_usd_feed else { return Ok(U256::ZERO) };
	let aggregator = ChainlinkAggregator::new(feed, &chain.provider);
	let at = BlockId::number(block);
	let round = aggregator.latestRoundData().block(at).call().await?;
	let decimals = aggregator.decimals().block(at).call().await?;
	let answer = U256::try_from(round.answer).unwrap_or_default();
	Ok(answer * U256::from(10).pow(U256::from(18u8.saturating_sub(decimals))))
}

/** An ERC-20's `symbol()` and `decimals()` as of `block`. */
pub async fn token_metadata(
	chain: &Chain,
	token: Address,
	block: u64,
) -> anyhow::Result<(String, u8)> {
	let erc20 = Erc20Metadata::new(token, &chain.provider);
	let at = BlockId::number(block);
	let symbol = erc20
		.symbol()
		.block(at)
		.call()
		.await
		.with_context(|| format!("{token} symbol()"))?;
	let decimals = erc20
		.decimals()
		.block(at)
		.call()
		.await
		.with_context(|| format!("{token} decimals()"))?;
	Ok((symbol, decimals))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn endpoint_resolution() {
		let resolve = |url: &str| orderbook_endpoint(url, "graphql").map(|u| u.to_string());
		assert_eq!(resolve("https://ob.example/").as_deref(), Some("https://ob.example/graphql"));
		assert_eq!(
			resolve("https://ob.example/solvers").as_deref(),
			Some("https://ob.example/graphql")
		);
		assert_eq!(
			resolve("https://ob.example/api/graphql").as_deref(),
			Some("https://ob.example/api/graphql")
		);
		assert_eq!(
			resolve("https://ob.example/api").as_deref(),
			Some("https://ob.example/api/graphql")
		);
		assert_eq!(resolve("not a url"), None);
	}

	#[test]
	fn rates_become_usd_prices() {
		let quoted = json!({ "base": "WETH", "quote": "USDC", "rate": "3000000000000000000000" });
		assert_eq!(usd_price(&quoted, "USDC").unwrap().to_string(), "3000");
		let inverted = json!({ "base": "USDC", "quote": "cNGN", "rate": "1500500000000000000000" });
		assert_eq!(usd_price(&inverted, "USDC").unwrap().to_string(), "0.0006664445184938353882");
		let other = json!({ "base": "WETH", "quote": "DAI", "rate": "1" });
		assert!(usd_price(&other, "USDC").is_none());
		assert!(usd_price(&json!({ "base": "X", "quote": "USDC", "rate": "0" }), "USDC").is_none());
	}

	#[test]
	fn stables() {
		assert!(is_stable_symbol("usdc") && is_stable_symbol("USD₮0") && !is_stable_symbol("DAI"));
	}
}
