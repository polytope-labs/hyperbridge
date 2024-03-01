use crate::{
    providers::global::{Client, RequestOrResponse},
    runtime,
    types::{BoxStream, Extrinsic, HyperBridgeConfig, LeafIndexQuery},
};
use anyhow::{anyhow, Error};
use codec::{Decode, Encode};
use core::time::Duration;
use ethers::prelude::{H160, H256};
use futures::stream;
use hex_literal::hex;
use ismp::{
    consensus::{StateCommitment, StateMachineHeight, StateMachineId},
    events::{Event, StateMachineUpdated},
    host::{Ethereum, StateMachine},
    messaging::Message,
    router::{Request, Response},
};
use ismp_solidity_abi::evm_host::PostRequestHandledFilter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use subxt::{config::Header, rpc_params, OnlineClient};

#[derive(Debug, Clone)]
pub struct SubstrateClient<C: subxt::Config + Clone> {
    /// RPC url of a hyperbridge node
    pub rpc_url: String,
    /// State machine
    pub state_machine: StateMachineId,
    /// An instance of Hyper bridge client using the default config
    pub client: OnlineClient<C>,
    pub hashing: HashAlgorithm,
}

impl<C: subxt::Config + Clone> SubstrateClient<C> {
    pub async fn new(
        rpc_url: String,
        state_machine: StateMachineId,
    ) -> Result<Self, anyhow::Error> {
        let client = OnlineClient::<C>::from_url(rpc_url.clone()).await?;

        Ok(Self { rpc_url, client, state_machine })
    }

    pub async fn latest_timestamp(&self) -> Result<Duration, anyhow::Error> {
        let timestamp_key =
            hex!("f0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb").to_vec();
        let response = self
            .client
            .rpc()
            .storage(&timestamp_key, None)
            .await?
            .ok_or_else(|| anyhow!("Failed to fetch timestamp"))?;
        let timestamp: u64 = codec::Decode::decode(&mut response.0.as_slice())?;

        Ok(Duration::from_millis(timestamp))
    }

    pub async fn query_request(&self, commitment: H256) -> Result<Option<Request>, anyhow::Error> {
        let build_leaf_index_query = LeafIndexQuery { commitment };

        let leaf_index_query = rpc_params![alloc::vec![build_leaf_index_query]];
        let hyper_bridge_response: Vec<Request> =
            self.client.rpc().request("ismp_queryRequests", leaf_index_query).await?;

        Ok(hyper_bridge_response.get(0).cloned())
    }

    pub async fn query_response(
        &self,
        commitment: H256,
    ) -> Result<Option<Response>, anyhow::Error> {
        let build_leaf_index_query = LeafIndexQuery { commitment };

        let leaf_index_query = rpc_params![alloc::vec![build_leaf_index_query]];
        let hyper_bridge_response: Vec<Response> =
            self.client.rpc().request("ismp_queryResponses", leaf_index_query).await?;

        Ok(hyper_bridge_response.get(0).cloned())
    }

    async fn query_ismp_events(
        &self,
        previous_height: u64,
        latest_height: u64,
    ) -> Result<Vec<Event>, anyhow::Error> {
        let range = (previous_height + 1)..=latest_height;
        if range.is_empty() {
            return Ok(Default::default());
        }

        #[derive(Clone, Hash, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
        #[serde(untagged)]
        pub enum BlockNumberOrHash<Hash> {
            /// Block hash
            Hash(Hash),
            /// Block number
            Number(u32),
        }

        let params = rpc_params![
            BlockNumberOrHash::<H256>::Number(previous_height.saturating_add(1) as u32),
            BlockNumberOrHash::<H256>::Number(latest_height as u32)
        ];
        let response: HashMap<String, Vec<Event>> =
            self.client.rpc().request("ismp_queryEvents", params).await?;
        let events = response.values().into_iter().cloned().flatten().collect();
        Ok(events)
    }
}

impl<C: subxt::Config + Clone> Client for SubstrateClient<C> {
    async fn query_latest_block_height(&self) -> Result<u64, Error> {
        Ok(self.client.blocks().at_latest().await?.number().into())
    }

    fn state_machine_id(&self) -> StateMachineId {
        self.state_machine
    }

