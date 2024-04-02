// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![deny(missing_docs)]

//! RPC Implementation for the Interoperable State Machine Protocol

use jsonrpsee::{
    core::{Error as RpcError, RpcResult as Result},
    proc_macros::rpc,
    types::{error::CallError, ErrorObject},
};

use codec::{Decode, Encode};
use ismp::{
    consensus::{ConsensusClientId, StateMachineId},
    events::{Event, StateMachineUpdated},
    router::{Request, Response},
};
use pallet_ismp::{
    child_trie::CHILD_TRIE_PREFIX,
    mmr::primitives::{Leaf, NodeIndex},
    primitives::{LeafIndexAndPos, LeafIndexQuery},
    ProofKeys,
};
use pallet_ismp_runtime_api::IsmpRuntimeApi;
use sc_client_api::{Backend, BlockBackend, ChildInfo, ProofProvider, StateBackend};
use serde::{Deserialize, Serialize};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_core::{
    offchain::{storage::OffchainDb, OffchainDbExt, OffchainStorage},
    H256,
};
use sp_runtime::traits::{Block as BlockT, Hash, Header};
use sp_trie::LayoutV0;
use std::{collections::HashMap, fmt::Display, sync::Arc};
use trie_db::{Recorder, Trie, TrieDBBuilder};

/// A type that could be a block number or a block hash
#[derive(Clone, Hash, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BlockNumberOrHash<Hash> {
    /// Block hash
    Hash(Hash),
    /// Block number
    Number(u32),
}

impl<Hash: std::fmt::Debug> Display for BlockNumberOrHash<Hash> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockNumberOrHash::Hash(hash) => write!(f, "{:?}", hash),
            BlockNumberOrHash::Number(block_num) => write!(f, "{}", block_num),
        }
    }
}

/// An MMR proof data for a group of leaves.
#[derive(codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
pub struct MmrProof<Hash> {
    /// The positions and leaf indices the proof is for.
    pub leaf_positions_and_indices: Vec<LeafIndexAndPos>,
    /// Number of leaves in MMR, when the proof was generated.
    pub leaf_count: NodeIndex,
    /// Proof elements (hashes of siblings of inner nodes on the path to the leaf).
    pub items: Vec<Hash>,
}

