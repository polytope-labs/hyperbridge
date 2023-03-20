use crate::error::Error;
use crate::host::ISMPHost;
use crate::prelude::Vec;
use codec::{Decode, Encode};
use core::time::Duration;

pub type ConsensusClientId = u64;
pub const ETHEREUM_CONSENSUS_CLIENT_ID: ConsensusClientId = 100;
pub const GNOSIS_CONSENSUS_CLIENT_ID: ConsensusClientId = 200;

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct StateCommitment {
    /// Timestamp in nanoseconds
    pub timestamp: u64,
    pub commitment_root: Vec<u8>,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct IntermediateState {
    pub height: StateMachineHeight,
    pub commitment: StateCommitment,
}

#[derive(Debug, Clone, Copy, Encode, Decode, scale_info::TypeInfo)]
pub struct StateMachineId {
    pub state_id: u64,
    pub consensus_client: ConsensusClientId,
}

#[derive(Debug, Clone, Copy, Encode, Decode, scale_info::TypeInfo)]
pub struct StateMachineHeight {
    pub id: StateMachineId,
    pub height: u64,
}

/// The consensus client handles logic for consensus proof verification
pub trait ConsensusClient {
    /// Should decode the scale encoded trusted consensus state and new consensus proof, verifying that:
    /// - the client isn't frozen yet
    /// - that the client hasn't elapsed it's unbonding period
    /// - check for byzantine behaviour
    /// - verify the consensus proofs
    /// - finally return the new consensusState and state commitments.
    /// - If byzantine behaviour is detected
    /// - Implementations can deposit an event after successful verification
    fn verify(
        &self,
        host: &dyn ISMPHost,
        trusted_consensus_state: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<IntermediateState>), Error>;

    /// Check if the client has expired since the last update
    fn is_expired(&self, host: &dyn ISMPHost) -> Result<bool, Error> {
        let host_timestamp = host.host_timestamp();
        let unbonding_period = self.unbonding_period();
        let last_update = host.consensus_update_time(self.consensus_id())?;
        Ok(host_timestamp.saturating_sub(last_update) > unbonding_period)
    }

    /// Return the configured ConsensusClientId for this client
    fn consensus_id(&self) -> ConsensusClientId;

    /// Return unbonding period
    fn unbonding_period(&self) -> Duration;

    /// Verify membership of proof of a commitment
    fn verify_membership(
        &self,
        host: &dyn ISMPHost,
        key: Vec<u8>,
        commitment: Vec<u8>,
    ) -> Result<(), Error>;

    /// Verify non-membership of proof of a commitment
    fn verify_non_membership(
        &self,
        host: &dyn ISMPHost,
        key: Vec<u8>,
        commitment: Vec<u8>,
    ) -> Result<(), Error>;

    /// Check if consensus client is frozen
    fn is_frozen(&self, host: &dyn ISMPHost, id: ConsensusClientId) -> Result<bool, Error>;
}
