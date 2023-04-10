use codec::{Decode, Encode};
use ismp_rs::{
    consensus_client::{ConsensusClientId, StateMachineHeight},
    error::Error as IsmpError,
    host::ChainID,
};
use sp_std::prelude::*;

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub enum HandlingError {
    ChallengePeriodNotElapsed {
        update_time: u64,
        current_time: u64,
        delay_period: Option<u64>,
        consensus_client_id: Option<ConsensusClientId>,
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
    ImplementationSpecific {
        msg: Vec<u8>,
    },
    UnbondingPeriodElapsed {
        consensus_id: ConsensusClientId,
    },
    MembershipProofVerificationFailed {
        msg: Vec<u8>,
    },
    NonMembershipProofVerificationFailed {
        msg: Vec<u8>,
    },
}

impl From<ismp_rs::error::Error> for HandlingError {
    fn from(value: ismp_rs::error::Error) -> Self {
        match value {
            IsmpError::ChallengePeriodNotElapsed { consensus_id, current_time, update_time } => {
                HandlingError::ChallengePeriodNotElapsed {
                    update_time: update_time.as_secs(),
                    current_time: current_time.as_secs(),
                    delay_period: None,
                    consensus_client_id: Some(consensus_id),
                }
            }
            IsmpError::ConsensusStateNotFound { id } => {
                HandlingError::ConsensusStateNotFound { id }
            }
            IsmpError::StateCommitmentNotFound { height } => {
                HandlingError::StateCommitmentNotFound { height }
            }
            IsmpError::FrozenConsensusClient { id } => HandlingError::FrozenConsensusClient { id },
            IsmpError::FrozenStateMachine { height } => {
                HandlingError::FrozenStateMachine { height }
            }
            IsmpError::RequestCommitmentNotFound { nonce, source, dest } => {
                HandlingError::RequestCommitmentNotFound { nonce, source, dest }
            }
            IsmpError::RequestVerificationFailed { nonce, source, dest } => {
                HandlingError::ResponseVerificationFailed { nonce, source, dest }
            }
            IsmpError::ResponseVerificationFailed { nonce, source, dest } => {
                HandlingError::ResponseVerificationFailed { nonce, source, dest }
            }
            IsmpError::ConsensusProofVerificationFailed { id } => {
                HandlingError::ConsensusProofVerificationFailed { id }
            }
            IsmpError::ExpiredConsensusClient { id } => {
                HandlingError::ExpiredConsensusClient { id }
            }
            IsmpError::CannotHandleConsensusMessage => HandlingError::CannotHandleConsensusMessage,
            IsmpError::ImplementationSpecific(msg) => {
                HandlingError::ImplementationSpecific { msg: msg.as_bytes().to_vec() }
            }
            IsmpError::UnbondingPeriodElapsed { consensus_id } => {
                HandlingError::UnbondingPeriodElapsed { consensus_id }
            }
            IsmpError::MembershipProofVerificationFailed(msg) => {
                HandlingError::MembershipProofVerificationFailed { msg: msg.as_bytes().to_vec() }
            }
            IsmpError::NonMembershipProofVerificationFailed(msg) => {
                HandlingError::NonMembershipProofVerificationFailed { msg: msg.as_bytes().to_vec() }
            }
        }
    }
}
