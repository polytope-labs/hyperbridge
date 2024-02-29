use crate::{
    providers::{evm_chain::EvmClient, global::Client, hyperbridge::HyperBridgeClient},
    runtime::api::{
        ismp::Event as Ev,
        runtime_types::{frame_system::EventRecord, gargantua_runtime::RuntimeEvent},
    },
};
use anyhow::anyhow;
use codec::Encode;
use ethers::{
    contract::abigen,
    middleware::Middleware,
    types::{H160, U256},
    utils::keccak256,
};
use futures::{Stream, StreamExt, TryFutureExt};
use ismp::{
    consensus::{ConsensusStateId, StateMachineHeight, StateMachineId},
    events::{Event, StateMachineUpdated},
    host::StateMachine,
    router,
    router::{Post, PostResponse},
};
use serde::{Deserialize, Serialize};
use sp_core::storage::{StorageChangeSet, StorageKey};
use std::{collections::BTreeMap, pin::Pin, str::FromStr, time::Duration};
use subxt::{
    ext::{codec, codec::Decode},
    rpc::Subscription,
    tx::TxPayload,
    utils::H256,
    Metadata, OnlineClient, PolkadotConfig,
};
use wasm_bindgen::{prelude::wasm_bindgen, JsError, JsValue};

// ========================================
// TYPES
// ========================================
pub type HyperBridgeConfig = PolkadotConfig;
pub type BoxStream<I> = Pin<Box<dyn Stream<Item = Result<I, anyhow::Error>>>>;
pub type BoxStreamJs<I> = Pin<Box<dyn Stream<Item = Result<I, JsError>>>>;

// ====================================
// ERRORS
// ====================================
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum HyperClientErrors {
    FailedToCreateDestClient,
    FailedToCreateHyperbridgeClient,
    FailedToCreateSourceClient,
    FailedToReadHyperbridgeTimestamp,
    FailedToGetRequestResponseFromHyperbridge,
    RequestIsNotDueForTimeOut,
    ResponseIsNotDueForTimeOut,
}

// =======================================
// DTOs                            =
// =======================================

abigen!(HandlerV1, "./abi/Handler.json", derives(serde::Deserialize, serde::Serialize));
abigen!(EvmHost, "./abi/EvmHost.json", derives(serde::Deserialize, serde::Serialize));

#[derive(Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub source_state_machine: String,
    pub dest_state_machine: String,
    pub source_rpc_url: String,
    pub dest_rpc_url: String,
    pub hyper_bridge_url: String,
    pub destination_ismp_host_address: H160,
    pub source_ismp_host_address: H160,
    pub consensus_state_id_source: ConsensusStateId,
    pub consensus_state_id_dest: ConsensusStateId,
    pub destination_ismp_handler: H160,
    pub source_ismp_handler: H160,
}

