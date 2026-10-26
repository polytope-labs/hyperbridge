use std::{collections::HashSet, net::SocketAddr, path::Path};

use anyhow::{anyhow, bail, Context};
use serde::Deserialize;

use crate::types::Hasher;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
	#[serde(default = "default_listen")]
	pub listen: SocketAddr,
	pub log: Option<String>,
	pub chains: Vec<ChainConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChainConfig {
	pub name: String,
	pub rpc: String,
	pub eth_chain_id: u64,
	pub hasher: Hasher,
	pub pallets: Vec<String>,
	#[serde(default = "default_max_block_range")]
	pub max_block_range: u64,
	#[serde(default = "default_cache_blocks")]
	pub cache_blocks: usize,
}

fn default_listen() -> SocketAddr {
	([0, 0, 0, 0], 8545).into()
}

fn default_max_block_range() -> u64 {
	1000
}

fn default_cache_blocks() -> usize {
	4096
}

impl Config {
	pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Config> {
		let path = path.as_ref();
		let raw = std::fs::read_to_string(path)
			.with_context(|| format!("failed to read config {}", path.display()))?;
		Self::parse(&raw)
	}

	pub fn parse(raw: &str) -> anyhow::Result<Config> {
		let mut config: Config = toml::from_str(raw).context("failed to parse config")?;
		for chain in &mut config.chains {
			chain.rpc =
				expand_env(&chain.rpc).with_context(|| format!("chain {}: rpc", chain.name))?;
		}
		config.validate()?;
		Ok(config)
	}

	fn validate(&self) -> anyhow::Result<()> {
		let mut names = HashSet::new();
		let mut chain_ids = HashSet::new();
		for chain in &self.chains {
			if !is_url_safe(&chain.name) {
				bail!("chain name {:?} must match [a-z0-9-]+", chain.name);
			}
			if !names.insert(chain.name.as_str()) {
				bail!("duplicate chain name {:?}", chain.name);
			}
			if !chain_ids.insert(chain.eth_chain_id) {
				bail!("chain {}: duplicate eth_chain_id {}", chain.name, chain.eth_chain_id);
			}
			if chain.rpc_urls().is_empty() {
				bail!("chain {}: rpc must not be empty", chain.name);
			}
			if chain.max_block_range < 1 {
				bail!("chain {}: max_block_range must be at least 1", chain.name);
			}
		}
		Ok(())
	}
}

impl ChainConfig {
	pub fn rpc_urls(&self) -> Vec<String> {
		self.rpc
			.split(',')
			.map(str::trim)
			.filter(|url| !url.is_empty())
			.map(str::to_owned)
			.collect()
	}
}

fn is_url_safe(name: &str) -> bool {
	let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
	!name.is_empty() && name.chars().all(allowed)
}

fn expand_env(input: &str) -> anyhow::Result<String> {
	let mut out = String::with_capacity(input.len());
	let mut rest = input;
	while let Some(start) = rest.find("${") {
		out.push_str(&rest[..start]);
		let after = &rest[start + 2..];
		let end = after.find('}').ok_or_else(|| anyhow!("unterminated ${{ in {input:?}"))?;
		let var = &after[..end];
		let value =
			std::env::var(var).map_err(|_| anyhow!("environment variable {var} is not set"))?;
		out.push_str(&value);
		rest = &after[end + 1..];
	}
	out.push_str(rest);
	Ok(out)
}

#[cfg(test)]
mod tests {
	use super::*;

	const EXAMPLE: &str = include_str!("../config.example.toml");

	fn set_example_env() {
		std::env::set_var("HYPERBRIDGE_NEXUS", "wss://nexus.example,wss://nexus-2.example");
		std::env::set_var("BIFROST", "wss://bifrost.example");
	}

	#[test]
	fn parses_example_config() {
		set_example_env();
		let config = Config::parse(EXAMPLE).unwrap();
		assert_eq!(config.listen, "0.0.0.0:8545".parse::<SocketAddr>().unwrap());
		assert_eq!(config.log.as_deref(), Some("info"));
		assert_eq!(config.chains.len(), 2);

		let nexus = &config.chains[0];
		assert_eq!(nexus.name, "hyperbridge-nexus");
		assert_eq!(nexus.rpc_urls(), vec!["wss://nexus.example", "wss://nexus-2.example"]);
		assert_eq!(nexus.eth_chain_id, 1000003367);
		assert_eq!(nexus.hasher, Hasher::Keccak);
		assert_eq!(nexus.pallets[0], "ismp");
		assert_eq!(nexus.max_block_range, 1000);
		assert_eq!(nexus.cache_blocks, 4096);

		let bifrost = &config.chains[1];
		assert_eq!(bifrost.name, "bifrost-mainnet");
		assert_eq!(bifrost.rpc_urls(), vec!["wss://bifrost.example"]);
		assert_eq!(bifrost.eth_chain_id, 1000002030);
		assert_eq!(bifrost.hasher, Hasher::Blake2);
		assert_eq!(bifrost.pallets, vec!["ismp"]);
	}

	#[test]
	fn rejects_duplicate_chain_name() {
		set_example_env();
		let duplicated = EXAMPLE.replace("bifrost-mainnet", "hyperbridge-nexus");
		let err = Config::parse(&duplicated).unwrap_err();
		assert!(err.to_string().contains("duplicate chain name"), "{err}");
	}

	#[test]
	fn rejects_duplicate_eth_chain_id() {
		set_example_env();
		let duplicated = EXAMPLE.replace("1000002030", "1000003367");
		let err = Config::parse(&duplicated).unwrap_err();
		assert!(err.to_string().contains("duplicate eth_chain_id"), "{err}");
	}

	#[test]
	fn rejects_unsafe_chain_name() {
		set_example_env();
		let renamed = EXAMPLE.replace("bifrost-mainnet", "Bifrost_Mainnet");
		let err = Config::parse(&renamed).unwrap_err();
		assert!(err.to_string().contains("[a-z0-9-]+"), "{err}");
	}

	#[test]
	fn rejects_unset_env_var() {
		set_example_env();
		let missing = EXAMPLE.replace("${BIFROST}", "${SUBSTRATE_ETH_RPC_UNSET_VAR}");
		let err = Config::parse(&missing).unwrap_err();
		assert!(format!("{err:#}").contains("SUBSTRATE_ETH_RPC_UNSET_VAR"), "{err:#}");
	}

	#[test]
	fn rejects_zero_block_range() {
		set_example_env();
		let zero = EXAMPLE
			.replace("eth_chain_id = 1000002030", "eth_chain_id = 1000002030\nmax_block_range = 0");
		let err = Config::parse(&zero).unwrap_err();
		assert!(err.to_string().contains("max_block_range"), "{err}");
	}

	#[test]
	fn rejects_empty_rpc() {
		set_example_env();
		std::env::set_var("SUBSTRATE_ETH_RPC_EMPTY", " , ");
		let empty = EXAMPLE.replace("${BIFROST}", "${SUBSTRATE_ETH_RPC_EMPTY}");
		let err = Config::parse(&empty).unwrap_err();
		assert!(err.to_string().contains("rpc must not be empty"), "{err}");
	}
}
