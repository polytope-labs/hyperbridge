use std::sync::Arc;

use polkadot_sdk::sp_runtime::traits::{One, Zero};
use primitive_types::{H160, H256};
use serde::{Deserialize, Serialize};
use subxt::{
	config::{ExtrinsicParams, HashFor, Header},
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature},
};

use arb_host::ArbConfig;
use ismp::{host::StateMachine, messaging::CreateConsensusState};
use op_host::OpConfig;
use tesseract_beefy::{
	host::{BeefyHost, BeefyHostConfig},
	prover::{Prover, ProverConfig},
};
use tesseract_bsc::BscPosConfig;
use tesseract_grandpa::{GrandpaConfig, GrandpaHost};
use tesseract_polygon::PolygonPosConfig;
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tesseract_substrate::{SubstrateClient, SubstrateConfig};
use tesseract_sync_committee::SyncCommitteeConfig;
use tesseract_tendermint::TendermintConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Various chain configurations supported by consensus task
pub enum AnyConfig {
	/// Ethereum Sepolia sync committee config
	Sepolia(SyncCommitteeConfig),
	/// Ethereum Mainnet sync committee config
	Ethereum(SyncCommitteeConfig),
	/// Any Arbitrum orbit chain config
	ArbitrumOrbit(ArbConfig),
	/// Any Opstack chain config
	OpStack(OpConfig),
	/// Bsc testnet chain config
	BscTestnet(BscPosConfig),
	/// Bsc mainnet chain config
	Bsc(BscPosConfig),
	/// Gnosis Chiado testnet sync committee config
	Chiado(SyncCommitteeConfig),
	/// Gnosis Mainnet sync committee config
	Gnosis(SyncCommitteeConfig),
	/// Grandpa committee config
	Grandpa(GrandpaConfig),
	/// Polygon POS chain config
	Polygon(PolygonPosConfig),
	/// Tendermint Config
	Tendermint(TendermintConfig),
}

pub enum AnyHost<R: subxt::Config, P: subxt::Config> {
	Beefy(BeefyHost<R, P>),
	Grandpa(GrandpaHost<R, P>),
}

impl<R, P> AnyHost<R, P>
where
	R: subxt::Config + Send + Sync + Clone,
	P: subxt::Config + Send + Sync + Clone,
{
	/// Retuns a reference to underlying [`SubstrateClient`] instance
	pub fn client(&self) -> &SubstrateClient<P> {
		match self {
			AnyHost::Beefy(beefy) => &beefy.client,
			AnyHost::Grandpa(grandpa) => &grandpa.substrate_client,
		}
	}
}

