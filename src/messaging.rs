use crate::consensus_client::{ConsensusClientId, StateMachineHeight};
use crate::router::{Request, Response};
use alloc::vec::Vec;
use codec::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct ConsensusMessage {
    /// Scale Encoded Consensus Proof
    pub consensus_proof: Vec<u8>,
    /// Consensus client id
    pub consensus_client_id: ConsensusClientId,
}
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct CreateConsensusClient {
    /// Scale encoded consensus state
    pub consensus_state: Vec<u8>,
    /// Consensus client id
    pub consensus_client_id: ConsensusClientId,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct RequestMessage {
    /// Request from source chain
    pub request: Request,
    /// Membership proof for this request
    pub proof: Proof,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct ResponseMessage {
    /// Response from sink chain
    pub response: Response,
    /// Membership proof for this response
    pub proof: Proof,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct Proof {
    /// State machine height
    pub height: StateMachineHeight,
    /// Raw proof
    pub proof: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo)]
pub enum Message {
    #[codec(index = 0)]
    CreateConsensusClient(CreateConsensusClient),
    #[codec(index = 1)]
    Consensus(ConsensusMessage),
    #[codec(index = 2)]
    Request(RequestMessage),
    #[codec(index = 3)]
    Response(ResponseMessage),
}
