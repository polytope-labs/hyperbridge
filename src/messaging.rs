use crate::consensus_client::{ConsensusClientId, StateMachineHeight};
use crate::router::{Request, Response};
use alloc::vec::Vec;
use codec::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode)]
pub struct ConsensusMessage {
    /// Scale Encoded Consensus Proof
    consensus_proof: Vec<u8>,
    /// Consensus client id
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
