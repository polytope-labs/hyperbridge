use crate::{
    providers::global::{Client, RequestOrResponse},
    types::{
        to_ismp_event, to_state_machine_updated, BoxStream, EvmHost, EvmHostEvents, EvmStateProof,
        HandlerV1, PostRequestHandledFilter, ResponseReceipt, StateMachineUpdatedFilter,
    },
};
use anyhow::{anyhow, Context, Error};
use codec::Encode;
use core::{str::FromStr, time::Duration};
use ethers::{
    middleware::Middleware,
    prelude::{ProviderExt, H160, H256, U256},
    providers::{Http, Provider, Ws},
    types::Address,
    utils::keccak256,
};
use futures::stream;
use gloo_timers::future::TimeoutFuture;
use ismp::{
    consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
    events::{Event, StateMachineUpdated},
    host::StateMachine,
    messaging::{Message, Proof},
};
use std::sync::Arc;

// =======================================
// CONSTANTS                            =
// =======================================
pub const REQUEST_COMMITMENTS_SLOT: u64 = 0;
/// Slot index for response commitments map
pub const RESPONSE_COMMITMENTS_SLOT: u64 = 1;
/// Slot index for requests receipts map
pub const REQUEST_RECEIPTS_SLOT: u64 = 2;
/// Slot index for response receipts map
pub const RESPONSE_RECEIPTS_SLOT: u64 = 3;

#[derive(Debug, Clone)]
pub struct EvmClient {
    // A WS rpc url of the EVM chain
    pub rpc_url: String,
    // Ethers provider instance
    pub client: Arc<Provider<Http>>,
    // Identifies the state machine this EVM client represents
    pub state_machine: StateMachine,
    // This is the Consensus State ID of the chain (e.g. BSC0)
    pub consensus_state_id: ConsensusStateId,
    // Address of the ISMP host of this state machine
    pub host_address: H160,
    // The ISMP handler address
    pub ismp_handler: H160,
}

impl EvmClient {
    // Creates an instance of an EVM client
    pub async fn new(
        rpc_url: String,
        consensus_state_id: ConsensusStateId,
        host_address: H160,
        handler_address: H160,
        state_machine: String,
    ) -> Result<Self, anyhow::Error> {
        let client = Arc::new(Provider::<Http>::connect(&rpc_url.clone()).await);
        let state_machine: StateMachine =
            StateMachine::from_str(&state_machine).map_err(|e| anyhow!("{e:?}"))?;
        Ok(Self {
            rpc_url,
            client,
            state_machine,
            consensus_state_id,
            host_address,
            ismp_handler: handler_address,
        })
    }

    pub fn request_commitment_key(&self, key: H256) -> H256 {
        let key = derive_map_key(key.0.to_vec(), REQUEST_COMMITMENTS_SLOT);
        let number = U256::from_big_endian(key.0.as_slice()) + U256::from(1);
        let mut bytes = [0u8; 32];
        number.to_big_endian(&mut bytes);
        H256::from(bytes)
    }

    pub fn response_commitment_key(&self, key: H256) -> H256 {
        let key = derive_map_key(key.0.to_vec(), RESPONSE_COMMITMENTS_SLOT);
        let number = U256::from_big_endian(key.0.as_slice()) + U256::from(1);
        let mut bytes = [0u8; 32];
        number.to_big_endian(&mut bytes);
        H256::from(bytes)
    }

    pub fn request_receipt_key(&self, key: H256) -> H256 {
        derive_map_key(key.0.to_vec(), REQUEST_RECEIPTS_SLOT)
    }

    pub fn response_receipt_key(&self, key: H256) -> H256 {
        derive_map_key(key.0.to_vec(), RESPONSE_RECEIPTS_SLOT)
    }
}

fn derive_map_key(mut key: Vec<u8>, slot: u64) -> H256 {
    let mut bytes = [0u8; 32];
    U256::from(slot as u64).to_big_endian(&mut bytes);
    key.extend_from_slice(&bytes);
    keccak256(&key).into()
}

impl Client for EvmClient {
    async fn query_latest_block_height(&self) -> Result<u64, anyhow::Error> {
        Ok(self.client.get_block_number().await?.as_u64())
    }

    fn state_machine_id(&self) -> StateMachineId {
        StateMachineId { state_id: self.state_machine, consensus_state_id: self.consensus_state_id }
    }

    async fn query_timestamp(&self) -> Result<Duration, anyhow::Error> {
        let host = EvmHost::new(self.host_address, self.client.clone());
        let current_host_time = host.timestamp().call().await?;
        Ok(Duration::from_secs(current_host_time.as_u64()))
    }

    async fn query_request_receipt(&self, request_hash: H256) -> Result<H160, Error> {
        let host = EvmHost::new(self.host_address, self.client.clone());
        let relayer = host.request_receipts(request_hash.0).call().await?;
        Ok(relayer)
    }

    async fn query_state_proof(&self, at: u64, keys: Vec<Vec<u8>>) -> Result<Vec<u8>, Error> {
        todo!()
    }

    async fn query_response_receipt(&self, request_commitment: H256) -> Result<H160, Error> {
        let host = EvmHost::new(self.host_address, self.client.clone());
        let response_receipt = host.response_receipts(request_commitment.0).call().await?;

        Ok(response_receipt.relayer)
    }

    async fn ismp_events_stream(
        &self,
        _item: RequestOrResponse,
    ) -> Result<BoxStream<Event>, Error> {
        Err(anyhow!("Ismp stream unavailable for evm client"))
    }

