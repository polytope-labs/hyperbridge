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
use tesseract_parachain::ParachainConfig;
use tesseract_pharos::PharosConfig;
use tesseract_polygon::PolygonPosConfig;
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tesseract_substrate::{SubstrateClient, SubstrateConfig};
use tesseract_sync_committee::SyncCommitteeConfig;
use tesseract_tendermint::TendermintConfig;
use zk_beefy;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Various chain configurations supported by consensus task.
///
/// Every variant is a struct variant with a single `#[serde(flatten)] inner`
/// field. Flattening keeps the TOML surface identical to the old tuple
/// shape — a `[<chain>.consensus]` table contains `type = "..."` plus the
/// inner config's fields at the same level, no extra nesting under
/// `inner = { ... }`.
pub enum AnyConfig {
	/// Ethereum Sepolia sync committee config
	Sepolia {
		#[serde(flatten)]
		inner: SyncCommitteeConfig,
	},
	/// Ethereum Mainnet sync committee config
	Ethereum {
		#[serde(flatten)]
		inner: SyncCommitteeConfig,
	},
	/// Any Arbitrum orbit chain config
	ArbitrumOrbit {
		#[serde(flatten)]
		inner: ArbConfig,
	},
	/// Any Opstack chain config
	OpStack {
		#[serde(flatten)]
		inner: OpConfig,
	},
	/// Bsc testnet chain config
	BscTestnet {
		#[serde(flatten)]
		inner: BscPosConfig,
	},
	/// Bsc mainnet chain config
	Bsc {
		#[serde(flatten)]
		inner: BscPosConfig,
	},
	/// Gnosis Chiado testnet sync committee config
	Chiado {
		#[serde(flatten)]
		inner: SyncCommitteeConfig,
	},
	/// Gnosis Mainnet sync committee config
	Gnosis {
		#[serde(flatten)]
		inner: SyncCommitteeConfig,
	},
	/// Grandpa committee config
	Grandpa {
		#[serde(flatten)]
		inner: GrandpaConfig,
	},
	/// Parachain consensus config — relayed from one parachain (self) to another
	/// parachain counterparty via relay-chain storage proofs.
	Parachain {
		#[serde(flatten)]
		inner: ParachainConfig,
	},
	/// Polygon POS chain config
	Polygon {
		#[serde(flatten)]
		inner: PolygonPosConfig,
	},
	/// Tendermint Config
	Tendermint {
		#[serde(flatten)]
		inner: TendermintConfig,
	},
	/// EVM Host chain config
	EvmHost {
		#[serde(flatten)]
		inner: EvmHostConfig,
	},
	/// Pharos chain config
	Pharos {
		#[serde(flatten)]
		inner: PharosConfig,
	},
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

use std::collections::{BTreeMap, HashMap};

use anyhow::anyhow;
use ismp::host::StateMachine;
use substrate_state_machine::HashAlgorithm;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};
use tesseract_sync_committee::L2Config;

/// Host-side config paired with a consensus variant. EVM-family consensus
/// clients need an [`tesseract_evm::EvmConfig`]; substrate-family (grandpa,
/// parachain) consensus clients need a [`SubstrateConfig`].
#[derive(Debug, Clone)]
pub enum HostKind {
	Evm(tesseract_evm::EvmConfig),
	Substrate(SubstrateConfig),
}

impl HostKind {
	pub fn as_evm(&self) -> Option<&tesseract_evm::EvmConfig> {
		match self {
			HostKind::Evm(e) => Some(e),
			_ => None,
		}
	}
	pub fn as_substrate(&self) -> Option<&SubstrateConfig> {
		match self {
			HostKind::Substrate(s) => Some(s),
			_ => None,
		}
	}
}

/// Extract all Eth L2 configs from the consensus/host pairings provided.
/// Keeps the paired `EvmConfig` alongside each variant because the inner L2
/// host constructors need it.
fn extract_l2_configs(
	supported_l2s: Vec<String>,
	config_map: HashMap<StateMachine, (AnyConfig, HostKind)>,
) -> BTreeMap<StateMachine, L2Config> {
	let mut map = BTreeMap::new();
	for (state_machine, (config, host)) in config_map
		.into_iter()
		.filter(|(state_machine, ..)| supported_l2s.contains(&state_machine.to_string()))
	{
		let HostKind::Evm(evm) = host else { continue };
		match config {
			AnyConfig::ArbitrumOrbit { inner } => {
				map.insert(state_machine, L2Config::ArbitrumOrbit(inner, evm));
			},
			AnyConfig::OpStack { inner } => {
				map.insert(state_machine, L2Config::OpStack(inner, evm));
			},
			_ => {},
		}
	}

	map
}