    async fn query_timestamp(&self) -> Result<Duration, Error> {
        self.latest_timestamp().await
    }

    async fn query_request_receipt(&self, request_hash: H256) -> Result<H160, Error> {
        todo!()
    }

    async fn query_state_proof(&self, at: u64, keys: Vec<Vec<u8>>) -> Result<Vec<u8>, Error> {
        /// Contains a scale encoded Mmr Proof or Trie proof
        #[derive(Serialize, Deserialize)]
        pub struct RpcProof {
            /// Scale encoded `MmrProof` or state trie proof `Vec<Vec<u8>>`
            pub proof: Vec<u8>,
            /// Height at which proof was recovered
            pub height: u32,
        }

        let params = rpc_params![at, keys];
        let response: RpcProof = self.client.rpc().request("ismp_queryStateProof", params).await?;
        let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*response.proof)?;
        let proof = SubstrateStateProof { hasher: self.hashing.clone(), storage_proof };
        Ok(response.proof)
    }

    async fn query_response_receipt(&self, request_commitment: H256) -> Result<H160, Error> {
        todo!()
    }

    async fn ismp_events_stream(&self, item: RequestOrResponse) -> Result<BoxStream<Event>, Error> {
        let subscription = self.client.rpc().subscribe_finalized_block_headers().await?;
        let initial_height: u64 = self.client.blocks().at_latest().await?.number().into();
        let stream = stream::unfold(
            (initial_height, subscription, self.clone()),
            move |(mut latest_height, mut subscription, client)| {
                let item = item.clone();
                async move {
                    loop {
                        let header = match subscription.next().await {
                            Some(Ok(header)) => header,
                            Some(Err(err)) => {
                                // log::error!(
                                // 	"Error encountered while watching finalized heads: {err:?}"
                                // );
                                continue;
                            },
                            None => return None,
                        };

                        let events = match client
                            .query_ismp_events(latest_height, header.number().into())
                            .await
                        {
                            Ok(e) => e,
                            Err(err) => {
                                // log::error!("Error encountered while querying ismp events
                                // {err:?}");
                                continue;
                            },
                        };

                        let event = events.into_iter().find_map(|event| {
                            let value = match event.clone() {
                                Event::PostRequest(post) => Some(RequestOrResponse::Request(post)),
                                Event::PostResponse(resp) =>
                                    Some(RequestOrResponse::Response(resp)),
                                _ => None,
                            };

                            if value == Some(item.clone()) {
                                Some(event)
                            } else {
                                None
                            }
                        });

                        let value = match event {
                            Some(event) =>
                                Some((Ok(event), (header.number().into(), subscription, client))),
                            None => {
                                latest_height = header.number().into();
                                continue;
                            },
                        };

                        return value;
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }

    async fn post_request_handled_stream(
        &self,
        _commitment: H256,
    ) -> Result<BoxStream<PostRequestHandledFilter>, Error> {
        Err(anyhow!("Post request handled stream is currently unavailable"))
    }

    async fn query_state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<StateCommitment, Error> {
        let addr = runtime::api::storage().ismp().state_commitments(&height.into());
        let commitment = self
            .client
            .storage()
            .at_latest()
            .await?
            .fetch(&addr)
            .await?
            .ok_or_else(|| anyhow!("State commitment not present for state machine"))?;

        let commitment = StateCommitment {
            timestamp: commitment.timestamp,
            overlay_root: commitment.overlay_root,
            state_root: commitment.state_root,
        };
        Ok(commitment)
    }

    async fn state_machine_update_notification(
        &self,
        counterparty_state_id: StateMachineId,
    ) -> Result<BoxStream<StateMachineUpdated>, Error> {
        let subscription = self.client.rpc().subscribe_finalized_block_headers().await?;
        let initial_height: u64 = self.client.blocks().at_latest().await?.number().into();
        let stream = stream::unfold(
            (initial_height, subscription, self.clone()),
            move |(mut latest_height, mut subscription, client)| async move {
                loop {
                    let header = match subscription.next().await {
                        Some(Ok(header)) => header,
                        Some(Err(err)) => {
                            // log::error!(
                            // 	"Error encountered while watching finalized heads: {err:?}"
                            // );
                            continue;
                        },
                        None => return None,
                    };

                    let events = match client
                        .query_ismp_events(latest_height, header.number().into())
                        .await
                    {
                        Ok(e) => e,
                        Err(err) => {
                            // log::error!("Error encountered while querying ismp events {err:?}");
                            continue;
                        },
                    };

                    let event = events
                        .into_iter()
                        .filter_map(|event| match event {
                            Event::StateMachineUpdated(e)
                                if e.state_machine_id == counterparty_state_id =>
                                Some(e),
                            _ => None,
                        })
                        .max_by(|x, y| x.latest_height.cmp(&y.latest_height));

                    let value = match event {
                        Some(event) =>
                            Some((Ok(event), (header.number().into(), subscription, client))),
                        None => {
                            latest_height = header.number().into();
                            continue;
                        },
                    };

                    return value;
                }
            },
        );

        Ok(Box::pin(stream))
    }

    fn request_commitment_full_key(&self, commitment: H256) -> Vec<u8> {
        let addr = runtime::api::storage().ismp().request_commitments(&commitment);
        self.client.storage().address_bytes(&addr).expect("Infallible")
    }

    fn request_receipt_full_key(&self, commitment: H256) -> Vec<u8> {
        let addr = runtime::api::storage().ismp().request_receipts(&commitment);
        self.client.storage().address_bytes(&addr).expect("Infallible")
    }

    fn response_commitment_full_key(&self, commitment: H256) -> Vec<u8> {
        let addr = runtime::api::storage().ismp().response_commitments(&commitment);
        self.client.storage().address_bytes(&addr).expect("Infallible")
    }

    fn response_receipt_full_key(&self, commitment: H256) -> Vec<u8> {
        let addr = runtime::api::storage().ismp().response_receipts(&commitment);
        self.client.storage().address_bytes(&addr).expect("Infallible")
    }

    fn encode(&self, msg: Message) -> Result<Vec<u8>, Error> {
        let call = vec![msg].encode();
        let hyper_bridge_timeout_extrinsic = Extrinsic::new("Ismp", "handle", call);
        let ext = self.client.tx().create_unsigned(&hyper_bridge_timeout_extrinsic)?;
        Ok(ext.into_encoded())
    }

    async fn submit(&self, msg: Message) -> Result<(), Error> {
        let call = vec![msg].encode();
        let hyper_bridge_timeout_extrinsic = Extrinsic::new("Ismp", "handle", call);
        let ext = self.client.tx().create_unsigned(&hyper_bridge_timeout_extrinsic)?;
        let _ = ext.submit_and_watch().await?.wait_for_in_block().await?;
        Ok(())
    }
}

impl From<runtime::api::runtime_types::ismp::consensus::StateCommitment> for StateCommitment {
    fn from(commitment: runtime::api::runtime_types::ismp::consensus::StateCommitment) -> Self {
        StateCommitment {
            timestamp: commitment.timestamp,
            overlay_root: commitment.overlay_root,
            state_root: commitment.state_root,
        }
    }
}

impl From<runtime::api::runtime_types::ismp::consensus::StateMachineHeight> for StateMachineHeight {
    fn from(
        state_machine_height: runtime::api::runtime_types::ismp::consensus::StateMachineHeight,
    ) -> Self {
        StateMachineHeight {
            id: state_machine_height.id.into(),
            height: state_machine_height.height,
        }
    }
}

impl From<runtime::api::runtime_types::ismp::consensus::StateMachineId> for StateMachineId {
    fn from(
        state_machine_id: runtime::api::runtime_types::ismp::consensus::StateMachineId,
    ) -> Self {
        StateMachineId {
            state_id: state_machine_id.state_id.into(),
            consensus_state_id: state_machine_id.consensus_state_id,
        }
    }
}

impl From<runtime::api::runtime_types::ismp::host::StateMachine> for StateMachine {
    fn from(state_machine_id: runtime::api::runtime_types::ismp::host::StateMachine) -> Self {
        match state_machine_id {
            runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(ethereum) =>
                match ethereum {
                    runtime::api::runtime_types::ismp::host::Ethereum::ExecutionLayer =>
                        StateMachine::Ethereum(Ethereum::ExecutionLayer),
                    runtime::api::runtime_types::ismp::host::Ethereum::Optimism =>
                        StateMachine::Ethereum(Ethereum::Optimism),
                    runtime::api::runtime_types::ismp::host::Ethereum::Arbitrum =>
                        StateMachine::Ethereum(Ethereum::Arbitrum),
                    runtime::api::runtime_types::ismp::host::Ethereum::Base =>
                        StateMachine::Ethereum(Ethereum::Base),
                },
            runtime::api::runtime_types::ismp::host::StateMachine::Polkadot(id) =>
                StateMachine::Polkadot(id),
            runtime::api::runtime_types::ismp::host::StateMachine::Kusama(id) =>
                StateMachine::Kusama(id),
            runtime::api::runtime_types::ismp::host::StateMachine::Grandpa(consensus_state_id) =>
                StateMachine::Grandpa(consensus_state_id),
            runtime::api::runtime_types::ismp::host::StateMachine::Beefy(consensus_state_id) =>
                StateMachine::Beefy(consensus_state_id),
            runtime::api::runtime_types::ismp::host::StateMachine::Polygon => StateMachine::Polygon,
            runtime::api::runtime_types::ismp::host::StateMachine::Bsc => StateMachine::Bsc,
        }
    }
}

impl From<StateMachineHeight> for runtime::api::runtime_types::ismp::consensus::StateMachineHeight {
    fn from(state_machine_height: StateMachineHeight) -> Self {
        runtime::api::runtime_types::ismp::consensus::StateMachineHeight {
            id: state_machine_height.id.into(),
            height: state_machine_height.height,
        }
    }
}

impl From<StateMachineId> for runtime::api::runtime_types::ismp::consensus::StateMachineId {
    fn from(state_machine_id: StateMachineId) -> Self {
        Self {
            state_id: state_machine_id.state_id.into(),
            consensus_state_id: state_machine_id.consensus_state_id,
        }
    }
}

impl From<StateMachine> for runtime::api::runtime_types::ismp::host::StateMachine {
    fn from(state_machine_id: StateMachine) -> Self {
        match state_machine_id {
            StateMachine::Ethereum(ethereum) => match ethereum {
                Ethereum::ExecutionLayer =>
                    runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(
                        runtime::api::runtime_types::ismp::host::Ethereum::ExecutionLayer,
                    ),
                Ethereum::Optimism =>
                    runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(
                        runtime::api::runtime_types::ismp::host::Ethereum::Optimism,
                    ),
                Ethereum::Arbitrum =>
                    runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(
                        runtime::api::runtime_types::ismp::host::Ethereum::Arbitrum,
                    ),
                Ethereum::Base => runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(
                    runtime::api::runtime_types::ismp::host::Ethereum::Base,
                ),
            },
            StateMachine::Polkadot(id) =>
                runtime::api::runtime_types::ismp::host::StateMachine::Polkadot(id),
            StateMachine::Kusama(id) =>
                runtime::api::runtime_types::ismp::host::StateMachine::Kusama(id),
            StateMachine::Grandpa(consensus_state_id) =>
                runtime::api::runtime_types::ismp::host::StateMachine::Grandpa(consensus_state_id),
            StateMachine::Beefy(consensus_state_id) =>
                runtime::api::runtime_types::ismp::host::StateMachine::Beefy(consensus_state_id),

            StateMachine::Polygon => runtime::api::runtime_types::ismp::host::StateMachine::Polygon,
            StateMachine::Bsc => runtime::api::runtime_types::ismp::host::StateMachine::Bsc,
        }
    }
}

/// Hashing algorithm for the state proof
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum HashAlgorithm {
    /// For chains that use keccak as their hashing algo
    Keccak,
    /// For chains that use blake2 as their hashing algo
    Blake2,
}

/// Holds the relevant data needed for state proof verification
#[derive(Debug, Encode, Decode, Clone)]
pub struct SubstrateStateProof {
    /// Algorithm to use for state proof verification
    pub hasher: HashAlgorithm,
    /// Storage proof for the parachain headers
    pub storage_proof: Vec<Vec<u8>>,
}
