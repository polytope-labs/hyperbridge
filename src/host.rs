use crate::{
    consensus_client::{
        ConsensusClient, ConsensusClientId, StateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error,
    prelude::Vec,
    router::{IISMPRouter, Request, Response},
};
use alloc::{boxed::Box, string::ToString};
use codec::{Decode, Encode};
use core::time::Duration;
use derive_more::Display;

pub trait ISMPHost {
    fn host(&self) -> ChainID;

    // Storage Read functions

    /// Returns the latest height of the state machine
    fn latest_commitment_height(&self, id: StateMachineId) -> Result<StateMachineHeight, Error>;
    /// Returns the state machine at the give height
    fn state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<StateCommitment, Error>;
    /// Returns the host timestamp when this consensus client was last updated
    fn consensus_update_time(&self, id: ConsensusClientId) -> Result<Duration, Error>;
    /// Returns the scale encoded consensus state for a consensus client
    fn consensus_state(&self, id: ConsensusClientId) -> Result<Vec<u8>, Error>;
    /// Return the host timestamp in nanoseconds
    fn host_timestamp(&self) -> Duration;
    /// Checks if a state machine is frozen at the provided height
    fn is_frozen(&self, height: StateMachineHeight) -> Result<bool, Error>;
    /// Fetch commitment of a request from storage
    fn request_commitment(&self, req: &Request) -> Result<Vec<u8>, Error>;

    // Storage Write functions

    /// Store a scale encoded consensus state
    fn store_consensus_state(&self, id: ConsensusClientId, state: Vec<u8>) -> Result<(), Error>;
    /// Store the timestamp when the consensus client was updated
    fn store_consensus_update_time(
        &self,
        id: ConsensusClientId,
        timestamp: Duration,
    ) -> Result<(), Error>;
    /// Store the timestamp when the state machine was updated
    fn store_state_machine_commitment(
        &self,
        height: StateMachineHeight,
        state: StateCommitment,
    ) -> Result<(), Error>;
    /// Freeze a state machine at the given height
    fn freeze_state_machine(&self, height: StateMachineHeight) -> Result<(), Error>;
    /// Store latest height for a state machine
    fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error>;

    /// Return the keccak256 hash of a request
    /// Commitment is the hash of the concatenation of the data below
    /// request.dest_chain.encode() + request.timeout_timestamp.encode() + request.nonce.encode() +
    /// request.data
    fn get_request_commitment(&self, req: &Request) -> Vec<u8> {
        let mut buf = Vec::new();
        let dest_chain = req.dest_chain.to_string().as_bytes().to_vec();
        let timeout_timestamp = req.timeout_timestamp.encode();
        let nonce = req.nonce.encode();
        buf.extend_from_slice(&dest_chain[..]);
        buf.extend_from_slice(&timeout_timestamp[..]);
        buf.extend_from_slice(&nonce[..]);
        buf.extend_from_slice(&req.data[..]);
        self.keccak256(&buf[..]).to_vec()
    }

    /// Return the keccak256 of a response
    fn get_response_commitment(&self, res: &Response) -> Vec<u8> {
        let mut buf = Vec::new();
        let nonce = res.request.nonce.encode();
        buf.extend_from_slice(&nonce[..]);
        buf.extend_from_slice(&res.response[..]);
        self.keccak256(&buf[..]).to_vec()
    }

    /// Should return a handle to the consensus client based on the id
    fn consensus_client(&self, id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error>;

    // Hashing
    /// Returns a keccak256 hash of a byte slice
    fn keccak256(&self, bytes: &[u8]) -> [u8; 32];

    /// Returns the configured delay period for a consensus client
    fn delay_period(&self, id: ConsensusClientId) -> Duration;

    /// Return a handle to the router
    fn ismp_router(&self) -> Box<dyn IISMPRouter>;
}

#[derive(Clone, Debug, Copy, Encode, Decode, Display, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum ChainID {
    #[codec(index = 0)]
    ETHEREUM,
    #[codec(index = 1)]
    GNOSIS,
    #[codec(index = 2)]
    ARBITRUM,
    #[codec(index = 3)]
    OPTIMISM,
    #[codec(index = 4)]
    BASE,
    #[codec(index = 5)]
    MOONBEAM,
    #[codec(index = 6)]
    ASTAR,
    #[codec(index = 7)]
    HYPERSPACE,
}
