use crate::consensus_client::{ConsensusClientId, StateMachineHeight};
use crate::router::{Request, Response};
use alloc::vec::Vec;
use codec::{Decode, Encode};

/// Generic message
#[derive(Debug, Clone, Encode, Decode)]
pub struct Message {
    /// Message identifier
    pub identifier: MessageIdentifier,
    /// Scale encoded message
    pub message: Vec<u8>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum MessageIdentifier {
    #[codec(index = 0)]
    Consensus,
    #[codec(index = 1)]
    Request,
    #[codec(index = 2)]
    Response,
    #[codec(index = 3)]
    Timeout,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ConsensusMessage {
    consensus_proof: Vec<u8>,
    consensus_client_id: ConsensusClientId,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct RequestMessage {
    request: Request,
    proof: Proof,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ResponseMessage {
    response: Response,
    proof: Proof,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Proof {
    height: StateMachineHeight,
    proof: Vec<Vec<u8>>,
}
