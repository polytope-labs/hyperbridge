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
	config::{env_key, env_urls, ChainConfig, ChainKind},
	errors::is_failed_execution,
	failover::FailoverTransport,
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
	/** The chain's Ethereum JSON-RPC URLs, in the order reads fail over between them: the
	 * chain's own, or substrate-eth-rpc's for a Substrate chain. */
	pub rpc_urls: Vec<String>,
	pub provider: DynProvider,
	/** For Substrate chains, the node's own RPCs, from `<CHAIN>_SUBSTRATE`. A standalone
	 * substrate-eth-rpc serves only `eth_*`, so there is no default. */
	pub substrate: Option<SubstrateRpc>,
}

impl Chain {
	pub fn connect(
		name: &str,
		config: &ChainConfig,
		http: &reqwest::Client,
	) -> anyhow::Result<Self> {
		let rpc_urls = config.rpc_urls(name)?;
		let provider = reader(&rpc_urls)?;
		let substrate = match config.kind {
			ChainKind::Substrate => {
				let urls = env_urls(&format!("{}_SUBSTRATE", env_key(name)))?;
				Some(SubstrateRpc::new(&urls, http.clone()))
			},
			ChainKind::Evm => None,
		};
		Ok(Self { name: name.to_string(), config: config.clone(), rpc_urls, provider, substrate })
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

/**
 * The client handlers read with: every request times out and fails over between the URLs, and
 * rate limits are retried.
 */
fn reader(urls: &[String]) -> anyhow::Result<DynProvider> {
	// alloy's own reqwest, which its HTTP transport takes.
	let http = alloy::transports::http::reqwest::Client::builder()
		.timeout(REQUEST_TIMEOUT)
		.build()?;
	let transports = urls
		.iter()
		.map(|url| Ok(Http::with_client(http.clone(), url.parse()?)))
		.collect::<anyhow::Result<Vec<_>>>()?;
	let retries = RetryBackoffLayer::new_with_policy(
		RATE_LIMIT_RETRIES,
		RETRY_BACKOFF_MS,
		COMPUTE_UNITS_PER_SECOND,
		RateLimits,
	);
	let client = ClientBuilder::default()
		.layer(retries)
		.transport(FailoverTransport::new(transports), false);
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
