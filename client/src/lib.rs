use pallet_ismp::primitives::HashAlgorithm;
use serde::{Deserialize, Serialize};
// use grandpa::{GrandpaConfig, GrandpaHost};
use ismp_sync_committee::constants::{mainnet::Mainnet, sepolia::Sepolia};
use parachain::ParachainHost;
use tesseract_bsc_pos::{BscPosConfig, BscPosHost};
use tesseract_evm::{
	arbitrum::client::{ArbConfig, ArbHost},
	optimism::client::{OpConfig, OpHost},
	EvmClient,
};
// use tesseract_polygon_pos::{PolygonPosConfig, PolygonPosHost};
use tesseract_substrate::{
	config::{Blake2SubstrateChain, KeccakSubstrateChain},
	SubstrateClient, SubstrateConfig,
};
use tesseract_sync_committee::{SyncCommitteeConfig, SyncCommitteeHost};

mod any;

type Parachain<T> = SubstrateClient<ParachainHost, T>;
// type Grandpa<T> = SubstrateClient<GrandpaHost<T>, T>;

crate::chain! {
	KeccakParachain(SubstrateConfig, Parachain<KeccakSubstrateChain>),
	Parachain(SubstrateConfig, Parachain<Blake2SubstrateChain>),
	EthereumSepolia(SyncCommitteeConfig, EvmClient<SyncCommitteeHost<Sepolia>>),
	EthereumMainnet(SyncCommitteeConfig, EvmClient<SyncCommitteeHost<Mainnet>>),
	Arbitrum(ArbConfig, EvmClient<ArbHost>),
	Optimism(OpConfig, EvmClient<OpHost>),
	Base(OpConfig, EvmClient<OpHost>),
	Bsc(BscPosConfig, EvmClient<BscPosHost>),
	// Polygon(PolygonPosConfig, EvmClient<PolygonPosHost>),
	// Polkadot(GrandpaConfig, Grandpa<Blake2SubstrateChain>),
	// Kusama(GrandpaConfig, Grandpa<Blake2SubstrateChain>),
}

impl AnyConfig {
	/// Convert the [`HyperbridgeConfig`] into an implementation of an [`IsmpHost`]
	pub async fn into_client(self) -> Result<AnyClient, anyhow::Error> {
		let client = match self {
			AnyConfig::KeccakParachain(config) | AnyConfig::Parachain(config) => {
				match config.hashing.clone().unwrap_or(HashAlgorithm::Keccak) {
					HashAlgorithm::Keccak => {
						let host = ParachainHost::default();
						AnyClient::KeccakParachain(Parachain::new(Some(host), config).await?)
					},
					HashAlgorithm::Blake2 => {
						let host = ParachainHost::default();
						AnyClient::Parachain(Parachain::new(Some(host), config).await?)
					},
				}
			},
			AnyConfig::EthereumSepolia(config) => {
				let host = if let Some(ref host) = config.host {
					Some(SyncCommitteeHost::new(host, &config.evm_config).await?)
				} else {
					None
				};
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::EthereumSepolia(client)
			},
			AnyConfig::EthereumMainnet(config) => {
				let host = if let Some(ref host) = config.host {
					Some(SyncCommitteeHost::new(host, &config.evm_config).await?)
				} else {
					None
				};
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::EthereumMainnet(client)
			},
			AnyConfig::Arbitrum(config) => {
				let host = if let Some(ref host) = config.host {
					Some(ArbHost::new(host, &config.evm_config).await?)
				} else {
					None
				};
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::Arbitrum(client)
			},
			AnyConfig::Optimism(config) => {
				let host = if let Some(ref host) = config.host {
					Some(OpHost::new(host, &config.evm_config).await?)
				} else {
					None
				};
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::Optimism(client)
			},
			AnyConfig::Base(config) => {
				let host = if let Some(ref host) = config.host {
					Some(OpHost::new(host, &config.evm_config).await?)
				} else {
					None
				};
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::Base(client)
			},
			AnyConfig::Bsc(config) => {
				let host = if let Some(ref host) = config.host {
					Some(BscPosHost::new(host, &config.evm_config).await?)
				} else {
					None
				};
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::Bsc(client)
			},
			// AnyConfig::Polygon(config) => {
			// 	let host = PolygonPosHost::new(&config).await?;
			// 	let client = EvmClient::new(Some(host), config.evm_config).await?;
			// 	AnyClient::Polygon(client)
			// },
			// /* AnyConfig::Polkadot(config) => {
			//     * let naive = GrandpaHost::new(&config).await?;
			//     * AnyClient::Grandpa(Grandpa::new(naive, config.substrate).await?)
			//     * } */
		};

		Ok(client)
	}
}
