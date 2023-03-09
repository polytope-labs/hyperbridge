use crate::consensus_client::{ConsensusClientId, StateMachineHeight};
use crate::router::{Request, Response};
use alloc::vec::Vec;
use codec::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode)]
pub struct ConsensusMessage {
    /// Scale Encoded Consensus Proof
    pub consensus_proof: Vec<u8>,
    /// Consensus client id
    pub consensus_client_id: ConsensusClientId,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct RequestMessage {
    pub request: Request,
    pub proof: Proof,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ResponseMessage {
    pub response: Response,
    pub proof: Proof,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Proof {
    pub height: StateMachineHeight,
    pub proof: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum Message {
    #[codec(index = 0)]
    Consensus(ConsensusMessage),
    #[codec(index = 1)]
    Request(RequestMessage),
    #[codec(index = 2)]
    Response(ResponseMessage),
}
