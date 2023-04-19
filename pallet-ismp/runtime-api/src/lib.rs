#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]

use ismp_rs::{
    consensus_client::ConsensusClientId,
    host::StateMachine,
    router::{Request, Response},
};
use pallet_ismp::primitives::{Error, Proof};

use ismp_primitives::mmr::{Leaf, LeafIndex};
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

#[derive(codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct LeafIndexQuery {
    pub source_chain: StateMachine,
    pub dest_chain: StateMachine,
    pub nonce: u64,
}

sp_api::decl_runtime_apis! {
    /// ISMP Runtime Apis
    pub trait IsmpRuntimeApi<Hash: codec::Codec> {
        /// Return the number of MMR leaves.
        fn mmr_leaf_count() -> Result<LeafIndex, Error>;

        /// Return the on-chain MMR root hash.
        fn mmr_root() -> Result<Hash, Error>;

        /// Generate a proof for the provided leaf indices
        fn generate_proof(
            leaf_indices: Vec<LeafIndex>
        ) -> Result<(Vec<Leaf>, Proof<Hash>), Error>;

        /// Fetch all ISMP events
        fn block_events() -> Option<Vec<pallet_ismp::events::Event>>;

        /// Return the scale encoded consensus state
        fn consensus_state(id: ConsensusClientId) -> Option<Vec<u8>>;

        /// Return the timestamp this client was last updated in seconds
        fn consensus_update_time(id: ConsensusClientId) -> Option<u64>;

        /// Get Request Leaf Indices
        fn get_request_leaf_indices(leaf_queries: Vec<LeafIndexQuery>) -> Option<Vec<LeafIndex>>;

        /// Get Response Leaf Indices
        fn get_response_leaf_indices(leaf_queries: Vec<LeafIndexQuery>) -> Option<Vec<LeafIndex>>;

        /// Get actual requests
        fn get_requests(leaf_indices: Vec<LeafIndex>) -> Option<Vec<Request>>;

        /// Get actual requests
        fn get_responses(leaf_indices: Vec<LeafIndex>) -> Option<Vec<Response>>;
    }
}
