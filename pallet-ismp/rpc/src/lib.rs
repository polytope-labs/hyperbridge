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

use codec::Encode;
use ismp_primitives::{
    mmr::{Leaf, LeafIndex},
    LeafIndexQuery,
};
use ismp_rs::{
    consensus::{ConsensusClientId, StateMachineId},
    router::{Get, Request, Response},
};
use ismp_runtime_api::IsmpRuntimeApi;
use sc_client_api::{BlockBackend, ProofProvider};
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::{collections::HashMap, fmt::Display, sync::Arc};

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

/// Contains a scale encoded Mmr Proof or Trie proof
#[derive(Serialize, Deserialize)]
pub struct Proof {
    /// Scale encoded `pallet_ismp::primitives::Proof` or state trie proof `Vec<Vec<u8>>`
    pub proof: Vec<u8>,
    /// Optional scale encoded `Vec<Leaf>` for mmr proof
    pub leaves: Option<Vec<u8>>,
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

    /// Query mmr proof for some requests
    #[method(name = "ismp_queryRequestsMmrProof")]
    fn query_requests_mmr_proof(&self, height: u32, query: Vec<LeafIndexQuery>) -> Result<Proof>;

    /// Query mmr proof for some responses
    #[method(name = "ismp_queryResponsesMmrProof")]
    fn query_responses_mmr_proof(&self, height: u32, query: Vec<LeafIndexQuery>) -> Result<Proof>;

    /// Query membership or non-membership proof for some keys
    #[method(name = "ismp_queryStateProof")]
    fn query_state_proof(&self, height: u32, keys: Vec<Vec<u8>>) -> Result<Proof>;

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

    /// Query the latest height for a state machine
    #[method(name = "ismp_queryStateMachineLatestHeight")]
    fn query_state_machine_latest_height(&self, id: StateMachineId) -> Result<u64>;

    /// Query ISMP Events that were deposited in a series of blocks
    /// Using String keys because HashMap fails to deserialize when key is not a String
    #[method(name = "ismp_queryEvents")]
    fn query_events(
        &self,
        block_numbers: Vec<BlockNumberOrHash<Hash>>,
    ) -> Result<HashMap<String, Vec<pallet_ismp::events::Event>>>;

    /// Query pending get requests that have a `state_machine_height` <=  `height`.
    #[method(name = "ismp_pendingGetRequests")]
    fn pending_get_requests(&self, height: u64) -> Result<Vec<Get>>;
}

/// An implementation of ISMP specific RPC methods.
pub struct IsmpRpcHandler<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> IsmpRpcHandler<C, B> {
    /// Create new `IsmpRpcHandler` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client, _marker: Default::default() }
    }
}