#[wasm_bindgen]
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Copy)]
pub enum MessageStatus {
    Pending,
    /// Source state machine has been finalized on hyperbridge
    SourceFinalized,
    /// Message has been delivered to hyperbridge
    HyperbridgeDelivered,
    /// Messaged has been finalized on hyperbridge
    HyperbridgeFinalized,
    /// Delivered to destination
    DestinationDelivered,
    /// Message has timed out
    Timeout,
    /// Message has not timed out
    NotTimedOut,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PostStreamState {
    /// Message has been finalized on source chain
    Pending,
    /// Source state machine has been updated on hyperbridge
    SourceFinalized,
    /// Message has been delivered to hyperbridge
    HyperbridgeDelivered(u64),
    /// Message has been finalized by hyperbridge
    HyperbridgeFinalized,
    /// Message has been delivered to destination
    DestinationDelivered,
    /// Stream has ended, check the message status
    End,
}

#[derive(Serialize, Deserialize)]
pub struct LeafIndexQuery {
    /// Commitment of the request or response
    pub commitment: H256,
}

/// Implements [`TxPayload`] for extrinsic encoding
pub struct Extrinsic {
    /// The pallet name, used to query the metadata
    pallet_name: String,
    /// The call name
    call_name: String,
    /// The encoded pallet call. Note that this should be the pallet call. Not runtime call
    encoded: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ReturnRequestTimeoutMessage {
    pub timeouts: Vec<Post>,
    pub height: StateMachineHeight,
    pub proof: Vec<Vec<u8>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ReturnRequestTimeoutData {
    pub host: H160,
    pub post_request_timeout_message: ReturnRequestTimeoutMessage,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ReturnResponseTimeoutMessage {
    pub timeouts: Vec<PostResponse>,
    pub height: StateMachineHeight,
    pub proof: Vec<Vec<u8>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ReturnResponseTimeoutData {
    pub host: H160,
    pub post_response_timeout_message: ReturnResponseTimeoutMessage,
}

#[derive(Encode, Decode, Clone)]
pub struct EvmStateProof {
    /// Contract account proof
    pub contract_proof: Vec<Vec<u8>>,
    /// A map of storage key to the associated storage proof
    pub storage_proof: BTreeMap<Vec<u8>, Vec<Vec<u8>>>,
}

// =======================================
// IMPLs                            =
// =======================================
impl Extrinsic {
    /// Creates a new extrinsic ready to be sent with subxt.
    pub fn new(
        pallet_name: impl Into<String>,
        call_name: impl Into<String>,
        encoded_call: Vec<u8>,
    ) -> Self {
        Extrinsic {
            pallet_name: pallet_name.into(),
            call_name: call_name.into(),
            encoded: encoded_call,
        }
    }
}

impl TxPayload for Extrinsic {
    fn encode_call_data_to(
        &self,
        metadata: &Metadata,
        out: &mut Vec<u8>,
    ) -> Result<(), subxt::Error> {
        // encode the pallet index
        let pallet = metadata.pallet_by_name_err(&self.pallet_name).unwrap();
        let call_index = pallet.call_variant_by_name(&self.call_name).unwrap().index;
        let pallet_index = pallet.index();
        pallet_index.encode_to(out);
        call_index.encode_to(out);

        // copy the encoded call to out
        out.extend_from_slice(&self.encoded);

        Ok(())
    }
}

impl ClientConfig {
    pub async fn dest_chain(&self) -> Result<impl Client, anyhow::Error> {
        let dest_state_machine = StateMachine::from_str(&self.dest_state_machine).unwrap();

        return match dest_state_machine {
            StateMachine::Bsc | StateMachine::Ethereum(_) | StateMachine::Polygon => {
                let evm_chain = EvmClient::new(
                    self.dest_rpc_url.clone(),
                    self.consensus_state_id_dest,
                    self.destination_ismp_host_address,
                    self.destination_ismp_handler.clone(),
                    self.dest_state_machine.clone(),
                )
                .await?;
                Ok(evm_chain)
            },
            _ => {
                let evm_chain = EvmClient::new(
                    self.dest_rpc_url.clone(),
                    self.consensus_state_id_dest,
                    self.destination_ismp_host_address,
                    self.destination_ismp_handler,
                    self.dest_state_machine.clone(),
                )
                .await?;
                Ok(evm_chain)
            },
        };
    }

    pub async fn source_chain(&self) -> Result<impl Client, anyhow::Error> {
        let source_state_machine: StateMachine =
            StateMachine::from_str(&self.source_state_machine).unwrap();

        return match source_state_machine {
            StateMachine::Bsc | StateMachine::Ethereum(_) | StateMachine::Polygon => {
                let evm_chain = EvmClient::new(
                    self.dest_rpc_url.clone(),
                    self.consensus_state_id_dest,
                    self.source_ismp_host_address,
                    self.source_ismp_handler,
                    self.source_state_machine.clone(),
                )
                .await?;
                Ok(evm_chain)
            },
            _ => {
                let evm_chain = EvmClient::new(
                    self.dest_rpc_url.clone(),
                    self.consensus_state_id_dest,
                    self.source_ismp_host_address,
                    self.source_ismp_handler,
                    self.source_state_machine.clone(),
                )
                .await?;
                Ok(evm_chain)
            },
        };
    }

    pub async fn hyperbridge_client(&self) -> Result<HyperBridgeClient, anyhow::Error> {
        let api =
            OnlineClient::<HyperBridgeConfig>::from_url(self.hyper_bridge_url.clone()).await?;

        Ok(HyperBridgeClient {
            client: api,
            rpc_url: self.hyper_bridge_url.clone(),
            state_machine: StateMachineId {
                state_id: StateMachine::Kusama(4634),
                consensus_state_id: *b"PARA",
            },
        })
    }
}

pub fn to_ismp_event(event: EvmHostEvents) -> Result<Event, anyhow::Error> {
    match event {
        EvmHostEvents::GetRequestEventFilter(get) => Ok(Event::GetRequest(router::Get {
            source: StateMachine::from_str(&String::from_utf8(get.source.0.into())?)
                .map_err(|e| anyhow!("{}", e))?,
            dest: StateMachine::from_str(&String::from_utf8(get.dest.0.into())?)
                .map_err(|e| anyhow!("{}", e))?,
            nonce: get.nonce.low_u64(),
            from: get.from.0.into(),
            keys: get.keys.into_iter().map(|key| key.0.into()).collect(),
            height: get.height.low_u64(),
            timeout_timestamp: get.timeout_timestamp.low_u64(),
            gas_limit: get.gaslimit.low_u64(),
        })),
        EvmHostEvents::PostRequestEventFilter(post) => Ok(Event::PostRequest(router::Post {
            source: StateMachine::from_str(&String::from_utf8(post.source.0.into())?)
                .map_err(|e| anyhow!("{}", e))?,
            dest: StateMachine::from_str(&String::from_utf8(post.dest.0.into())?)
                .map_err(|e| anyhow!("{}", e))?,
            nonce: post.nonce.low_u64(),
            from: post.from.0.into(),
            to: post.to.0.into(),
            timeout_timestamp: post.timeout_timestamp.low_u64(),
            data: post.data.0.into(),
            gas_limit: post.gaslimit.low_u64(),
        })),
        EvmHostEvents::PostResponseEventFilter(resp) =>
            Ok(Event::PostResponse(router::PostResponse {
                post: router::Post {
                    source: StateMachine::from_str(&String::from_utf8(resp.source.0.into())?)
                        .map_err(|e| anyhow!("{}", e))?,
                    dest: StateMachine::from_str(&String::from_utf8(resp.dest.0.into())?)
                        .map_err(|e| anyhow!("{}", e))?,
                    nonce: resp.nonce.low_u64(),
                    from: resp.from.0.into(),
                    to: resp.to.0.into(),
                    timeout_timestamp: resp.timeout_timestamp.low_u64(),
                    data: resp.data.0.into(),
                    gas_limit: resp.gaslimit.low_u64(),
                },
                response: resp.response.0.into(),
                timeout_timestamp: resp.timeout_timestamp.low_u64(),
                gas_limit: resp.res_gaslimit.low_u64(),
            })),
        _ => Err(anyhow!("Unknown event")),
    }
}

pub fn to_state_machine_updated(event: StateMachineUpdatedFilter) -> Event {
    let state_machine_updated = StateMachineUpdated {
        state_machine_id: StateMachineId {
            state_id: StateMachine::Kusama(event.state_machine_id.low_u64() as u32),
            consensus_state_id: Default::default(),
        },
        latest_height: event.height.low_u64(),
    };

    Event::StateMachineUpdated(state_machine_updated)
}
