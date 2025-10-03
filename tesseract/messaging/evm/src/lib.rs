use crate::abi::{EvmHost, PingModule};

use ethers::{
	core::k256::ecdsa::SigningKey,
	prelude::{k256::SecretKey, LocalWallet, MiddlewareBuilder, SignerMiddleware, Wallet},
	providers::{Http, Middleware, Provider},
	signers::Signer,
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
pub mod provider;

// #[cfg(test)]
// mod test;
pub mod tx;

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
	/// State machine Identifier for this client on it's counterparties.
	#[serde(with = "serde_hex_utils::as_string")]
	pub state_machine: StateMachine,
	/// Consensus state id for the consensus client on counterparty chain
	pub consensus_state_id: String,
	/// Ismp Host contract address
	pub ismp_host: H160,
	/// Relayer account private key
	pub signer: String,
	/// Etherscan API key
	pub etherscan_api_key: String,
	/// Batch size to parallelize tracing
	pub tracing_batch_size: Option<usize>,
	/// Batch size when querying events
	pub query_batch_size: Option<u64>,
	/// Polling frequency for state machine updates in seconds
	pub poll_interval: Option<u64>,
	/// An optional buffer to add to gas price as a percentage of the current gas price
	/// to increase likelihood of the transactions going through e.g 1%, 2%
	pub gas_price_buffer: Option<u32>,
	/// The client type the rpc is running, defaults to Geth
	pub client_type: Option<ClientType>,
	/// Initial height from which to start querying messages
	pub initial_height: Option<u64>,
}

impl EvmConfig {
	/// Convert the config into a client.
	pub async fn into_client(self) -> anyhow::Result<EvmClient> {
		let client = EvmClient::new(self).await?;

		Ok(client)
	}

	pub fn state_machine(&self) -> StateMachine {
		self.state_machine
	}
}

impl Default for EvmConfig {
	fn default() -> Self {
		Self {
			rpc_urls: Default::default(),
			state_machine: StateMachine::Evm(1),
			consensus_state_id: Default::default(),
			ismp_host: Default::default(),
			signer: Default::default(),
			etherscan_api_key: Default::default(),
			tracing_batch_size: Default::default(),
			query_batch_size: Default::default(),
			poll_interval: Default::default(),
			gas_price_buffer: Default::default(),
			client_type: Default::default(),
			initial_height: Default::default(),
		}
	}
}

/// Core EVM client.
pub struct EvmClient {
	/// Execution Rpc client
	pub client: Arc<Provider<Http>>,
	/// Transaction signer
	pub signer: Arc<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>,
	/// Public Key Address
	pub address: Vec<u8>,
	/// Consensus state Id
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this client.
	pub state_machine: StateMachine,
	/// Latest state machine height.
	initial_height: u64,
	/// Config
	pub config: EvmConfig,
	/// EVM chain Id.
	pub chain_id: u64,
	/// Client type
	pub client_type: ClientType,
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
		let bytes = match from_hex(config.signer.as_str()) {
			Ok(bytes) => bytes,
			Err(_) => {
				// it's probably a file.
				let contents = tokio::fs::read_to_string(config.signer.as_str()).await?;
				from_hex(contents.as_str())?
			},
		};
		let signer = sp_core::ecdsa::Pair::from_seed_slice(&bytes)?;
		let address = signer.public().to_eth_address().expect("Infallible").to_vec();

		let http_client = Http::new_client_with_chain_middleware(
			config.rpc_urls.into_iter().map(|url| url.parse()).collect::<Result<_, _>>()?,
			Some(Duration::from_secs(180)),
		);
		let provider = Provider::new(http_client);
		let client = Arc::new(provider.clone());
		let chain_id = client.get_chainid().await?.low_u64();
		let signer = LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
			.with_chain_id(chain_id);
		let signer = Arc::new(provider.with_signer(signer));
		let consensus_state_id = {
			let mut consensus_state_id: ConsensusStateId = Default::default();
			consensus_state_id.copy_from_slice(config.consensus_state_id.as_bytes());
			consensus_state_id
		};

		let latest_height = if let Some(initial_height) = config.initial_height {
			initial_height
		} else {
			client.get_block_number().await?.as_u64()
		};
		let mut partial_client = Self {
			client,
			signer,
			address,
			consensus_state_id,
			state_machine: config.state_machine,
			initial_height: latest_height,
			config: config_clone,
			chain_id,
			client_type: config.client_type.unwrap_or_default(),
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
		let client = Arc::new(self.client.clone());
		let contract = EvmHost::new(self.config.ismp_host.0, client);
		let events = contract
			.events()
			.address(ethers::core::types::H160(self.config.ismp_host.0).into())
			.from_block(from)
			.to_block(to)
			.query()
			.await?
			.into_iter()
			.filter_map(|ev| ev.try_into().ok())
			.collect::<_>();
		Ok(events)
	}

	/// Set the consensus state on the IsmpHost
	pub async fn set_consensus_state(
		&self,
		consensus_state: Vec<u8>,
		height: StateMachineHeight,
		commitment: StateCommitment,
	) -> Result<(), anyhow::Error> {
		let contract = EvmHost::new(self.config.ismp_host.0, self.signer.clone());
		let call = contract.set_consensus_state(consensus_state.clone().into(), height, commitment);

		let gas = call.estimate_gas().await?;
		call.gas(gas).send().await?.await?;

		Ok(())
	}

	/// Dispatch a test request to the parachain.
	pub async fn dispatch_to_parachain(
		&self,
		address: H160,
		para_id: u32,
	) -> Result<(), anyhow::Error> {
		let contract = PingModule::new(address.0, self.signer.clone());
		let call = contract.dispatch_to_parachain(para_id.into());

		let gas = call.estimate_gas().await?;
		call.gas(gas).send().await?.await?;

		Ok(())
	}

	pub async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		if self.config.initial_height.is_none() {
			self.initial_height =
				counterparty.query_latest_height(self.state_machine_id()).await?.into();
		}

		log::info!("Initialized height for {:?} at {}", self.state_machine, self.initial_height);

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
		let contract = EvmHost::new(self.config.ismp_host.0, self.client.clone());
		let params = contract.host_params().call().await?;
		Ok(params.host_manager.0.into())
	}

	pub async fn handler(&self) -> Result<H160, anyhow::Error> {
		let contract = EvmHost::new(self.config.ismp_host.0, self.client.clone());
		let params = contract.host_params().call().await?;
		Ok(params.handler.0.into())
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
			initial_height: self.initial_height,
			config: self.config.clone(),
			chain_id: self.chain_id.clone(),
			client_type: self.client_type.clone(),
			state_machine_update_sender: self.state_machine_update_sender.clone(),
			queue: self.queue.clone(),
		}
	}
}