#[async_trait::async_trait]
impl<R, P> IsmpHost for AnyHost<R, P>
where
	R: subxt::Config + Send + Sync + Clone,
	P: subxt::Config + Send + Sync + Clone,
	<P::ExtrinsicParams as ExtrinsicParams<P>>::Params: Send + Sync + DefaultParams,
	P::Signature: From<MultiSignature> + Send + Sync,
	P::AccountId: From<AccountId32> + Into<P::Address> + Clone + 'static + Send + Sync,
	H256: From<HashFor<P>>,

	R: subxt::Config + Send + Sync + Clone,
	R::Header: Send + Sync,
	<R::Header as Header>::Number: Ord + Zero + finality_grandpa::BlockNumberOps + One + From<u32>,
	u32: From<<R::Header as Header>::Number>,
	H256: From<HashFor<R>>,
	R::Header: codec::Decode,
	<R::Hasher as subxt::config::Hasher>::Output: From<HashFor<R>>,
	HashFor<R>: From<<R::Hasher as subxt::config::Hasher>::Output>,
	HashFor<R>: From<H256>,
	<R::ExtrinsicParams as ExtrinsicParams<R>>::Params: Send + Sync + DefaultParams,
	R::Signature: From<MultiSignature> + Send + Sync,
	R::AccountId: From<AccountId32> + Into<R::Address> + Clone + 'static + Send + Sync,
{
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		match self {
			AnyHost::Beefy(beefy) => beefy.start_consensus(counterparty).await,
			AnyHost::Grandpa(grandpa) => grandpa.start_consensus(counterparty).await,
		}
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		match self {
			AnyHost::Beefy(beefy) => beefy.query_initial_consensus_state().await,
			AnyHost::Grandpa(grandpa) => grandpa.query_initial_consensus_state().await,
		}
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		match self {
			AnyHost::Beefy(beefy) => beefy.provider(),
			AnyHost::Grandpa(grandpa) => grandpa.provider(),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConsensusHost {
	Beefy {
		// Substrate state machine config
		substrate: SubstrateConfig,
		// Configuration options for the BEEFY prover
		prover: ProverConfig,
		// Host options for BEEFY
		beefy: BeefyHostConfig,
	},
	Grandpa(GrandpaConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperbridgeHostConfig {
	/// Configuration options for the beefy prover and host
	#[serde(flatten)]
	pub host: ConsensusHost,
}

impl HyperbridgeHostConfig {
	/// Constructs an instance of the [`IsmpHost`] from the provided configs
	pub async fn into_client<R, P>(self) -> Result<AnyHost<R, P>, anyhow::Error>
	where
		R: subxt::Config + Send + Sync + Clone,
		P: subxt::Config + Send + Sync + Clone,
		<P::ExtrinsicParams as ExtrinsicParams<P>>::Params: Send + Sync + DefaultParams,
		P::Signature: From<MultiSignature> + Send + Sync,
		P::AccountId: From<AccountId32> + Into<P::Address> + Clone + 'static + Send + Sync,
		H256: From<HashFor<P>>,

		<R::Header as Header>::Number: Ord + Zero + From<u32>,
		u32: From<<R::Header as Header>::Number>,
		H256: From<HashFor<R>>,
		R::Header: codec::Decode,
		<R::ExtrinsicParams as ExtrinsicParams<R>>::Params: Send + Sync + DefaultParams,
		R::Signature: From<MultiSignature> + Send + Sync,
		R::AccountId: From<AccountId32> + Into<R::Address> + Clone + 'static + Send + Sync,
	{
		let host = match self.host {
			ConsensusHost::Beefy { substrate, prover, beefy } => {
				let client = SubstrateClient::<P>::new(substrate).await?;
				let prover = Prover::<R, P>::new(prover.clone()).await?;
				AnyHost::Beefy(BeefyHost::<R, P>::new(beefy, prover, client).await?)
			},
			ConsensusHost::Grandpa(grandpa) =>
				AnyHost::Grandpa(GrandpaHost::<R, P>::new(&grandpa).await?),
		};

		Ok(host)
	}
}

impl AnyConfig {
	/// Returns the state machine for the config
	pub fn state_machine(&self) -> StateMachine {
		match self {
			AnyConfig::Sepolia(config) => config.evm_config.state_machine,
			AnyConfig::Ethereum(config) => config.evm_config.state_machine,
			AnyConfig::ArbitrumOrbit(config) => config.evm_config.state_machine,
			AnyConfig::OpStack(config) => config.evm_config.state_machine,
			AnyConfig::BscTestnet(config) => config.evm_config.state_machine,
			AnyConfig::Bsc(config) => config.evm_config.state_machine,
			AnyConfig::Chiado(config) => config.evm_config.state_machine,
			AnyConfig::Gnosis(config) => config.evm_config.state_machine,
			AnyConfig::Grandpa(config) => config.substrate.state_machine,
			AnyConfig::Polygon(config) => config.evm_config.state_machine,
			AnyConfig::Tendermint(config) => config.evm_config.state_machine,
		}
	}

	/// Returns the Ismp host contract address for EVM chains.
	pub fn host_address(&self) -> Option<H160> {
		match self {
			AnyConfig::Bsc(c) => Some(c.evm_config.ismp_host.clone()),
			AnyConfig::Sepolia(c) => Some(c.evm_config.ismp_host.clone()),
			AnyConfig::OpStack(c) => Some(c.evm_config.ismp_host.clone()),
			AnyConfig::ArbitrumOrbit(c) => Some(c.evm_config.ismp_host.clone()),
			AnyConfig::Ethereum(c) => Some(c.evm_config.ismp_host.clone()),
			AnyConfig::BscTestnet(c) => Some(c.evm_config.ismp_host.clone()),
			AnyConfig::Chiado(c) => Some(c.evm_config.ismp_host.clone()),
			AnyConfig::Gnosis(c) => Some(c.evm_config.ismp_host.clone()),
			AnyConfig::Grandpa(_) => None,
			AnyConfig::Polygon(c) => Some(c.evm_config.ismp_host.clone()),
			AnyConfig::Tendermint(c) => Some(c.evm_config.ismp_host.clone()),
		}
	}
}
