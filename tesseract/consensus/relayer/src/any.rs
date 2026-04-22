use std::sync::Arc;

use polkadot_sdk::sp_runtime::traits::{One, Zero};
use primitive_types::H256;
use serde::{Deserialize, Serialize};
use subxt::{
	config::{ExtrinsicParams, HashFor, Header},
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature},
};

use arb_host::ArbConfig;
use evm_host::EvmHostConfig;
use ismp::messaging::CreateConsensusState;
use op_host::OpConfig;
use tesseract_beefy::{
	host::{BeefyHost, BeefyHostConfig},
	prover::{Prover, ProverConfig},
};
use tesseract_bsc::BscPosConfig;
use tesseract_grandpa::{GrandpaConfig, GrandpaHost};
use tesseract_pharos::PharosConfig;
use tesseract_polygon::PolygonPosConfig;
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tesseract_substrate::{SubstrateClient, SubstrateConfig};
use tesseract_sync_committee::SyncCommitteeConfig;
use tesseract_tendermint::TendermintConfig;
use zk_beefy;

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
	/// EVM Host chain config
	EvmHost(EvmHostConfig),
	/// Pharos chain config
	Pharos(PharosConfig),
}

pub enum AnyHost<R: subxt::Config, P: subxt::Config> {
	Beefy(BeefyHost<R, P, zk_beefy::LocalProver, tesseract_beefy::backend::RedisProofBackend>),
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
		// Redis config for the relayer's proof queue
		redis: tesseract_beefy::backend::RedisConfig,
	},
	Grandpa {
		/// Substrate state machine config
		substrate: SubstrateConfig,
		/// Grandpa-specific host config
		grandpa: GrandpaConfig,
	},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperbridgeHostConfig {
	/// Configuration options for the beefy prover and host
	#[serde(flatten)]
	pub host: ConsensusHost,
}

impl HyperbridgeHostConfig {
	pub fn substrate_config(&self) -> SubstrateConfig {
		match &self.host {
			ConsensusHost::Beefy { substrate, .. } => substrate.clone(),
			ConsensusHost::Grandpa { substrate, .. } => substrate.clone(),
		}
	}
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
			ConsensusHost::Beefy { substrate, prover, beefy, redis } => {
				let client = SubstrateClient::<P>::new(substrate).await?;
				let prover_instance =
					Prover::<R, P, zk_beefy::LocalProver>::new(prover.clone()).await?;

				let backend =
					Arc::new(tesseract_beefy::backend::RedisProofBackend::new(redis).await?);

				AnyHost::Beefy(BeefyHost::new(beefy, prover_instance, client, backend).await?)
			},
			ConsensusHost::Grandpa { substrate, grandpa } =>
				AnyHost::Grandpa(GrandpaHost::<R, P>::new(&substrate, &grandpa).await?),
		};

		Ok(host)
	}
}

// NOTE: The `state_machine()` / `host_address()` helpers that used to live here
// were removed — the consensus config variants no longer embed `EvmConfig` /
// `SubstrateConfig`, so the info needed to answer those queries now lives
// alongside each consensus variant in the caller's pairing (see
// `create_client_map`'s input). Callers should read `EvmConfig::state_machine`
// / `.ismp_host` directly from the paired EVM host config.
