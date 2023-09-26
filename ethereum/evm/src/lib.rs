use crate::{
    abi::{IIsmpHost, IsmpHandler, MockModule, StateMachineUpdatedFilter},
    consts::{REQUEST_COMMITMENTS_SLOT, REQUEST_RECEIPTS_SLOT, RESPONSE_COMMITMENTS_SLOT},
};
use ethabi::ethereum_types::{H256, U256};
use ethers::{
    core::k256::ecdsa::SigningKey,
    prelude::{k256::SecretKey, LocalWallet, MiddlewareBuilder, SignerMiddleware, Wallet},
    providers::{Provider, Ws},
    signers::Signer,
};
use ismp::{
    consensus::ConsensusStateId,
    events::Event,
    host::{Ethereum, StateMachine},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, keccak_256, Pair, H160};
use std::sync::Arc;
use tesseract_primitives::IsmpHost;

pub mod abi;
pub mod arbitrum;
pub mod consts;
mod host;
#[cfg(test)]
pub mod mock;
pub mod optimism;
pub mod provider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmConfig {
    /// WS url for execution client
    pub execution: String,
    /// State machine Identifier for this client on it's counterparties.
    pub state_machine: StateMachine,
    /// Consensus state id for the consensus client on counterparty chain
    pub consensus_state_id: String,
    /// Ismp Host contract address
    pub ismp_host_address: H160,
    /// Ismp Handler contract address
    pub handler_address: H160,
    /// Relayer account seed
    pub signer: String,
    /// Latest state machine height
    pub latest_state_machine_height: u64,
    /// Block gas limit
    pub gas_limit: u64,
}

impl Default for EvmConfig {
    fn default() -> Self {
        Self {
            execution: Default::default(),
            state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
            consensus_state_id: Default::default(),
            ismp_host_address: Default::default(),
            handler_address: Default::default(),
            signer: Default::default(),
            latest_state_machine_height: Default::default(),
            gas_limit: Default::default(),
        }
    }
}

/// Core EVM client.
pub struct EvmClient<I> {
    /// Ismp host implementation
    pub host: I,
    /// Execution Rpc client
    #[cfg(feature = "testing")]
    pub client: Arc<Provider<Ws>>,
    #[cfg(not(feature = "testing"))]
    client: Arc<Provider<Ws>>,
    /// Transaction signer
    signer: Arc<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>>,
    /// State Update Event Object
    events:
        Arc<ethers::contract::Event<Arc<Provider<Ws>>, Provider<Ws>, StateMachineUpdatedFilter>>,
    /// Consensus state Id
    consensus_state_id: ConsensusStateId,
    /// State machine Identifier for this client.
    state_machine: StateMachine,
    /// Latest state machine height.
    latest_state_machine_height: Arc<Mutex<u64>>,
    /// Ismp Host contract address
    ismp_host_address: H160,
    /// Ismp Handler contract address
    handler_address: H160,
    /// Block gas limit
    gas_limit: u64,
}

impl<I> EvmClient<I>
where
    I: IsmpHost + Send + Sync,
{
    pub async fn new(host: I, config: EvmConfig) -> Result<Self, anyhow::Error> {
        let bytes = from_hex(config.signer.as_str())?;
        let signer = sp_core::ecdsa::Pair::from_seed_slice(&bytes)?;
        let signer = LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
            .with_chain_id(32382u64);
        let provider = Provider::<Ws>::connect(config.execution).await?;
        let client = Arc::new(provider.clone());
        let contract = IsmpHandler::new(config.handler_address, client.clone());
        let events = Arc::new(contract.events());
        let signer = provider.with_signer(signer);
        let signer = Arc::new(signer);
        Ok(Self {
            host,
            client,
            signer,
            events,
            consensus_state_id: {
                let mut consensus_state_id: ConsensusStateId = Default::default();
                consensus_state_id.copy_from_slice(config.consensus_state_id.as_bytes());
                consensus_state_id
            },
            state_machine: config.state_machine,
            latest_state_machine_height: Arc::new(Mutex::new(config.latest_state_machine_height)),
            ismp_host_address: config.ismp_host_address,
            handler_address: config.handler_address,
            gas_limit: config.gas_limit,
        })
    }

    pub async fn events(&self, from: u64, to: u64) -> Result<Vec<Event>, anyhow::Error> {
        let client = Arc::new(self.client.clone());
        let contract = IIsmpHost::new(self.ismp_host_address, client);
        let events = contract
            .events()
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
        let contract = IIsmpHost::new(self.ismp_host_address, self.signer.clone());
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
        let contract = MockModule::new(address, self.signer.clone());
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
}

fn derive_map_key(mut key: Vec<u8>, slot: u64) -> H256 {
    let mut bytes = [0u8; 32];
    U256::from(slot as u64).to_big_endian(&mut bytes);
    key.extend_from_slice(&bytes);
    keccak_256(&key).into()
}
