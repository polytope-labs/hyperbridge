use std::time::Duration;

use alloy::{
	providers::{DynProvider, Provider, ProviderBuilder},
	rpc::client::ClientBuilder,
	transports::{
		http::Http,
		layers::{RateLimitRetryPolicy, RetryBackoffLayer, RetryPolicy},
		TransportError,
	},
};

use crate::{
	config::{env_key, env_url, ChainConfig, ChainKind},
	errors::is_failed_execution,
	substrate::SubstrateRpc,
};

/** A read with no answer in this long fails, so a hung RPC can't hold a batch's transaction
 * open. */
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/** Retries of a rate-limited read, a second or more apart, before its batch fails and retries. */
const RATE_LIMIT_RETRIES: u32 = 10;
const RETRY_BACKOFF_MS: u64 = 1_000;
/** The compute budget rindexer gives its own client's retries. */
const COMPUTE_UNITS_PER_SECOND: u64 = 660;

/** A chain the indexer follows, with clients for the reads handlers make. */
pub struct Chain {
	pub name: String,
	pub config: ChainConfig,
	pub rpc_url: String,
	/** The chain's Ethereum JSON-RPC: the chain itself, or substrate-eth-rpc for Substrate. */
	pub provider: DynProvider,
	/** For Substrate chains, the node's own RPC, from `<CHAIN>_SUBSTRATE`. A standalone
	 * substrate-eth-rpc serves only `eth_*`, so it has no default. */
	pub substrate: Option<SubstrateRpc>,
}

impl Chain {
	pub fn connect(
		name: &str,
		config: &ChainConfig,
		http: &reqwest::Client,
	) -> anyhow::Result<Self> {
		let rpc_url = config.rpc_url(name)?;
		let provider = reader(&rpc_url)?;
		let substrate = match config.kind {
			ChainKind::Substrate => {
				let url = env_url(&format!("{}_SUBSTRATE", env_key(name)))?;
				Some(SubstrateRpc::new(&url, http.clone()))
			},
			ChainKind::Evm => None,
		};
		Ok(Self { name: name.to_string(), config: config.clone(), rpc_url, provider, substrate })
	}

	pub fn state_machine_id(&self) -> &str {
		&self.config.state_machine_id
	}

	pub fn substrate(&self) -> anyhow::Result<&SubstrateRpc> {
		self.substrate
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("{} is not a Substrate chain", self.name))
	}
}

/** The client handlers read with: every request times out, and rate limits are retried. */
fn reader(url: &str) -> anyhow::Result<DynProvider> {
	// alloy's own reqwest, which its HTTP transport takes.
	let http = alloy::transports::http::reqwest::Client::builder()
		.timeout(REQUEST_TIMEOUT)
		.build()?;
	let retries = RetryBackoffLayer::new_with_policy(
		RATE_LIMIT_RETRIES,
		RETRY_BACKOFF_MS,
		COMPUTE_UNITS_PER_SECOND,
		RateLimits,
	);
	let client = ClientBuilder::default()
		.layer(retries)
		.transport(Http::with_client(http, url.parse()?), false);
	Ok(ProviderBuilder::new().connect_client(client).erased())
}

/**
 * alloy's retries of rate limits, except for a failed execution. Its message carries the
 * contract's revert reason, which can say "rate limit" too.
 */
#[derive(Debug, Clone, Copy)]
struct RateLimits;

impl RetryPolicy for RateLimits {
	fn should_retry(&self, err: &TransportError) -> bool {
		!is_failed_execution(err) && RateLimitRetryPolicy::default().should_retry(err)
	}

	fn backoff_hint(&self, err: &TransportError) -> Option<Duration> {
		RateLimitRetryPolicy::default().backoff_hint(err)
	}
}
