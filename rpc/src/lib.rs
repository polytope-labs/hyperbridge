#![warn(missing_docs)]

//! ISMP RPC Implementation.

use jsonrpsee::{
    core::{Error as RpcError, RpcResult as Result},
    proc_macros::rpc,
    types::{error::CallError, ErrorObject},
};

use codec::Encode;
use ismp_rs::{
    consensus_client::ConsensusClientId,
    router::{Request, Response},
};
use ismp_runtime_api::{IsmpRuntimeApi, LeafIndexQuery};
use pallet_ismp::mmr::{Leaf, LeafIndex};
use sc_client_api::{BlockBackend, ProofProvider};
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
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

    /// Query ISMP Events that were deposited in a series of blocks
    /// Using String keys because HashMap fails to deserialize when key is not a String
    #[method(name = "ibc_queryEvents")]
    fn query_events(
        &self,
        block_numbers: Vec<BlockNumberOrHash<Hash>>,
    ) -> Result<HashMap<String, Vec<pallet_ismp::events::Event>>>;
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
        let at = BlockId::Hash(self.client.info().best_hash);
        let request_indices: Vec<LeafIndex> =
            api.get_request_leaf_indices(&at, query).ok().flatten().ok_or_else(|| {
                runtime_error_into_rpc_error("Error fetching request leaf indices")
            })?;

        api.get_requests(&at, request_indices)
            .ok()
            .flatten()
            .ok_or_else(|| runtime_error_into_rpc_error("Error fetching requests"))
    }

    fn query_responses(&self, query: Vec<LeafIndexQuery>) -> Result<Vec<Response>> {
        let api = self.client.runtime_api();
        let at = BlockId::Hash(self.client.info().best_hash);
        let response_indices: Vec<LeafIndex> =
            api.get_response_leaf_indices(&at, query).ok().flatten().ok_or_else(|| {
                runtime_error_into_rpc_error("Error fetching response leaf indices")
            })?;

        api.get_responses(&at, response_indices)
            .ok()
            .flatten()
            .ok_or_else(|| runtime_error_into_rpc_error("Error fetching responses"))
    }

    fn query_requests_mmr_proof(&self, height: u32, query: Vec<LeafIndexQuery>) -> Result<Proof> {
        let api = self.client.runtime_api();
        let at = BlockId::Number(height.into());
        let request_indices: Vec<LeafIndex> =
            api.get_request_leaf_indices(&at, query).ok().flatten().ok_or_else(|| {
                runtime_error_into_rpc_error("Error fetching response leaf indices")
            })?;

        let (leaves, proof): (Vec<Leaf>, pallet_ismp::primitives::Proof<Block::Hash>) = api
            .generate_proof(&at, request_indices)
            .map_err(|_| runtime_error_into_rpc_error("Error calling runtime api"))?
            .map_err(|_| runtime_error_into_rpc_error("Error generating mmr proof"))?;
        Ok(Proof { proof: proof.encode(), leaves: Some(leaves.encode()), height })
    }

    fn query_responses_mmr_proof(&self, height: u32, query: Vec<LeafIndexQuery>) -> Result<Proof> {
        let api = self.client.runtime_api();
        let at = BlockId::Number(height.into());
        let response_indices: Vec<LeafIndex> =
            api.get_response_leaf_indices(&at, query).ok().flatten().ok_or_else(|| {
                runtime_error_into_rpc_error("Error fetching response leaf indices")
            })?;

        let (leaves, proof): (Vec<Leaf>, pallet_ismp::primitives::Proof<Block::Hash>) = api
            .generate_proof(&at, response_indices)
            .map_err(|_| runtime_error_into_rpc_error("Error calling runtime api"))?
            .map_err(|_| runtime_error_into_rpc_error("Error generating mmr proof"))?;
        Ok(Proof { proof: proof.encode(), leaves: Some(leaves.encode()), height })
    }

    fn query_state_proof(&self, _height: u32, _keys: Vec<Vec<u8>>) -> Result<Proof> {
        unimplemented!()
    }

    fn query_consensus_state(
        &self,
        height: Option<u32>,
        client_id: ConsensusClientId,
    ) -> Result<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = height
            .map(|height| BlockId::Number(height.into()))
            .unwrap_or(BlockId::Hash(self.client.info().best_hash));
        api.consensus_state(&at, client_id)
            .ok()
            .flatten()
            .ok_or_else(|| runtime_error_into_rpc_error("Error fetching Consensus state"))
    }

    fn query_consensus_update_time(&self, client_id: ConsensusClientId) -> Result<u64> {
        let api = self.client.runtime_api();
        let at = BlockId::Hash(self.client.info().best_hash);
        api.consensus_update_time(&at, client_id)
            .ok()
            .flatten()
            .ok_or_else(|| runtime_error_into_rpc_error("Error fetching Consensus state"))
    }

    fn query_events(
        &self,
        block_numbers: Vec<BlockNumberOrHash<Block::Hash>>,
    ) -> Result<HashMap<String, Vec<pallet_ismp::events::Event>>> {
        let api = self.client.runtime_api();
        let mut events = HashMap::new();
        for block_number_or_hash in block_numbers {
            let at = match block_number_or_hash {
                BlockNumberOrHash::Hash(block_hash) => BlockId::Hash(block_hash),
                BlockNumberOrHash::Number(block_number) => BlockId::Number(block_number.into()),
            };

            let temp = api
                .block_events(&at)
                .ok()
                .flatten()
                .ok_or_else(|| runtime_error_into_rpc_error("failed to read block events"))?;
            events.insert(block_number_or_hash.to_string(), temp);
        }
        Ok(events)
    }
}
