/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "messaging-evm";

use crate::{
	abi::{EvmHostInstance, PingModuleInstance},
	transport::RpcTransport,
};

use alloy::{
	eips::BlockId,
	network::EthereumWallet,
	primitives::{Address, U256 as AlloyU256},
	providers::{
		fillers::{
			BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
			WalletFiller,
		},
		Identity, Provider, ProviderBuilder, RootProvider,
	},
	signers::local::PrivateKeySigner,
};
use ismp::{consensus::ConsensusStateId, events::Event, host::StateMachine, messaging::Message};
use polkadot_sdk::frame_support::crypto::ecdsa::ECDSAExt;
use primitive_types::{H256, U256};

use evm_state_machine::presets::{
	REQUEST_COMMITMENTS_SLOT, REQUEST_RECEIPTS_SLOT, RESPONSE_COMMITMENTS_SLOT,
	RESPONSE_RECEIPTS_SLOT,
};

use ismp_solidity_abi::shared_types::{StateCommitment, StateMachineHeight};
use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, keccak_256, Pair, H160};
use std::{sync::Arc, time::Duration};
use tesseract_primitives::{
	queue::{start_pipeline, PipelineQueue},
	IsmpProvider, StateMachineUpdated, StreamError, TxResult,
};
use tx::handle_message_submission;

pub mod abi;
mod byzantine;
pub mod gas_oracle;
pub mod registry;
pub mod transport;
use tx::wait_for_transaction_receipt;
pub mod provider;

// #[cfg(test)]
// mod test;
pub mod tx;

pub type AlloyProvider = RootProvider;

/// Create a RootProvider from multiple URLs with automatic failover,
/// which uses alloy FallbackService when multiple URLs are provided.
pub fn create_provider(urls: &[String]) -> Result<RootProvider, anyhow::Error> {
	use alloy::{
		rpc::client::RpcClient,
		transports::{http::Http, layers::FallbackService},
	};

	if urls.is_empty() {
		return Err(anyhow::anyhow!("At least one RPC URL must be provided"));
	}

	if urls.len() == 1 {
		Ok(RootProvider::new_http(urls[0].parse()?))
	} else {
		let transports: Vec<Http<_>> = urls
			.iter()
			.map(|u| Ok(Http::new(u.parse()?)))
			.collect::<Result<_, anyhow::Error>>()?;
		let active_count = transports.len();
		let service = FallbackService::new(transports, active_count);
		let rpc_client = RpcClient::builder().transport(service, false);
		Ok(RootProvider::new(rpc_client))
	}
}

/// Recommended fillers for transaction sending (gas, blob gas, nonce, chain ID)
type RecommendedFills =
	JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>;

/// Type alias for Alloy provider with signer (recommended fillers + wallet)
pub type AlloySignerProvider = FillProvider<
	JoinFill<JoinFill<Identity, RecommendedFills>, WalletFiller<EthereumWallet>>,
	RootProvider,
>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientType {
	Geth,
	Erigon,
}

impl Default for ClientType {
	fn default() -> Self {
		Self::Geth
	}
}

