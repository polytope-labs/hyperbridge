use core::time::Duration;
use frame_support::RuntimeDebug;
use ismp_primitives::mmr::{LeafIndex, NodeIndex};
use ismp_rs::{
    consensus::{ConsensusClient, ConsensusClientId, StateMachineHeight},
    host::StateMachine,
    router::Post,
};
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// The `ConsensusEngineId` of ISMP.
pub const ISMP_ID: sp_runtime::ConsensusEngineId = *b"ISMP";

/// An MMR proof data for a group of leaves.
#[derive(codec::Encode, codec::Decode, RuntimeDebug, Clone, PartialEq, Eq, TypeInfo)]
pub struct Proof<Hash> {
    /// The indices of the leaves the proof is for.
    pub leaf_indices: Vec<LeafIndex>,
    /// Number of leaves in MMR, when the proof was generated.
    pub leaf_count: NodeIndex,
    /// Proof elements (hashes of siblings of inner nodes on the path to the leaf).
    pub items: Vec<Hash>,
}

/// Merkle Mountain Range operation error.
#[derive(RuntimeDebug, codec::Encode, codec::Decode, PartialEq, Eq)]
pub enum Error {
    InvalidNumericOp,
    Push,
    GetRoot,
    Commit,
    GenerateProof,
    Verify,
    LeafNotFound,
    PalletNotIncluded,
    InvalidLeafIndex,
    InvalidBestKnownBlock,
}

pub trait ConsensusClientProvider {
    fn consensus_client(
        id: ConsensusClientId,
    ) -> Result<Box<dyn ConsensusClient>, ismp_rs::error::Error>;

    fn challenge_period(id: ConsensusClientId) -> Duration;
}

pub enum IsmpMessage {
    Post {
        /// The destination state machine of this request.
        dest_chain: StateMachine,
        /// Module Id of the sending module
        from: Vec<u8>,
        /// Module ID of the receiving module
        to: Vec<u8>,
        /// Timestamp which this request expires in seconds.
        timeout_timestamp: u64,
        /// Encoded Request.
        data: Vec<u8>,
    },
    Get {
        /// The destination state machine of this request.
        dest_chain: StateMachine,
        /// Module Id of the sending module
        from: Vec<u8>,
        /// Raw Storage keys that this request is interested in.
        keys: Vec<Vec<u8>>,
        /// Height at which to read the state machine.
        height: StateMachineHeight,
        /// Host Timestamp which this request expires in seconds
        timeout_timestamp: u64,
    },
    Response {
        post: Post,
        response: Vec<u8>,
    },
}

pub trait IsmpDispatch {
    fn dispatch_message(msg: IsmpMessage) -> Result<(), ismp_rs::router::DispatchError>;
}