    async fn post_request_handled_stream(
        &self,
        commitment: H256,
    ) -> Result<BoxStream<PostRequestHandledFilter>, Error> {
        let initial_height = self.client.get_block_number().await?.as_u64();
        let client = self.clone();

        let stream = stream::unfold(
            (initial_height, client),
            move |(mut latest_height, client)| async move {
                let state_machine = client.state_machine;
                loop {
                    // Wait 30 seconds
                    TimeoutFuture::new(30000).await;
                    let block_number = match client.client.get_block_number().await {
                            Ok(number) => number.low_u64(),
                            Err(err) =>
                                return Some((
                                    Err(err).context(format!(
                                        "Error encountered fetching latest block number for {state_machine:?}"
                                    )),
                                    (latest_height, client),
                                )),
                        };

                    // in case we get old heights, best to ignore them
                    if block_number < latest_height {
                        continue;
                    }

                    let contract = EvmHost::new(client.host_address, client.client.clone());
                    let results = match contract
                        .events()
                        .address(client.host_address.into())
                        .from_block(latest_height)
                        .to_block(block_number)
                        .query()
                        .await
                    {
                        Ok(events) => events,
                        Err(err) =>
                            return Some((
                                Err(err).context(format!(
                                    "Failed to query events on {state_machine:?}"
                                )),
                                (latest_height, client),
                            )),
                    };

                    let events = results
                        .into_iter()
                        .filter_map(|ev| match ev {
                            EvmHostEvents::PostRequestHandledFilter(filter)
                                if filter.commitment == commitment.0 =>
                                Some(filter),
                            _ => None,
                        })
                        .collect::<Vec<PostRequestHandledFilter>>();

                    // we only want the highest event
                    if let Some(event) = events.last() {
                        return Some((Ok(event.clone()), (block_number + 1, client)))
                    } else {
                        latest_height = block_number + 1;
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }

    async fn state_machine_update_notification(
        &self,
        _counterparty_state_id: StateMachineId,
    ) -> Result<BoxStream<StateMachineUpdated>, Error> {
        let initial_height = self.client.get_block_number().await?.as_u64();

        let stream = stream::unfold(
            (initial_height, self.clone()),
            move |(mut latest_height, client)| async move {
                let state_machine = client.state_machine;
                loop {
                    // Wait 30 seconds
                    TimeoutFuture::new(30000).await;
                    let block_number = match client.client.get_block_number().await {
                        Ok(number) => number.low_u64(),
                        Err(err) =>
                            return Some((
                                Err(err).context(format!(
                                    "Error encountered fetching latest block number for {state_machine:?}"
                                )),
                                (latest_height, client),
                            )),
                    };

                    // in case we get old heights, best to ignore them
                    if block_number < latest_height {
                        continue;
                    }

                    let contract = HandlerV1::new(client.ismp_handler, client.client.clone());
                    let results = match contract
                        .events()
                        .address(client.ismp_handler.into())
                        .from_block(latest_height)
                        .to_block(block_number)
                        .query()
                        .await
                    {
                        Ok(events) => events,
                        Err(err) =>
                            return Some((
                                Err(err).context(format!(
                                    "Failed to query events on {state_machine:?}"
                                )),
                                (latest_height, client),
                            )),
                    };
                    let mut events = results
                        .into_iter()
                        .map(|ev| ev.into())
                        .collect::<Vec<StateMachineUpdated>>();
                    // we only want the highest event
                    events.sort_by(|a, b| a.latest_height.cmp(&b.latest_height));
                    if let Some(event) = events.last() {
                        return Some((Ok(event.clone()), (block_number + 1, client)))
                    } else {
                        latest_height = block_number + 1;
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }

    async fn query_state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<StateCommitment, Error> {
        let contract = EvmHost::new(self.host_address, self.client.clone());
        let id = match height.id.state_id {
            StateMachine::Polkadot(para_id) => para_id,
            StateMachine::Kusama(para_id) => para_id,
            _ => Err(anyhow!(
                "Unknown State Machine: {:?} Expected polkadot or kusama state machine",
                height.id.state_id
            ))?,
        };
        let state_machine_height = crate::types::evm_host::StateMachineHeight {
            state_machine_id: id.into(),
            height: height.height.into(),
        };
        let commitment = contract.state_machine_commitment(state_machine_height).call().await?;
        Ok(StateCommitment {
            timestamp: commitment.timestamp.low_u64(),
            overlay_root: Some(commitment.overlay_root.into()),
            state_root: commitment.state_root.into(),
        })
    }

    fn request_commitment_full_key(&self, commitment: H256) -> Vec<u8> {
        self.request_commitment_key(commitment).0.to_vec()
    }

    fn request_receipt_full_key(&self, commitment: H256) -> Vec<u8> {
        self.request_receipt_key(commitment).0.to_vec()
    }

    fn response_commitment_full_key(&self, commitment: H256) -> Vec<u8> {
        self.response_commitment_key(commitment).0.to_vec()
    }

    fn response_receipt_full_key(&self, commitment: H256) -> Vec<u8> {
        self.response_receipt_key(commitment).0.to_vec()
    }

    async fn submit(&self, _msg: Message) -> Result<H256, Error> {
        todo!()
    }
}

impl From<StateMachineUpdatedFilter> for StateMachineUpdated {
    fn from(filter: StateMachineUpdatedFilter) -> Self {
        StateMachineUpdated {
            latest_height: filter.height.as_u64(),
            state_machine_id: StateMachineId {
                state_id: StateMachine::Kusama(filter.state_machine_id.as_u32()),
                consensus_state_id: *b"PARA",
            },
        }
    }
}
