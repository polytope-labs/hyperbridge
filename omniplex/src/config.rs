use std::{collections::BTreeMap, path::Path};

use alloy::primitives::Address;
use anyhow::{bail, Context};
use serde::Deserialize;

/** Every chain the indexer follows, keyed by network name (e.g. `bsc-mainnet`). */
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
	pub environment: Environment,
	pub chains: BTreeMap<String, ChainConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
	Mainnet,
	/** Prices read as 0 on testnets, as they did in SubQuery. */
	Testnet,
}

impl Environment {
	/** Hyperbridge's own state machine id. */
	pub fn hyperbridge(&self) -> &'static str {
		match self {
			Environment::Mainnet => "POLKADOT-3367",
			Environment::Testnet => "KUSAMA-4009",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChainKind {
	Evm,
	/** A Substrate chain read through substrate-eth-rpc. */
	Substrate,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainConfig {
	pub kind: ChainKind,
	/** The `eth_chainId` the RPC answers. For Substrate chains, the facade's chain id. */
	pub chain_id: u64,
	/** The ISMP state machine id, e.g. `EVM-56` or `POLKADOT-3367`. */
	pub state_machine_id: String,
	pub start_block: u64,
	/** Blocks behind the head that rindexer waits before delivering logs. */
	pub reorg_safe_distance: u64,
	/** Largest `eth_getLogs` span the RPC accepts. */
	#[serde(default)]
	pub max_block_range: Option<u64>,
	/** An OP-stack chain, whose receipts carry an `l1Fee` paid on top of gas. */
	#[serde(default)]
	pub op_stack: bool,
	#[serde(default)]
	pub contracts: Contracts,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contracts {
	pub ethereum_host: Option<Address>,
	pub handler_v2: Option<Address>,
	pub erc6160ext20: Option<Address>,
	pub intent_gateway_v3: Option<Address>,
	/** Chainlink native-currency/USD feed, for pricing gas. */
	pub native_usd_feed: Option<Address>,
	#[serde(default)]
	pub solver_accounts: Vec<Address>,
	/** Tokens the solver inventory tracks, each with the ERC-4626 vaults that hold it. */
	#[serde(default)]
	pub yield_vaults: BTreeMap<Address, YieldVaults>,
	#[serde(default)]
	pub token_slots: BTreeMap<Address, TokenSlots>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YieldVaults {
	pub description: String,
	#[serde(default)]
	pub vaults: Vec<Address>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenSlots {
	pub description: String,
	pub balance_slot: u64,
	pub allowance_slot: u64,
}

impl Config {
	pub fn load(path: &Path) -> anyhow::Result<Self> {
		let text =
			std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
		let config: Config =
			toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
		for (name, chain) in &config.chains {
			if chain.kind == ChainKind::Evm && chain.contracts.ethereum_host.is_none() {
				bail!("chain {name}: EVM chains need contracts.ethereum_host");
			}
		}
		Ok(config)
	}
}

impl ChainConfig {
	/** The RPC URLs, read from the chain's env var: `bsc-mainnet` reads `BSC_MAINNET`. */
	pub fn rpc_urls(&self, name: &str) -> anyhow::Result<Vec<String>> {
		env_urls(&env_key(name))
	}
}

pub fn env_key(name: &str) -> String {
	name.replace('-', "_").to_uppercase()
}

/**
 * The comma-separated URLs in env var `key`, in the order they're first tried. Reads fail over
 * between them, as SubQuery did: see `failover`.
 */
pub fn env_urls(key: &str) -> anyhow::Result<Vec<String>> {
	let value = std::env::var(key).with_context(|| format!("{key} is not set"))?;
	let urls: Vec<String> = value
		.split(',')
		.map(str::trim)
		.filter(|url| !url.is_empty())
		.map(String::from)
		.collect();
	if urls.is_empty() {
		bail!("{key} is empty");
	}
	Ok(urls)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn an_rpc_variable_lists_urls_in_order() {
		let read = |value: &str| {
			let key = format!("INDEXER_TEST_URL_{}", value.len());
			std::env::set_var(&key, value);
			env_urls(&key).map_err(|e| e.to_string())
		};
		assert_eq!(read(" http://node:9944 ").unwrap(), ["http://node:9944"]);
		assert_eq!(
			read("http://a:8545, http://b:8546,").unwrap(),
			["http://a:8545", "http://b:8546"]
		);
		assert!(read(" , ").unwrap_err().contains("empty"));
		assert!(env_urls("INDEXER_TEST_URL_UNSET").unwrap_err().to_string().contains("not set"));
	}

	#[test]
	fn committed_configs_load() {
		for env in ["mainnet", "testnet"] {
			let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("config/{env}.toml"));
			let config = Config::load(&path).unwrap();
			let ids: std::collections::BTreeSet<_> =
				config.chains.values().map(|c| c.chain_id).collect();
			assert_eq!(ids.len(), config.chains.len(), "{env}: chain ids must be unique");
		}
	}
}
