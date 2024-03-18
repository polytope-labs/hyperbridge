use crate::{
    providers::interface::{Client, RequestOrResponse},
    types::BoxStream,
};
use ethers::prelude::Middleware;

use crate::{
    providers::interface::WithMetadata,
    types::{EventMetadata, EvmStateProof, SubstrateStateProof},
};
use anyhow::{anyhow, Context, Error};
use core::time::Duration;
use ethers::{
    prelude::{ProviderExt, H160, H256, U256},
    providers::{Http, Provider},
    utils::keccak256,
};
use futures::{stream, StreamExt};
use ismp::{
    consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
    events::{Event, StateMachineUpdated},
    host::StateMachine,
    messaging::{Message, TimeoutMessage},
    router::Request,
};
use ismp_solidity_abi::{
    evm_host::{EvmHost, EvmHostEvents, GetRequest, PostRequestHandledFilter},
    handler::{GetTimeoutMessage, Handler, PostRequestTimeoutMessage, PostResponseTimeoutMessage},
};
use std::{collections::BTreeMap, ops::RangeInclusive, sync::Arc};

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
        state_machine: StateMachine,
    ) -> Result<Self, anyhow::Error> {
        let client = Arc::new(Provider::<Http>::connect(&rpc_url.clone()).await);
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
        use codec::Encode;
        let mut map: BTreeMap<Vec<u8>, Vec<Vec<u8>>> = BTreeMap::new();
        let locations = keys.iter().map(|key| H256::from_slice(key)).collect();
        let proof = self.client.get_proof(self.host_address, locations, Some(at.into())).await?;
        for (index, key) in keys.into_iter().enumerate() {
            map.insert(
                key,
                proof
                    .storage_proof
                    .get(index)
                    .cloned()
                    .ok_or_else(|| {
                        anyhow!("Invalid key supplied, storage proof could not be retrieved")
                    })?
                    .proof
                    .into_iter()
                    .map(|bytes| bytes.0.into())
                    .collect(),
            );
        }

        let state_proof = EvmStateProof {
            contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
            storage_proof: map,
        };
        Ok(state_proof.encode())
    }

    async fn query_response_receipt(&self, request_commitment: H256) -> Result<H160, Error> {
        let host = EvmHost::new(self.host_address, self.client.clone());
        let response_receipt = host.response_receipts(request_commitment.0).call().await?;

        Ok(response_receipt.relayer)
    }

    async fn query_ismp_event(
        &self,
        range: RangeInclusive<u64>,
    ) -> Result<Vec<WithMetadata<Event>>, anyhow::Error> {
        let contract = EvmHost::new(self.host_address, self.client.clone());
        contract
            .events()
            .address(self.host_address.into())
            .from_block(*range.start())
            .to_block(*range.end())
            .query_with_meta()
            .await?
            .into_iter()
            .map(|(event, meta)| {
                Ok(WithMetadata {
                    meta: EventMetadata {
                        block_hash: meta.block_hash,
                        transaction_hash: meta.transaction_hash,
                        block_number: meta.block_number.as_u64(),
                    },
                    event: event.try_into()?,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    async fn ismp_events_stream(
        &self,
        _item: RequestOrResponse,
    ) -> Result<BoxStream<WithMetadata<Event>>, Error> {
        Err(anyhow!("Ismp stream unavailable for evm client"))
    }

    async fn post_request_handled_stream(
        &self,
        commitment: H256,
    ) -> Result<BoxStream<WithMetadata<PostRequestHandledFilter>>, Error> {
        let initial_height = self.client.get_block_number().await?.as_u64();
        let client = self.clone();
        let interval = wasm_timer::Interval::new(Duration::from_secs(30));
        let stream = stream::unfold(
            (initial_height, interval, client),
            move |(latest_height, mut interval, client)| async move {
                let state_machine = client.state_machine;
                interval.next().await;
                let block_number = match client.client.get_block_number().await {
                    Ok(number) => number.low_u64(),
                    Err(err) =>
                        return Some((
                            Err(err).context(format!(
                            "Error encountered fetching latest block number for {state_machine:?}"
                        )),
                            (latest_height, interval, client),
                        )),
                };

                // in case we get old heights, best to ignore them
                if block_number < latest_height {
                    return Some((Ok(None), (block_number, interval, client)))
                }

                let contract = EvmHost::new(client.host_address, client.client.clone());
                let results = match contract
                    .events()
                    .address(client.host_address.into())
                    .from_block(latest_height)
                    .to_block(block_number)
                    .query_with_meta()
                    .await
                {
                    Ok(events) => events,
                    Err(err) =>
                        return Some((
                            Err(err)
                                .context(format!("Failed to query events on {state_machine:?}")),
                            (latest_height, interval, client),
                        )),
                };

                let events = results
                    .into_iter()
                    .filter_map(|(ev, meta)| match ev {
                        EvmHostEvents::PostRequestHandledFilter(filter)
                            if filter.commitment == commitment.0 =>
                            Some(WithMetadata {
                                meta: EventMetadata {
                                    block_hash: meta.block_hash,
                                    transaction_hash: meta.transaction_hash,
                                    block_number: meta.block_number.as_u64(),
                                },
                                event: filter,
                            }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                // we only want the highest event
                Some((Ok(events.last().cloned()), (block_number + 1, interval, client)))
            },
        )
        .filter_map(|item| async move {
            match item {
                Ok(None) => None,
                Ok(Some(event)) => Some(Ok(event)),
                Err(err) => Some(Err(err)),
            }
        });

        Ok(Box::pin(stream))
    }

    async fn state_machine_update_notification(
        &self,
        _counterparty_state_id: StateMachineId,
    ) -> Result<BoxStream<WithMetadata<StateMachineUpdated>>, Error> {
        let initial_height = self.client.get_block_number().await?.as_u64();
        let interval = wasm_timer::Interval::new(Duration::from_secs(30));
        let stream = stream::unfold(
            (initial_height, interval, self.clone()),
            move |(latest_height, mut interval, client)| async move {
                let state_machine = client.state_machine;
                interval.next().await;
                let block_number = match client.client.get_block_number().await {
                    Ok(number) => number.low_u64(),
                    Err(err) =>
                        return Some((
                            Err(err).context(format!(
                            "Error encountered fetching latest block number for {state_machine:?}"
                        )),
                            (latest_height, interval, client),
                        )),
                };

                // in case we get old heights, best to ignore them
                if block_number < latest_height {
                    return Some((Ok(None), (block_number, interval, client)))
                }

                let contract = Handler::new(client.ismp_handler, client.client.clone());
                let results = match contract
                    .events()
                    .address(client.ismp_handler.into())
                    .from_block(latest_height)
                    .to_block(block_number)
                    .query_with_meta()
                    .await
                {
                    Ok(events) => events,
                    Err(err) =>
                        return Some((
                            Err(err)
                                .context(format!("Failed to query events on {state_machine:?}")),
                            (latest_height, interval, client),
                        )),
                };
                let mut events = results
                    .into_iter()
                    .map(|(ev, meta)| WithMetadata {
                        meta: EventMetadata {
                            block_hash: meta.block_hash,
                            transaction_hash: meta.transaction_hash,
                            block_number: meta.block_number.as_u64(),
                        },
                        event: StateMachineUpdated::from(ev),
                    })
                    .collect::<Vec<_>>();
                // we only want the highest event
                events.sort_by(|a, b| a.event.latest_height.cmp(&b.event.latest_height));
                // we only want the highest event
                Some((Ok(events.last().cloned()), (block_number + 1, interval, client)))
            },
        )
        .filter_map(|item| async move {
            match item {
                Ok(None) => None,
                Ok(Some(event)) => Some(Ok(event)),
                Err(err) => Some(Err(err)),
            }
        });

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
        let state_machine_height = ismp_solidity_abi::shared_types::StateMachineHeight {
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

    fn encode(&self, msg: Message) -> Result<Vec<u8>, Error> {
        let contract = Handler::new(self.ismp_handler, self.client.clone());
        match msg {
            Message::Timeout(TimeoutMessage::Post { timeout_proof, requests }) => {
                let post_requests = requests
                    .into_iter()
                    .filter_map(|req| match req {
                        Request::Post(post) => Some(post.into()),
                        Request::Get(_) => None,
                    })
                    .collect();

                let state_proof: SubstrateStateProof =
                    match codec::Decode::decode(&mut timeout_proof.proof.as_slice()) {
                        Ok(proof) => proof,
                        _ => Err(anyhow!("Error decoding proof"))?,
                    };
                let message = PostRequestTimeoutMessage {
                    timeouts: post_requests,
                    height: ismp_solidity_abi::shared_types::StateMachineHeight {
                        state_machine_id: {
                            match timeout_proof.height.id.state_id {
                                StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
                                _ => Err(anyhow!("Expected polkadot or kusama state machines"))?,
                            }
                        },
                        height: timeout_proof.height.height.into(),
                    },
                    proof: state_proof.storage_proof.into_iter().map(|key| key.into()).collect(),
                };
                let call = contract.handle_post_request_timeouts(self.host_address, message);

                Ok(call.tx.data().cloned().expect("Infallible").to_vec())
            },
            Message::Timeout(TimeoutMessage::PostResponse { timeout_proof, responses }) => {
                let post_responses = responses.into_iter().map(|res| res.into()).collect();

                let state_proof: SubstrateStateProof =
                    match codec::Decode::decode(&mut timeout_proof.proof.as_slice()) {
                        Ok(proof) => proof,
                        _ => Err(anyhow!("Expected polkadot or kusama state machines"))?,
                    };
                let message = PostResponseTimeoutMessage {
                    timeouts: post_responses,
                    height: ismp_solidity_abi::shared_types::StateMachineHeight {
                        state_machine_id: {
                            match timeout_proof.height.id.state_id {
                                StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
                                _ => Err(anyhow!("Expected polkadot or kusama state machines"))?,
                            }
                        },
                        height: timeout_proof.height.height.into(),
                    },
                    proof: state_proof.storage_proof.into_iter().map(|key| key.into()).collect(),
                };
                let call = contract.handle_post_response_timeouts(self.host_address, message);
                Ok(call.tx.data().cloned().expect("Infallible").to_vec())
            },
            Message::Timeout(TimeoutMessage::Get { requests }) => {
                let get_requests = requests
                    .into_iter()
                    .filter_map(|req| match req {
                        Request::Get(get) => Some(GetRequest {
                            source: get.source.to_string().as_bytes().to_vec().into(),
                            dest: get.dest.to_string().as_bytes().to_vec().into(),
                            nonce: get.nonce,
                            from: get.from.into(),
                            keys: get.keys.into_iter().map(|key| key.into()).collect(),
                            timeout_timestamp: get.timeout_timestamp,
                            gaslimit: get.gas_limit.into(),
                            height: get.height.into(),
                        }),
                        _ => None,
                    })
                    .collect();

                let message = GetTimeoutMessage { timeouts: get_requests };
                let call = contract.handle_get_request_timeouts(self.host_address, message);

                Ok(call.tx.data().cloned().expect("Infallible").to_vec())
            },
            _ => Err(anyhow!("Only timeout messages are suported"))?,
        }
    }

    async fn submit(&self, _msg: Message) -> Result<EventMetadata, Error> {
        Err(anyhow!("Client cannot submit messages"))
    }

    async fn query_state_machine_update_time(
        &self,
        height: StateMachineHeight,
    ) -> Result<Duration, Error> {
        let contract = EvmHost::new(self.host_address, self.client.clone());
        let value =
            contract.state_machine_commitment_update_time(height.try_into()?).call().await?;
        Ok(Duration::from_secs(value.low_u64()))
    }

    async fn query_challenge_period(&self, _id: ConsensusStateId) -> Result<Duration, Error> {
        let contract = EvmHost::new(self.host_address, self.client.clone());
        let value = contract.challenge_period().call().await?;
        Ok(Duration::from_secs(value.low_u64()))
    }
}