/// Contains a scale encoded Mmr Proof or Trie proof
#[derive(Serialize, Deserialize)]
pub struct Proof {
    /// Scale encoded `MmrProof` or state trie proof `Vec<Vec<u8>>`
    pub proof: Vec<u8>,
    /// Height at which proof was recovered
    pub height: u32,
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_error(e: impl std::fmt::Display) -> RpcError {
    RpcError::Call(CallError::Custom(ErrorObject::owned(
        9876, // no real reason for this value
        "Something wrong",
        Some(format!("{}", e)),
    )))
}

/// Relevant transaction metadata for an event
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct EventMetadata {
    /// The hash of the block where the event was emitted
    pub block_hash: H256,
    /// The hash of the extrinsic responsible for the event
    pub transaction_hash: H256,
    /// The block number where the event was emitted
    pub block_number: u64,
}

/// Holds an event along with relevant metadata about the event
#[derive(Serialize, Deserialize)]
pub struct EventWithMetadata {
    /// The event metdata
    pub meta: EventMetadata,
    /// The event in question
    pub event: Event,
}

/// ISMP RPC methods.
#[rpc(client, server)]
pub trait IsmpApi<Hash>
where
    Hash: PartialEq + Eq + std::hash::Hash,
{
    /// Query full request data from the ismp pallet
    #[method(name = "ismp_queryRequests")]
    fn query_requests(&self, query: Vec<LeafIndexQuery>) -> Result<Vec<Request>>;

    /// Query full response data from the ismp pallet
    #[method(name = "ismp_queryResponses")]
    fn query_responses(&self, query: Vec<LeafIndexQuery>) -> Result<Vec<Response>>;

    /// Query mmr proof for some commitments
    #[method(name = "ismp_queryMmrProof")]
    fn query_mmr_proof(&self, height: u32, keys: ProofKeys) -> Result<Proof>;

    /// Query state proof from global state trie
    #[method(name = "ismp_queryStateProof")]
    fn query_state_proof(&self, height: u32, keys: Vec<Vec<u8>>) -> Result<Proof>;

    /// Query pallet ismp child trie proof
    #[method(name = "ismp_queryChildTrieProof")]
    fn query_child_trie_proof(&self, height: u32, keys: Vec<Vec<u8>>) -> Result<Proof>;

    /// Query scale encoded consensus state
    #[method(name = "ismp_queryConsensusState")]
    fn query_consensus_state(
        &self,
        height: Option<u32>,
        client_id: ConsensusClientId,
    ) -> Result<Vec<u8>>;

    /// Query timestamp of when this client was last updated in seconds
    #[method(name = "ismp_queryConsensusUpdateTime")]
    fn query_consensus_update_time(&self, client_id: ConsensusClientId) -> Result<u64>;

    /// Query the challenge period for client
    #[method(name = "ismp_queryChallengePeriod")]
    fn query_challenge_period(&self, client_id: ConsensusClientId) -> Result<u64>;

    /// Query the latest height for a state machine
    #[method(name = "ismp_queryStateMachineLatestHeight")]
    fn query_state_machine_latest_height(&self, id: StateMachineId) -> Result<u64>;

    /// Query ISMP Events that were deposited in a series of blocks
    /// Using String keys because HashMap fails to deserialize when key is not a String
    #[method(name = "ismp_queryEvents")]
    fn query_events(
        &self,
        from: BlockNumberOrHash<Hash>,
        to: BlockNumberOrHash<Hash>,
    ) -> Result<HashMap<String, Vec<Event>>>;

    /// Query ISMP Events that were deposited in a series of blocks
    /// Using String keys because HashMap fails to deserialize when key is not a String
    #[method(name = "ismp_queryEventsWithMetadata")]
    fn query_events_with_metadata(
        &self,
        from: BlockNumberOrHash<Hash>,
        to: BlockNumberOrHash<Hash>,
    ) -> Result<HashMap<String, Vec<EventWithMetadata>>>;
}

/// An implementation of ISMP specific RPC methods.
pub struct IsmpRpcHandler<C, B, S, T> {
    client: Arc<C>,
    backend: Arc<T>,
    offchain_db: OffchainDb<S>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B, S, T> IsmpRpcHandler<C, B, S, T> {
    /// Create new `IsmpRpcHandler` with the given reference to the client.
    pub fn new(client: Arc<C>, backend: Arc<T>, offchain_storage: S) -> Self {
        Self {
            client,
            offchain_db: OffchainDb::new(offchain_storage),
            backend,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, S, T> IsmpApiServer<Block::Hash> for IsmpRpcHandler<C, Block, S, T>
where
    Block: BlockT,
    S: OffchainStorage + Clone + Send + Sync + 'static,
    T: Backend<Block> + Send + Sync + 'static,
    C: Send
        + Sync
        + 'static
        + ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + ProofProvider<Block>
        + BlockBackend<Block>,
    C::Api: IsmpRuntimeApi<Block, Block::Hash>,
    Block::Hash: Into<H256>,
    u64: From<<Block::Header as Header>::Number>,
{
    fn query_requests(&self, query: Vec<LeafIndexQuery>) -> Result<Vec<Request>> {
        let mut api = self.client.runtime_api();
        api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
        let at = self.client.info().best_hash;
        api.get_requests(at, query.into_iter().map(|query| query.commitment).collect())
            .map_err(|_| runtime_error_into_rpc_error("Error fetching requests"))
    }

    fn query_responses(&self, query: Vec<LeafIndexQuery>) -> Result<Vec<Response>> {
        let mut api = self.client.runtime_api();
        api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
        let at = self.client.info().best_hash;
        api.get_responses(at, query.into_iter().map(|query| query.commitment).collect())
            .map_err(|_| runtime_error_into_rpc_error("Error fetching responses"))
    }

    fn query_mmr_proof(&self, height: u32, keys: ProofKeys) -> Result<Proof> {
        let mut api = self.client.runtime_api();
        api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
        let at = self
            .client
            .block_hash(height.into())
            .ok()
            .flatten()
            .ok_or_else(|| runtime_error_into_rpc_error("invalid block height provided"))?;
        let (_, proof): (Vec<Leaf>, pallet_ismp::primitives::Proof<Block::Hash>) = api
            .generate_proof(at, keys)
            .map_err(|_| runtime_error_into_rpc_error("Error calling runtime api"))?
            .map_err(|_| runtime_error_into_rpc_error("Error generating mmr proof"))?;
        let proof = MmrProof {
            leaf_positions_and_indices: proof.leaf_positions,
            leaf_count: proof.leaf_count,
            items: proof.items,
        };
        Ok(Proof { proof: proof.encode(), height })
    }

    fn query_state_proof(&self, height: u32, keys: Vec<Vec<u8>>) -> Result<Proof> {
        let at = self.client.block_hash(height.into()).ok().flatten().ok_or_else(|| {
            runtime_error_into_rpc_error("Could not find valid blockhash for provided height")
        })?;
        let proof: Vec<_> = self
            .client
            .read_proof(at, &mut keys.iter().map(|key| key.as_slice()))
            .map(|proof| proof.into_iter_nodes().collect())
            .map_err(|_| runtime_error_into_rpc_error("Error generating state proof"))?;
        Ok(Proof { proof: proof.encode(), height })
    }

    fn query_child_trie_proof(&self, height: u32, keys: Vec<Vec<u8>>) -> Result<Proof> {
        let at = self.client.block_hash(height.into()).ok().flatten().ok_or_else(|| {
            runtime_error_into_rpc_error("Could not find valid blockhash for provided height")
        })?;
        let child_info = ChildInfo::new_default(CHILD_TRIE_PREFIX);
        let storage_proof = self
            .client
            .read_child_proof(at, &child_info, &mut keys.iter().map(|key| key.as_slice()))
            .map_err(|_| runtime_error_into_rpc_error("Error generating child trie proof"))?;
        let state = self
            .backend
            .state_at(at)
            .map_err(|_| runtime_error_into_rpc_error("Error accessing state backend"))?;
        let child_root = state
            .storage(child_info.prefixed_storage_key().as_slice())
            .map_err(|_| runtime_error_into_rpc_error("Error reading child trie root"))?
            .map(|r| {
                let mut hash = <<Block::Header as Header>::Hashing as Hash>::Output::default();

                // root is fetched from DB, not writable by runtime, so it's always valid.
                hash.as_mut().copy_from_slice(&r[..]);

                hash
            })
            .ok_or_else(|| runtime_error_into_rpc_error("Error reading child trie root"))?;

        let db = storage_proof.into_memory_db::<<Block::Header as Header>::Hashing>();

        let mut recorder = Recorder::<LayoutV0<<Block::Header as Header>::Hashing>>::default();
        let trie =
            TrieDBBuilder::<LayoutV0<<Block::Header as Header>::Hashing>>::new(&db, &child_root)
                .with_recorder(&mut recorder)
                .build();
        for key in keys {
            let _ = trie
                .get(&key)
                .map_err(|_| runtime_error_into_rpc_error("Error generating child trie proof"))?;
        }

        let proof_nodes = recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();
        Ok(Proof { proof: proof_nodes.encode(), height })
    }

    fn query_consensus_state(
        &self,
        height: Option<u32>,
        client_id: ConsensusClientId,
    ) -> Result<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = height
            .and_then(|height| self.client.block_hash(height.into()).ok().flatten())
            .unwrap_or(self.client.info().best_hash);
        api.consensus_state(at, client_id)
            .ok()
            .flatten()
            .ok_or_else(|| runtime_error_into_rpc_error("Error fetching Consensus state"))
    }

    fn query_consensus_update_time(&self, client_id: ConsensusClientId) -> Result<u64> {
        let api = self.client.runtime_api();
        let at = self.client.info().best_hash;
        api.consensus_update_time(at, client_id)
            .ok()
            .flatten()
            .ok_or_else(|| runtime_error_into_rpc_error("Error fetching Consensus update time"))
    }

    fn query_challenge_period(&self, client_id: ConsensusClientId) -> Result<u64> {
        let api = self.client.runtime_api();
        let at = self.client.info().best_hash;
        api.challenge_period(at, client_id)
            .ok()
            .flatten()
            .ok_or_else(|| runtime_error_into_rpc_error("Error fetching Challenge period"))
    }

    fn query_state_machine_latest_height(&self, id: StateMachineId) -> Result<u64> {
        let api = self.client.runtime_api();
        let at = self.client.info().best_hash;
        api.latest_state_machine_height(at, id).ok().flatten().ok_or_else(|| {
            runtime_error_into_rpc_error("Error fetching latest state machine height")
        })
    }

    fn query_events(
        &self,
        from: BlockNumberOrHash<Block::Hash>,
        to: BlockNumberOrHash<Block::Hash>,
    ) -> Result<HashMap<String, Vec<Event>>> {
        let mut events = HashMap::new();
        let to =
            match to {
                BlockNumberOrHash::Hash(block_hash) => block_hash,
                BlockNumberOrHash::Number(block_number) =>
                    self.client.block_hash(block_number.into()).ok().flatten().ok_or_else(|| {
                        runtime_error_into_rpc_error("Invalid block number provided")
                    })?,
            };

        let from =
            match from {
                BlockNumberOrHash::Hash(block_hash) => block_hash,
                BlockNumberOrHash::Number(block_number) =>
                    self.client.block_hash(block_number.into()).ok().flatten().ok_or_else(|| {
                        runtime_error_into_rpc_error("Invalid block number provided")
                    })?,
            };

        let from_block = self
            .client
            .header(from)
            .map_err(|e| runtime_error_into_rpc_error(e.to_string()))?
            .ok_or_else(|| runtime_error_into_rpc_error("Invalid block number or hash provided"))?;

        let mut header = self
            .client
            .header(to)
            .map_err(|e| runtime_error_into_rpc_error(e.to_string()))?
            .ok_or_else(|| runtime_error_into_rpc_error("Invalid block number or hash provided"))?;

        while header.number() >= from_block.number() {
            let mut api = self.client.runtime_api();
            api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
            let at = header.hash();

            let mut request_commitments = vec![];
            let mut response_commitments = vec![];
            let mut temp: Vec<Event> = api
                .block_events(at)
                .map_err(|e| {
                    runtime_error_into_rpc_error(format!("failed to read block events {:?}", e))
                })?
                .into_iter()
                .filter_map(|event| match event {
                    pallet_ismp::events::Event::Request { commitment, .. } => {
                        request_commitments.push(commitment);
                        None
                    },
                    pallet_ismp::events::Event::Response { commitment, .. } => {
                        response_commitments.push(commitment);
                        None
                    },
                    pallet_ismp::events::Event::StateMachineUpdated {
                        state_machine_id,
                        latest_height,
                    } => Some(Event::StateMachineUpdated(StateMachineUpdated {
                        state_machine_id,
                        latest_height,
                    })),
                    pallet_ismp::events::Event::PostRequestHandled(handled) =>
                        Some(Event::PostRequestHandled(handled)),
                    pallet_ismp::events::Event::PostResponseHandled(handled) =>
                        Some(Event::PostResponseHandled(handled)),
                    pallet_ismp::events::Event::GetRequestHandled(handled) =>
                        Some(Event::GetRequestHandled(handled)),
                    pallet_ismp::events::Event::PostRequestTimeoutHandled(handled) =>
                        Some(Event::PostRequestTimeoutHandled(handled)),
                    pallet_ismp::events::Event::PostResponseTimeoutHandled(handled) =>
                        Some(Event::PostResponseTimeoutHandled(handled)),
                    pallet_ismp::events::Event::GetRequestTimeoutHandled(handled) =>
                        Some(Event::GetRequestTimeoutHandled(handled)),
                })
                .collect();

            let request_events = api
                .get_requests(at, request_commitments)
                .map_err(|_| runtime_error_into_rpc_error("Error fetching requests"))?
                .into_iter()
                .map(|req| match req {
                    Request::Post(post) => Event::PostRequest(post),
                    Request::Get(get) => Event::GetRequest(get),
                });

            let response_events = api
                .get_responses(at, response_commitments)
                .map_err(|_| runtime_error_into_rpc_error("Error fetching response"))?
                .into_iter()
                .filter_map(|res| match res {
                    Response::Post(post) => Some(Event::PostResponse(post)),
                    _ => None,
                });

            temp.extend(request_events);
            temp.extend(response_events);

            events.insert(header.hash().to_string(), temp);
            header = self
                .client
                .header(*header.parent_hash())
                .map_err(|e| runtime_error_into_rpc_error(e.to_string()))?
                .ok_or_else(|| {
                    runtime_error_into_rpc_error("Invalid block number or hash provided")
                })?;
        }
        Ok(events)
    }

    fn query_events_with_metadata(
        &self,
        from: BlockNumberOrHash<Block::Hash>,
        to: BlockNumberOrHash<Block::Hash>,
    ) -> Result<HashMap<String, Vec<EventWithMetadata>>> {
        let mut events = HashMap::new();
        let to =
            match to {
                BlockNumberOrHash::Hash(block_hash) => block_hash,
                BlockNumberOrHash::Number(block_number) =>
                    self.client.block_hash(block_number.into()).ok().flatten().ok_or_else(|| {
                        runtime_error_into_rpc_error("Invalid block number provided")
                    })?,
            };

        let from =
            match from {
                BlockNumberOrHash::Hash(block_hash) => block_hash,
                BlockNumberOrHash::Number(block_number) =>
                    self.client.block_hash(block_number.into()).ok().flatten().ok_or_else(|| {
                        runtime_error_into_rpc_error("Invalid block number provided")
                    })?,
            };

        let from_block = self
            .client
            .header(from)
            .map_err(|e| runtime_error_into_rpc_error(e.to_string()))?
            .ok_or_else(|| runtime_error_into_rpc_error("Invalid block number or hash provided"))?;

        let mut header = self
            .client
            .header(to)
            .map_err(|e| runtime_error_into_rpc_error(e.to_string()))?
            .ok_or_else(|| runtime_error_into_rpc_error("Invalid block number or hash provided"))?;

        while header.number() >= from_block.number() {
            let mut api = self.client.runtime_api();
            api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
            let at = header.hash();

            let block_events = api.block_events_with_metadata(at).map_err(|e| {
                runtime_error_into_rpc_error(format!("failed to read block events {:?}", e))
            })?;

            let mut temp = vec![];

            for (event, index) in block_events {
                let event = match event {
                    pallet_ismp::events::Event::Request { commitment, .. } => api
                        .get_requests(at, vec![commitment])
                        .map_err(|_| runtime_error_into_rpc_error("Error fetching requests"))?
                        .into_iter()
                        .map(|req| match req {
                            Request::Post(post) => Event::PostRequest(post),
                            Request::Get(get) => Event::GetRequest(get),
                        })
                        .next(),
                    pallet_ismp::events::Event::Response { commitment, .. } => api
                        .get_responses(at, vec![commitment])
                        .map_err(|_| runtime_error_into_rpc_error("Error fetching response"))?
                        .into_iter()
                        .filter_map(|res| match res {
                            Response::Post(post) => Some(Event::PostResponse(post)),
                            _ => None,
                        })
                        .next(),
                    pallet_ismp::events::Event::StateMachineUpdated {
                        state_machine_id,
                        latest_height,
                    } => Some(Event::StateMachineUpdated(StateMachineUpdated {
                        state_machine_id,
                        latest_height,
                    })),
                    pallet_ismp::events::Event::PostRequestHandled(handled) =>
                        Some(Event::PostRequestHandled(handled)),
                    pallet_ismp::events::Event::PostResponseHandled(handled) =>
                        Some(Event::PostResponseHandled(handled)),
                    pallet_ismp::events::Event::GetRequestHandled(handled) =>
                        Some(Event::GetRequestHandled(handled)),
                    pallet_ismp::events::Event::PostRequestTimeoutHandled(handled) =>
                        Some(Event::PostRequestTimeoutHandled(handled)),
                    pallet_ismp::events::Event::PostResponseTimeoutHandled(handled) =>
                        Some(Event::PostResponseTimeoutHandled(handled)),
                    pallet_ismp::events::Event::GetRequestTimeoutHandled(handled) =>
                        Some(Event::GetRequestTimeoutHandled(handled)),
                };

                if let Some(event) = event {
                    // get the block extrinsics
                    let extrinsic = self
                        .client
                        .block_body(at)
                        .map_err(|err| {
                            runtime_error_into_rpc_error(format!(
                                "Error fetching extrinsic for block {at:?}: {err:?}"
                            ))
                        })?
                        .ok_or_else(|| {
                            runtime_error_into_rpc_error(format!(
                                "No extrinsics found for block {at:?}"
                            ))
                        })?
                        // using swap remove should be fine unless the node is in an inconsistent
                        // state
                        .swap_remove(index as usize);

                    let extrinsic =
                        Vec::<u8>::decode(&mut extrinsic.encode().as_slice()).map_err(|err| {
                            runtime_error_into_rpc_error(format!(
                                "Could not decode extrinsic with index {index:?}: {err:?}"
                            ))
                        })?;
                    let extrinsic_hash =
                        <Block::Header as Header>::Hashing::hash(extrinsic.as_slice());
                    temp.push(EventWithMetadata {
                        meta: EventMetadata {
                            block_hash: at.into(),
                            transaction_hash: extrinsic_hash.into(),
                            block_number: u64::from(*header.number()),
                        },
                        event,
                    });
                }
            }

            events.insert(header.hash().to_string(), temp);
            header = self
                .client
                .header(*header.parent_hash())
                .map_err(|e| runtime_error_into_rpc_error(e.to_string()))?
                .ok_or_else(|| {
                    runtime_error_into_rpc_error("Invalid block number or hash provided")
                })?;
        }
        Ok(events)
    }
}