/// Build the map of consensus clients from per-chain `(AnyConfig, HostKind)`
/// pairings.
///
/// Each chain entry is a `(AnyConfig, HostKind)` pair: the consensus variant
/// and the host-side config (EVM or Substrate). The consensus variant alone
/// doesn't embed the host config — that's in `HostKind` so one set of types
/// carries the context needed to construct the concrete `IsmpHost`.
pub async fn create_client_map(
	chains: HashMap<StateMachine, (AnyConfig, HostKind)>,
) -> anyhow::Result<HashMap<StateMachine, Arc<dyn IsmpHost>>> {
	let mut clients = HashMap::new();

	// Snapshot for l2 resolution (each call into_* consumes its entry).
	let l2_source = chains.clone();

	for (state_machine, (config, host)) in chains {
		let client = match (config, host) {
			(AnyConfig::Sepolia { inner }, HostKind::Evm(evm)) => {
				let l2_configs = extract_l2_configs(
					inner.layer_twos.clone().unwrap_or_default(),
					l2_source.clone(),
				);
				inner.into_sepolia(evm, l2_configs).await?
			},
			(AnyConfig::Ethereum { inner }, HostKind::Evm(evm)) => {
				let l2_configs = extract_l2_configs(
					inner.layer_twos.clone().unwrap_or_default(),
					l2_source.clone(),
				);
				inner.into_mainnet(evm, l2_configs).await?
			},
			(AnyConfig::ArbitrumOrbit { inner }, HostKind::Evm(evm)) =>
				inner.into_client(evm).await?,
			(AnyConfig::OpStack { inner }, HostKind::Evm(evm)) => inner.into_client(evm).await?,
			(AnyConfig::BscTestnet { inner }, HostKind::Evm(evm)) =>
				inner.into_client::<tesseract_bsc::Testnet>(evm).await?,
			(AnyConfig::Bsc { inner }, HostKind::Evm(evm)) =>
				inner.into_client::<tesseract_bsc::Mainnet>(evm).await?,
			(AnyConfig::Chiado { inner }, HostKind::Evm(evm)) => inner.into_chiado(evm).await?,
			(AnyConfig::Gnosis { inner }, HostKind::Evm(evm)) => inner.into_gnosis(evm).await?,
			(AnyConfig::Polygon { inner }, HostKind::Evm(evm)) => inner.into_client(evm).await?,
			(AnyConfig::Tendermint { inner }, HostKind::Evm(evm)) => inner.into_client(evm).await?,
			(AnyConfig::EvmHost { inner }, HostKind::Evm(evm)) => inner.into_client(evm).await?,
			(AnyConfig::Pharos { inner }, HostKind::Evm(evm)) => match evm.state_machine {
				StateMachine::Evm(688689) =>
					inner.into_client::<pharos_primitives::Testnet>(evm).await?,
				_ => inner.into_client::<pharos_primitives::Mainnet>(evm).await?,
			},
			(AnyConfig::Grandpa { inner }, HostKind::Substrate(substrate)) => {
				match substrate.hashing {
					Some(HashAlgorithm::Keccak) =>
						inner
							.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>(substrate)
							.await?,
					_ =>
						inner
							.into_client::<Blake2SubstrateChain, Blake2SubstrateChain>(substrate)
							.await?,
				}
			},
			(AnyConfig::Parachain { inner }, HostKind::Substrate(substrate)) => {
				// S is the parachain's own subxt config (hasher chosen by its
				// `hashing` setting); R is the relay chain subxt config — always
				// `Blake2SubstrateChain` since Polkadot/Kusama/Paseo all use
				// BlakeTwo256.
				match substrate.hashing {
					Some(HashAlgorithm::Keccak) =>
						inner
							.into_client::<KeccakSubstrateChain, Blake2SubstrateChain>(substrate)
							.await?,
					_ =>
						inner
							.into_client::<Blake2SubstrateChain, Blake2SubstrateChain>(substrate)
							.await?,
				}
			},
			(variant, host) => {
				return Err(anyhow!(
					"incompatible (consensus, host) pairing for {state_machine}: {variant:?} with \
					 {host:?}"
				));
			},
		};
		clients.insert(state_machine, client);
	}

	Ok(clients)
}
