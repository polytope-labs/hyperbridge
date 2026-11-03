use alloy::providers::{DynProvider, Provider, ProviderBuilder};

use crate::config::ChainConfig;

/** A chain the indexer follows, with a provider for the reads handlers make. */
pub struct Chain {
	pub name: String,
	pub config: ChainConfig,
	pub rpc_url: String,
	pub provider: DynProvider,
}

impl Chain {
	pub fn connect(name: &str, config: &ChainConfig) -> anyhow::Result<Self> {
		let rpc_url = config.rpc_url(name)?;
		let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?).erased();
		Ok(Self { name: name.to_string(), config: config.clone(), rpc_url, provider })
	}

	pub fn state_machine_id(&self) -> &str {
		&self.config.state_machine_id
	}
}