impl ClientType {
	pub fn erigon(&self) -> bool {
		match &self {
			ClientType::Erigon => true,
			_ => false,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmConfig {
	/// RPC urls for the execution client
	pub rpc_urls: Vec<String>,
	/// State machine Identifier for this client on its counterparties. When
	/// omitted, [`EvmClient::new`] derives it from `eth_chainId` (so the
	/// relayer config can stay minimal for any known chain).
	#[serde(default, with = "tesseract_primitives::serde_adapters::option_state_machine")]
	pub state_machine: Option<StateMachine>,
	/// Consensus state id for the consensus client on counterparty chain.
	/// When omitted, [`EvmClient::new`] looks it up in
	/// [`registry::consensus_state_id_for_chain_id`] using the resolved
	/// chain id.
	#[serde(default)]
	pub consensus_state_id: Option<String>,
	/// `IsmpHost` contract address. When omitted, [`EvmClient::new`]
	/// looks it up in [`registry::ismp_host_for_chain_id`] using the
	/// resolved chain id.
	#[serde(default)]
	pub ismp_host: Option<H160>,
	/// Relayer account private key. When omitted, the chain runs in
	/// inbound-only mode: events are read but no transactions are submitted to
	/// it, so it's excluded from outbound delivery, fee withdrawal, and
	/// fisherman roles.
	#[serde(default)]
	pub signer: Option<String>,
	/// Batch size to parallelize tracing
	pub tracing_batch_size: Option<usize>,
	/// Batch size when querying events
	pub query_batch_size: Option<u64>,
	/// Polling frequency for state machine updates in seconds
	pub poll_interval: Option<u64>,
	/// An optional buffer to add to gas price in basis points
	/// to increase likelihood of the transactions going through e.g 100 (1%), 200 (2%)
	pub gas_price_buffer: Option<u32>,
	/// The client type the rpc is running, defaults to Geth
	pub client_type: Option<ClientType>,
	/// Initial height from which to start querying messages
	pub initial_height: Option<u64>,
	/// Selects the JSON-RPC transport variant.  Defaults to [`RpcTransport::Standard`].
	/// Set to [`RpcTransport::Tron`] for TRON nodes whose JSON-RPC proxy rejects
	/// EIP-1559 fields (`type`, `accessList`).
	#[serde(default)]
	pub transport: RpcTransport,
}

impl EvmConfig {
	/// Convert the config into a client.
	pub async fn into_client(self) -> anyhow::Result<EvmClient> {
		let client = EvmClient::new(self).await?;

		Ok(client)
	}

	/// Returns the explicit `state_machine` if set, otherwise `None`.
	/// Callers that need the resolved value should construct an
	/// [`EvmClient`] and read `client.state_machine` instead.
	pub fn state_machine(&self) -> Option<StateMachine> {
		self.state_machine
	}
}

impl Default for EvmConfig {
	fn default() -> Self {
		Self {
			rpc_urls: Default::default(),
			state_machine: None,
			consensus_state_id: None,
			ismp_host: None,
			signer: None,
			tracing_batch_size: Default::default(),
			query_batch_size: Default::default(),
			poll_interval: Default::default(),
			gas_price_buffer: Default::default(),
			client_type: Default::default(),
			initial_height: Default::default(),
			transport: Default::default(),
		}
	}
}

/// Core EVM client.
pub struct EvmClient {
	/// Execution Rpc client
	pub client: Arc<AlloyProvider>,
	/// Transaction signer provider. For chains the operator did not configure
	/// a signer for, this is built from a randomly generated key. The
	/// relayer's spawn-time filter (`PerChainConfig::outbound_enabled`) keeps
	/// signer-less chains out of any task that would actually broadcast a
	/// transaction, so the dummy is never used to send anything.
	pub signer: Arc<AlloySignerProvider>,
	/// Public Key Address
	pub address: Vec<u8>,
	/// Consensus state Id (resolved from config or registry).
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this client (resolved from config or
	/// `eth_chainId`).
	pub state_machine: StateMachine,
	/// `IsmpHost` contract address (resolved from config or registry).
	pub ismp_host: H160,
	/// Latest state machine height.
	initial_height: u64,
	/// Config (user-supplied; the resolved `state_machine`, `ismp_host`,
	/// and `consensus_state_id` are reflected on the dedicated fields
	/// above, not in this snapshot).
	pub config: EvmConfig,
	/// EVM chain Id.
	pub chain_id: u64,
	/// Client type
	pub client_type: ClientType,
	/// Private key signer for synchronous signing operations. Same dummy-key
	/// note as `signer`.
	pub private_key_signer: PrivateKeySigner,
	/// Producer for state machine updated stream
	state_machine_update_sender: Arc<
		tokio::sync::Mutex<
			Option<tokio::sync::broadcast::Sender<Result<StateMachineUpdated, StreamError>>>,
		>,
	>,
	/// Tx submission pipeline
	queue: Option<Arc<PipelineQueue<Vec<Message>, anyhow::Result<TxResult>>>>,
}

impl EvmClient {
	pub async fn new(config: EvmConfig) -> Result<Self, anyhow::Error> {
		let config_clone = config.clone();
		// If the operator configured a signer, parse it; otherwise generate a
		// throwaway one so all the signer-shaped fields downstream still
		// type-check. Inbound-only chains never reach a path that signs,
		// because the relayer's `outbound_enabled()` filter keeps them out
		// of outbound, fee-withdrawal, and fisherman tasks before any
		// signing call.
		let signer_pair = match config.signer.as_deref() {
			Some(raw) => {
				let bytes = match from_hex(raw) {
					Ok(bytes) => bytes,
					Err(_) => {
						// Treat the value as a file path containing hex bytes.
						let contents = tokio::fs::read_to_string(raw).await?;
						from_hex(contents.as_str())?
					},
				};
				sp_core::ecdsa::Pair::from_seed_slice(&bytes)?
			},
			None => sp_core::ecdsa::Pair::generate().0,
		};
		let address = signer_pair.public().to_eth_address().expect("Infallible").to_vec();

		if config.rpc_urls.is_empty() {
			return Err(anyhow::anyhow!("At least one RPC URL must be provided"));
		}

		let http_client = alloy::transports::http::reqwest::Client::builder()
			.timeout(Duration::from_secs(180))
			.build()?;

		let root_provider = if config.rpc_urls.len() == 1 {
			let url: alloy::transports::http::reqwest::Url = config.rpc_urls[0].parse()?;
			match config.transport {
				RpcTransport::Tron => {
					use crate::transport::TronLayer;
					let http = alloy::transports::http::Http::with_client(http_client, url);
					let rpc_client = alloy::rpc::client::ClientBuilder::default()
						.layer(TronLayer)
						.transport(http, false);
					RootProvider::new(rpc_client)
				},
				RpcTransport::Standard => {
					let rpc_client =
						alloy::rpc::client::RpcClient::new_http_with_client(http_client, url);
					RootProvider::new(rpc_client)
				},
			}
		} else {
			let transports: Vec<alloy::transports::http::Http<_>> = config
				.rpc_urls
				.iter()
				.map(|u| {
					let url: alloy::transports::http::reqwest::Url = u.parse()?;
					Ok(alloy::transports::http::Http::with_client(http_client.clone(), url))
				})
				.collect::<Result<_, anyhow::Error>>()?;
			let active_count = transports.len();
			let service = alloy::transports::layers::FallbackService::new(transports, active_count);
			match config.transport {
				RpcTransport::Tron => {
					use crate::transport::TronLayer;
					let rpc_client = alloy::rpc::client::ClientBuilder::default()
						.layer(TronLayer)
						.transport(service, false);
					RootProvider::new(rpc_client)
				},
				RpcTransport::Standard => {
					let rpc_client =
						alloy::rpc::client::RpcClient::builder().transport(service, false);
					RootProvider::new(rpc_client)
				},
			}
		};
		let client = Arc::new(root_provider.clone());
		let chain_id = client.get_chain_id().await?;

		// Build the signer provider. Whether `signer_pair` was parsed from
		// the configured key or freshly generated for an inbound-only chain,
		// the type shape is the same downstream.
		let private_key_signer = PrivateKeySigner::from_slice(signer_pair.seed().as_slice())?;
		let wallet = EthereumWallet::from(private_key_signer.clone());
		let signer_provider = ProviderBuilder::new().wallet(wallet).connect_provider(root_provider);
		let signer = Arc::new(signer_provider);

		// Resolve the three optional config fields. Explicit values always
		// win; otherwise we use the chain id (already fetched above) to
		// look up the canonical entries from `crate::registry`.
		let state_machine =
			config.state_machine.unwrap_or_else(|| StateMachine::Evm(chain_id as u32));

		let ismp_host = match config.ismp_host {
			Some(host) => host,
			None => crate::registry::ismp_host_for_chain_id(chain_id).ok_or_else(|| {
				anyhow::anyhow!(
					"no IsmpHost configured for chain_id={chain_id}; set ismp_host explicitly \
					 or add the chain to tesseract_evm::registry"
				)
			})?,
		};

		let consensus_state_id_str = match config.consensus_state_id.as_deref() {
			Some(s) => s.to_string(),
			None => crate::registry::consensus_state_id_for_chain_id(chain_id)
				.map(|s| s.to_string())
				.ok_or_else(|| {
					anyhow::anyhow!(
						"no consensus_state_id configured for chain_id={chain_id}; set it \
						 explicitly or add the chain to tesseract_evm::registry"
					)
				})?,
		};
		let consensus_state_id = {
			let mut id: ConsensusStateId = Default::default();
			id.copy_from_slice(consensus_state_id_str.as_bytes());
			id
		};

		let latest_height = if let Some(initial_height) = config.initial_height {
			initial_height
		} else {
			client.get_block_number().await?
		};
		let mut partial_client = Self {
			client,
			signer,
			address,
			consensus_state_id,
			state_machine,
			ismp_host,
			initial_height: latest_height,
			config: config_clone.clone(),
			chain_id,
			client_type: config.client_type.unwrap_or_default(),
			private_key_signer,
			state_machine_update_sender: Arc::new(tokio::sync::Mutex::new(None)),
			queue: None,
		};

		let partial_client_clone = partial_client.clone();
		let queue = start_pipeline(move |messages| {
			let client = partial_client_clone.clone();
			async move { handle_message_submission(&client, messages).await }
		});
		partial_client.queue = Some(Arc::new(queue));
		Ok(partial_client)
	}

	pub async fn events(&self, from: u64, to: u64) -> Result<Vec<Event>, anyhow::Error> {
		use alloy::rpc::types::Filter;
		use alloy_sol_types::SolEvent;
		use ismp_solidity_abi::{
			evm_host::EvmHost::{
				GetRequestEvent, GetRequestHandled, PostRequestEvent, PostRequestHandled,
				PostResponseEvent, PostResponseHandled,
				StateMachineUpdated as EvmStateMachineUpdated,
			},
			EvmHostEvents,
		};

		let host_addr = Address::from_slice(&self.ismp_host.0);
		let filter = Filter::new().address(host_addr).from_block(from).to_block(to);

		let logs = self.client.get_logs(&filter).await?;

		let events = logs
			.into_iter()
			.filter_map(|log| {
				// Try to decode as each event type and convert to EvmHostEvents
				if let Ok(event) = PostRequestEvent::decode_log(&log.inner) {
					return EvmHostEvents::PostRequestEvent(event.data).try_into().ok();
				}
				if let Ok(event) = PostResponseEvent::decode_log(&log.inner) {
					return EvmHostEvents::PostResponseEvent(event.data).try_into().ok();
				}
				if let Ok(event) = GetRequestEvent::decode_log(&log.inner) {
					return EvmHostEvents::GetRequestEvent(event.data).try_into().ok();
				}
				if let Ok(event) = PostRequestHandled::decode_log(&log.inner) {
					return EvmHostEvents::PostRequestHandled(event.data).try_into().ok();
				}
				if let Ok(event) = PostResponseHandled::decode_log(&log.inner) {
					return EvmHostEvents::PostResponseHandled(event.data).try_into().ok();
				}
				if let Ok(event) = GetRequestHandled::decode_log(&log.inner) {
					return EvmHostEvents::GetRequestHandled(event.data).try_into().ok();
				}
				if let Ok(event) = EvmStateMachineUpdated::decode_log(&log.inner) {
					return EvmHostEvents::StateMachineUpdated(event.data).try_into().ok();
				}
				None
			})
			.collect::<Vec<_>>();
		Ok(events)
	}

	/// Set the consensus state on the IsmpHost
	pub async fn set_consensus_state(
		&self,
		consensus_state: Vec<u8>,
		height: StateMachineHeight,
		commitment: StateCommitment,
	) -> Result<(), anyhow::Error> {
		use alloy::primitives::Bytes;

		let host_addr = Address::from_slice(&self.ismp_host.0);
		let contract = EvmHostInstance::new(host_addr, self.signer.clone());
		let call = contract.setConsensusState(Bytes::from(consensus_state), height, commitment);

		let gas = call.estimate_gas().await?;
		let pending = call.gas(gas).send().await?;
		let tx_hash = *pending.tx_hash();
		wait_for_transaction_receipt(H256::from_slice(tx_hash.as_slice()), self).await?;

		Ok(())
	}

	/// Dispatch a test request to the parachain.
	pub async fn dispatch_to_parachain(
		&self,
		address: H160,
		para_id: u32,
	) -> Result<(), anyhow::Error> {
		let ping_addr = Address::from_slice(&address.0);
		let contract = PingModuleInstance::new(ping_addr, self.signer.clone());
		let call = contract.dispatchToParachain(AlloyU256::from(para_id));

		let gas = call.estimate_gas().await?;
		let pending = call.gas(gas).send().await?;
		let tx_hash = *pending.tx_hash();
		wait_for_transaction_receipt(H256::from_slice(tx_hash.as_slice()), self).await?;

		Ok(())
	}

	pub async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		if self.config.initial_height.is_none() {
			self.initial_height = counterparty
				.query_latest_height(self.state_machine_id())
				.await
				.unwrap_or(self.initial_height as u32)
				.into();
		}

		log::info!(target: LOG_TARGET, "Initialized height for {:?} at {}", self.state_machine, self.initial_height);

		Ok(())
	}

	pub fn request_commitment_key(&self, key: H256) -> (H256, H256) {
		let key = derive_map_key(key.0.to_vec(), REQUEST_COMMITMENTS_SLOT);
		let number = U256::from_big_endian(key.0.as_slice()) + U256::from(1);
		let bytes = number.to_big_endian();
		(key, H256(bytes))
	}

	pub fn response_commitment_key(&self, key: H256) -> (H256, H256) {
		let key = derive_map_key(key.0.to_vec(), RESPONSE_COMMITMENTS_SLOT);
		let number = U256::from_big_endian(key.0.as_slice()) + U256::from(1);
		let bytes = number.to_big_endian();
		(key, H256(bytes))
	}

	pub fn request_receipt_key(&self, key: H256) -> H256 {
		derive_map_key(key.0.to_vec(), REQUEST_RECEIPTS_SLOT)
	}

	pub fn response_receipt_key(&self, key: H256) -> Vec<Vec<u8>> {
		let key = derive_map_key(key.0.to_vec(), RESPONSE_RECEIPTS_SLOT);
		let number = U256::from_big_endian(key.0.as_slice()) + U256::from(1);
		let bytes = number.to_big_endian();

		vec![key.0.to_vec(), bytes.to_vec()]
	}

	pub async fn host_manager(&self) -> Result<H160, anyhow::Error> {
		let host_addr = Address::from_slice(&self.ismp_host.0);
		let contract = EvmHostInstance::new(host_addr, self.client.clone());
		let params = contract.hostParams().block(BlockId::latest()).call().await?;
		Ok(H160::from_slice(params.hostManager.as_slice()))
	}

	pub async fn handler(&self) -> Result<H160, anyhow::Error> {
		let host_addr = Address::from_slice(&self.ismp_host.0);
		let contract = EvmHostInstance::new(host_addr, self.client.clone());
		let params = contract.hostParams().block(BlockId::latest()).call().await?;
		Ok(H160::from_slice(params.handler.as_slice()))
	}
}

pub fn derive_map_key(mut key: Vec<u8>, slot: u64) -> H256 {
	let bytes = U256::from(slot as u64).to_big_endian();
	key.extend_from_slice(&bytes);
	keccak_256(&key).into()
}

const STATE_COMMITMENT_SLOT: u64 = 5;
// keccak256(uint256(4009) . keccak256(uint256(200_000_000) . uint256(STATE_COMMITMENT_SLOT)))
pub fn state_comitment_key(state_machine_id: U256, block_height: U256) -> (H256, H256, H256) {
	// Parent map key
	let slot = U256::from(STATE_COMMITMENT_SLOT).to_big_endian();

	let state_id = state_machine_id.to_big_endian();
	let mut key = state_id.to_vec();
	key.extend_from_slice(&slot);
	let parent_map_key = keccak_256(&key);

	// Commitment key
	let bytes = block_height.to_big_endian();
	let mut commitment_key = bytes.to_vec();
	commitment_key.extend_from_slice(&parent_map_key);

	let slot_hash = keccak_256(&commitment_key);

	// Timestamp is at offset 0

	// overlay root is at offset 1

	let overlay_root_slot = {
		let slot = U256::from_big_endian(&slot_hash) + U256::one();
		let bytes = slot.to_big_endian();
		H256::from_slice(&bytes)
	};

	// state root is at offset 2

	let state_root_key = {
		let slot = U256::from_big_endian(&slot_hash) + U256::one() + U256::one();
		let bytes = slot.to_big_endian();
		H256::from_slice(&bytes)
	};

	(slot_hash.into(), overlay_root_slot, state_root_key)
}

impl Clone for EvmClient {
	fn clone(&self) -> Self {
		Self {
			client: self.client.clone(),
			signer: self.signer.clone(),
			address: self.address.clone(),
			consensus_state_id: self.consensus_state_id,
			state_machine: self.state_machine,
			ismp_host: self.ismp_host,
			initial_height: self.initial_height,
			config: self.config.clone(),
			chain_id: self.chain_id.clone(),
			client_type: self.client_type.clone(),
			private_key_signer: self.private_key_signer.clone(),
			state_machine_update_sender: self.state_machine_update_sender.clone(),
			queue: self.queue.clone(),
		}
	}
}
