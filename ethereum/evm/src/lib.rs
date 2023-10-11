use crate::{
	abi::{IIsmpHost, IsmpHandler, PingModule, StateMachineUpdatedFilter},
	consts::{REQUEST_COMMITMENTS_SLOT, REQUEST_RECEIPTS_SLOT, RESPONSE_COMMITMENTS_SLOT},
};
use ethabi::ethereum_types::{H256, U256};
use ethers::{
	core::k256::ecdsa::SigningKey,
	prelude::{k256::SecretKey, LocalWallet, MiddlewareBuilder, SignerMiddleware, Wallet},
	providers::{Middleware, Provider, Ws},
	signers::Signer,
};
use ismp::{
	consensus::{ConsensusStateId, StateMachineId},
	events::Event,
	host::{Ethereum, StateMachine},
	messaging::Message,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, keccak_256, Pair, H160};
use std::sync::Arc;
use tesseract_primitives::{queue::PipelineQueue, IsmpHost, IsmpProvider};

pub mod abi;
pub mod arbitrum;
pub mod consts;
mod host;
#[cfg(any(feature = "testing", test))]
pub mod mock;
pub mod optimism;
pub mod provider;
pub mod tx_queue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LatestHeight {
	LastMessaging,
	LatestHeight,
	Const(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmConfig {
	/// WS url for execution client
	pub execution_ws: String,
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
	/// Latest state machine height
	pub latest_height: Option<LatestHeight>,
	/// EVM chain Id.
	pub chain_id: u64,
	/// Block gas limit
	pub gas_limit: u64,
}

impl Default for EvmConfig {
	fn default() -> Self {
		Self {
			execution_ws: Default::default(),
			state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
			consensus_state_id: Default::default(),
			ismp_host: Default::default(),
			handler: Default::default(),
			signer: Default::default(),
			latest_height: Default::default(),
			chain_id: Default::default(),
			gas_limit: Default::default(),
		}
	}
}

/// Core EVM client.
pub struct EvmClient<I> {
	/// Ismp host implementation
	pub host: I,
	/// Execution Rpc client
	pub client: Arc<Provider<Ws>>,
	/// Transaction signer
	pub signer: Arc<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>>,
	/// State Update Event Object
	events:
		Arc<ethers::contract::Event<Arc<Provider<Ws>>, Provider<Ws>, StateMachineUpdatedFilter>>,
	/// Consensus state Id
	consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this client.
	state_machine: StateMachine,
	/// Latest state machine height.
	latest_height: Arc<Mutex<u64>>,
	/// Ismp Host contract address
	ismp_host: H160,
	/// Ismp Handler contract address
	handler: H160,
	/// Block gas limit
	gas_limit: u64,
	/// Transaction submission pipleline
	tx_queue: Option<PipelineQueue<Vec<Message>>>,
}

impl<I> EvmClient<I>
where
	I: IsmpHost + Send + Sync,
{
	pub async fn new<C: IsmpProvider>(
		host: I,
		config: EvmConfig,
		counterparty: &C,
	) -> Result<Self, anyhow::Error> {
		let bytes = from_hex(config.signer.as_str())?;
		let signer = sp_core::ecdsa::Pair::from_seed_slice(&bytes)?;
		let signer = LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
			.with_chain_id(config.chain_id);
		let provider = Provider::<Ws>::connect_with_reconnects(config.execution_ws, 1000).await?;
		let client = Arc::new(provider.clone());
		let contract = IsmpHandler::new(config.handler, client.clone());
		let events = Arc::new(contract.events().address(config.handler.into()));
		let signer = Arc::new(provider.with_signer(signer));
		let consensus_state_id = {
			let mut consensus_state_id: ConsensusStateId = Default::default();
			consensus_state_id.copy_from_slice(config.consensus_state_id.as_bytes());
			consensus_state_id
		};

		let latest_height = match config.latest_height {
			Some(LatestHeight::LastMessaging) | None => {
				let state_machine_id =
					StateMachineId { state_id: config.state_machine, consensus_state_id };
				counterparty.query_latest_messaging_height(state_machine_id).await?
			},
			Some(LatestHeight::LatestHeight) => client.get_block_number().await?.as_u64(),
			Some(LatestHeight::Const(height)) => height,
		};
		Ok(Self {
			host,
			client,
			signer,
			events,
			consensus_state_id,
			state_machine: config.state_machine,
			latest_height: Arc::new(Mutex::new(latest_height)),
			ismp_host: config.ismp_host,
			handler: config.handler,
			tx_queue: None,
			gas_limit: config.gas_limit,
		})
	}

	pub async fn events(&self, from: u64, to: u64) -> Result<Vec<Event>, anyhow::Error> {
		let client = Arc::new(self.client.clone());
		let contract = IIsmpHost::new(self.ismp_host, client);
		let events = contract
			.events()
			.address(self.ismp_host.into())
			.from_block(from)
			.to_block(to)
			.query()
			.await?
			.into_iter()
			.map(|ev| ev.try_into())
			.collect::<Result<_, _>>()?;
		Ok(events)
	}

	/// Set the consensus state on the IsmpHost
	pub async fn set_consensus_state(&self, consensus_state: Vec<u8>) -> Result<(), anyhow::Error> {
		let contract = IIsmpHost::new(self.ismp_host, self.signer.clone());
		let call = contract.set_consensus_state(consensus_state.clone().into());

		// let gas = call.estimate_gas().await?; // todo: fix estimate gas
		// dbg!(gas);
		call.gas(10_000_000).send().await?.await?;

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

		// let gas = call.estimate_gas().await?; // todo: fix estimate gas
		// dbg!(gas);
		call.gas(10_000_000).send().await?.await?;

		Ok(())
	}

	pub fn request_commitment_key(&self, key: H256) -> H256 {
		// commitment is mapped to a  bool
		derive_map_key(key.0.to_vec(), REQUEST_COMMITMENTS_SLOT)
	}

	pub fn response_commitment_key(&self, key: H256) -> H256 {
		// commitment is mapped to a  bool
		derive_map_key(key.0.to_vec(), RESPONSE_COMMITMENTS_SLOT)
	}

	pub fn request_receipt_key(&self, key: H256) -> H256 {
		// commitment is mapped to a  bool
		derive_map_key(key.0.to_vec(), REQUEST_RECEIPTS_SLOT)
	}

	pub fn set_queue(&mut self, tx_queue: PipelineQueue<Vec<Message>>) {
		self.tx_queue = Some(tx_queue);
	}
}

fn derive_map_key(mut key: Vec<u8>, slot: u64) -> H256 {
	let mut bytes = [0u8; 32];
	U256::from(slot as u64).to_big_endian(&mut bytes);
	key.extend_from_slice(&bytes);
	keccak_256(&key).into()
}
