use crate::{
	abi::{EvmHost, PingModule},
	consts::{
		REQUEST_COMMITMENTS_SLOT, REQUEST_RECEIPTS_SLOT, RESPONSE_COMMITMENTS_SLOT,
		RESPONSE_RECEIPTS_SLOT,
	},
};

use ethabi::ethereum_types::{H256, U256};
use ethers::{
	core::k256::ecdsa::SigningKey,
	prelude::{k256::SecretKey, LocalWallet, MiddlewareBuilder, SignerMiddleware, Wallet},
	providers::{Http, Middleware, Provider},
	signers::Signer,
};
use frame_support::crypto::ecdsa::ECDSAExt;
use ismp::{
	consensus::ConsensusStateId,
	events::Event,
	host::{Ethereum, StateMachine},
};

use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, keccak_256, Pair, H160};
use std::sync::Arc;
use tesseract_primitives::{IsmpHost, IsmpProvider};

pub mod abi;
pub mod arbitrum;
pub mod consts;
mod gas_oracle;
mod host;
#[cfg(any(feature = "testing", test))]
pub mod mock;
pub mod optimism;
pub mod provider;

#[cfg(test)]
mod test;
pub mod tx;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmConfig {
	/// RPC url for the execution client
	pub rpc_url: String,
	/// State machine Identifier for this client on it's counterparties.
	pub state_machine: StateMachine,
	/// Consensus state id for the consensus client on counterparty chain
	pub consensus_state_id: String,
	/// Ismp Host contract address
	pub ismp_host: H160,
	/// Ismp Handler contract address
	pub handler: H160,
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
}

impl Default for EvmConfig {
	fn default() -> Self {
		Self {
			rpc_url: Default::default(),
			state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
			consensus_state_id: Default::default(),
			ismp_host: Default::default(),
			handler: Default::default(),
			signer: Default::default(),
			etherscan_api_key: Default::default(),
			tracing_batch_size: Default::default(),
			query_batch_size: Default::default(),
			poll_interval: Default::default(),
		}
	}
}

/// Core EVM client.
pub struct EvmClient<I> {
	/// Ismp naive implementation
	pub host: Option<I>,
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
	config: EvmConfig,
	/// EVM chain Id.
	pub chain_id: u64,
}

impl<I> EvmClient<I>
where
	I: IsmpHost + Send + Sync + 'static,
{
	pub async fn new(host: Option<I>, config: EvmConfig) -> Result<Self, anyhow::Error> {
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

		let provider = Provider::<Http>::try_from(config.rpc_url.clone())?;
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

		let latest_height = client.get_block_number().await?.as_u64();
		Ok(Self {
			host,
			client,
			signer,
			address,
			consensus_state_id,
			state_machine: config.state_machine,
			initial_height: latest_height,
			config: config_clone,
			chain_id,
		})
	}

	pub async fn events(&self, from: u64, to: u64) -> Result<Vec<Event>, anyhow::Error> {
		let client = Arc::new(self.client.clone());
		let contract = EvmHost::new(self.config.ismp_host, client);
		let events = contract
			.events()
			.address(self.config.ismp_host.into())
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
	pub async fn set_consensus_state(&self, consensus_state: Vec<u8>) -> Result<(), anyhow::Error> {
		let contract = EvmHost::new(self.config.ismp_host, self.signer.clone());
		let call = contract.set_consensus_state(consensus_state.clone().into());

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
		let contract = PingModule::new(address, self.signer.clone());
		let call = contract.dispatch_to_parachain(para_id.into());

		let gas = call.estimate_gas().await?;
		call.gas(gas).send().await?.await?;

		Ok(())
	}

	pub async fn set_latest_finalized_height<P: IsmpProvider + 'static>(
		&mut self,
		counterparty: &P,
	) -> Result<(), anyhow::Error> {
		self.initial_height =
			counterparty.query_latest_height(self.state_machine_id()).await?.into();

		log::info!("Initialized height for {:?} at {}", self.state_machine, self.initial_height);

		Ok(())
	}

	pub fn request_commitment_key(&self, key: H256) -> (H256, H256) {
		let key = derive_map_key(key.0.to_vec(), REQUEST_COMMITMENTS_SLOT);
		let number = U256::from_big_endian(key.0.as_slice()) + U256::from(1);
		let mut bytes = [0u8; 32];
		number.to_big_endian(&mut bytes);
		(key, H256(bytes))
	}

	pub fn response_commitment_key(&self, key: H256) -> (H256, H256) {
		let key = derive_map_key(key.0.to_vec(), RESPONSE_COMMITMENTS_SLOT);
		let number = U256::from_big_endian(key.0.as_slice()) + U256::from(1);
		let mut bytes = [0u8; 32];
		number.to_big_endian(&mut bytes);
		(key, H256(bytes))
	}

	pub fn request_receipt_key(&self, key: H256) -> H256 {
		derive_map_key(key.0.to_vec(), REQUEST_RECEIPTS_SLOT)
	}

	pub fn response_receipt_key(&self, key: H256) -> Vec<Vec<u8>> {
		let key = derive_map_key(key.0.to_vec(), RESPONSE_RECEIPTS_SLOT);
		let number = U256::from_big_endian(key.0.as_slice()) + U256::from(1);
		let mut bytes = [0u8; 32];
		number.to_big_endian(&mut bytes);

		vec![key.0.to_vec(), bytes.to_vec()]
	}

	pub async fn host_manager(&self) -> Result<H160, anyhow::Error> {
		let contract = EvmHost::new(self.config.ismp_host, self.client.clone());
		let params = contract.host_params().call().await?;
		Ok(params.host_manager)
	}
}

pub fn derive_map_key(mut key: Vec<u8>, slot: u64) -> H256 {
	let mut bytes = [0u8; 32];
	U256::from(slot as u64).to_big_endian(&mut bytes);
	key.extend_from_slice(&bytes);
	keccak_256(&key).into()
}