impl<C, Block> IsmpApiServer<Block::Hash> for IsmpRpcHandler<C, Block>
where
    Block: BlockT,
    C: Send
        + Sync
        + 'static
        + ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + ProofProvider<Block>
        + BlockBackend<Block>,
    C::Api: IsmpRuntimeApi<Block, Block::Hash>,
{
    fn query_requests(&self, query: Vec<LeafIndexQuery>) -> Result<Vec<Request>> {
        let api = self.client.runtime_api();
        let at = self.client.info().best_hash;
        let request_indices: Vec<LeafIndex> = api
            .get_request_leaf_indices(at, query)
            .map_err(|_| runtime_error_into_rpc_error("Error fetching request leaf indices"))?;

        api.get_requests(at, request_indices)
            .map_err(|_| runtime_error_into_rpc_error("Error fetching requests"))
    }

    fn query_responses(&self, query: Vec<LeafIndexQuery>) -> Result<Vec<Response>> {
        let api = self.client.runtime_api();
        let at = self.client.info().best_hash;
        let response_indices: Vec<LeafIndex> = api
            .get_response_leaf_indices(at, query)
            .map_err(|_| runtime_error_into_rpc_error("Error fetching response leaf indices"))?;

        api.get_responses(at, response_indices)
            .map_err(|_| runtime_error_into_rpc_error("Error fetching responses"))
    }

    fn query_requests_mmr_proof(&self, height: u32, query: Vec<LeafIndexQuery>) -> Result<Proof> {
        let api = self.client.runtime_api();
        let at = self
            .client
            .block_hash(height.into())
            .ok()
            .flatten()
            .ok_or_else(|| runtime_error_into_rpc_error("invalid block height provided"))?;
        let request_indices: Vec<LeafIndex> = api
            .get_request_leaf_indices(at, query)
            .map_err(|_| runtime_error_into_rpc_error("Error fetching response leaf indices"))?;

        let (leaves, proof): (Vec<Leaf>, pallet_ismp::primitives::Proof<Block::Hash>) = api
            .generate_proof(at, request_indices)
            .map_err(|_| runtime_error_into_rpc_error("Error calling runtime api"))?
            .map_err(|_| runtime_error_into_rpc_error("Error generating mmr proof"))?;
        Ok(Proof { proof: proof.encode(), leaves: Some(leaves.encode()), height })
    }

    fn query_responses_mmr_proof(&self, height: u32, query: Vec<LeafIndexQuery>) -> Result<Proof> {
        let api = self.client.runtime_api();
        let at = self
            .client
            .block_hash(height.into())
            .ok()
            .flatten()
            .ok_or_else(|| runtime_error_into_rpc_error("invalid block height provided"))?;
        let response_indices: Vec<LeafIndex> = api
            .get_response_leaf_indices(at, query)
            .map_err(|_| runtime_error_into_rpc_error("Error fetching response leaf indices"))?;

        let (leaves, proof): (Vec<Leaf>, pallet_ismp::primitives::Proof<Block::Hash>) = api
            .generate_proof(at, response_indices)
            .map_err(|_| runtime_error_into_rpc_error("Error calling runtime api"))?
            .map_err(|_| runtime_error_into_rpc_error("Error generating mmr proof"))?;
        Ok(Proof { proof: proof.encode(), leaves: Some(leaves.encode()), height })
    }

    fn query_state_proof(&self, height: u32, keys: Vec<Vec<u8>>) -> Result<Proof> {
        let at = self.client.block_hash(height.into()).ok().flatten().ok_or_else(|| {
            runtime_error_into_rpc_error("Could not find valid blockhash for provided height")
        })?;
        let proof: Vec<_> = self
            .client
            .read_proof(at, &mut keys.iter().map(|key| key.as_slice()))
            .map(|proof| proof.into_iter_nodes().collect())
            .map_err(|_| runtime_error_into_rpc_error("Error reading state proof"))?;
        Ok(Proof { proof: proof.encode(), leaves: None, height })
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

    fn query_state_machine_latest_height(&self, id: StateMachineId) -> Result<u64> {
        let api = self.client.runtime_api();
        let at = self.client.info().best_hash;
        api.latest_state_machine_height(at, id).ok().flatten().ok_or_else(|| {
            runtime_error_into_rpc_error("Error fetching latest state machine height")
        })
    }

    fn pending_get_requests(&self, height: u64) -> Result<Vec<Get>> {
        let api = self.client.runtime_api();
        let at = self.client.info().best_hash;

        api.pending_get_requests(at)
            .map(|reqs| reqs.into_iter().filter(|req| req.height <= height).collect())
            .map_err(|_| runtime_error_into_rpc_error("Error fetching get requests"))
    }

    fn query_events(
        &self,
        block_numbers: Vec<BlockNumberOrHash<Block::Hash>>,
    ) -> Result<HashMap<String, Vec<pallet_ismp::events::Event>>> {
        let api = self.client.runtime_api();
        let mut events = HashMap::new();
        for block_number_or_hash in block_numbers {
            let at = match block_number_or_hash {
                BlockNumberOrHash::Hash(block_hash) => block_hash,
                BlockNumberOrHash::Number(block_number) => {
                    self.client.block_hash(block_number.into()).ok().flatten().ok_or_else(|| {
                        runtime_error_into_rpc_error("Invalid block number provided")
                    })?
                }
            };

            let temp = api
                .block_events(at)
                .ok()
                .ok_or_else(|| runtime_error_into_rpc_error("failed to read block events"))?;
            events.insert(block_number_or_hash.to_string(), temp);
        }
        Ok(events)
    }
}
