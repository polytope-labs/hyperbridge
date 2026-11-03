use alloy::providers::{DynProvider, Provider, ProviderBuilder};

use crate::{
	config::{env_key, ChainConfig, ChainKind},
	substrate::SubstrateRpc,
};

/** A chain the indexer follows, with clients for the reads handlers make. */
pub struct Chain {
	pub name: String,
	pub config: ChainConfig,
	pub rpc_url: String,
	/** The chain's Ethereum JSON-RPC: the chain itself, or substrate-eth-rpc for Substrate. */
	pub provider: DynProvider,
	/** For Substrate chains, the node's own RPC. It reads `<CHAIN>_SUBSTRATE`, and defaults to
	 * `rpc_url`, which is right when the facade runs inside the Hyperbridge node. */
	pub substrate: Option<SubstrateRpc>,
}

impl Chain {
	pub fn connect(
		name: &str,
		config: &ChainConfig,
		http: &reqwest::Client,
	) -> anyhow::Result<Self> {
		let rpc_url = config.rpc_url(name)?;
		let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?).erased();
		let substrate = (config.kind == ChainKind::Substrate).then(|| {
			let url = std::env::var(format!("{}_SUBSTRATE", env_key(name)))
				.unwrap_or_else(|_| rpc_url.clone());
			SubstrateRpc::new(&url, http.clone())
		});
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
