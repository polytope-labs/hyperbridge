use crate::consensus_client::{ConsensusClientId, StateMachineHeight};
use crate::host::ChainID;
use alloc::string::String;
use core::time::Duration;

#[derive(Debug)]
pub enum Error {
    DelayNotElapsed {
        update_time: Duration,
        current_time: Duration,
    },
    ConsensusStateNotFound {
        id: ConsensusClientId,
    },
    StateCommitmentNotFound {
        height: StateMachineHeight,
    },
    FrozenConsensusClient {
        id: ConsensusClientId,
    },
    FrozenStateMachine {
        height: StateMachineHeight,
    },
    RequestCommitmentNotFound {
        nonce: u64,
        source: ChainID,
        dest: ChainID,
    },
    RequestVerificationFailed {
        nonce: u64,
        source: ChainID,
        dest: ChainID,
    },
    ResponseVerificationFailed {
        nonce: u64,
        source: ChainID,
        dest: ChainID,
    },
    ConsensusProofVerificationFailed {
        id: ConsensusClientId,
    },
    ExpiredConsensusClient {
        id: ConsensusClientId,
    },
    CannotHandleConsensusMessage,
    ImplementationSpecific(String),
}
