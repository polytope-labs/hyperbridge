use ethabi::ethereum_types::H256;
use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
        StateMachineId,
    },
    error::Error,
    host::StateMachine,
    router::{IsmpRouter, Request},
};
use std::time::Duration;

pub struct Host;

impl ismp::host::IsmpHost for Host {
    fn host_state_machine(&self) -> StateMachine {
        todo!()
    }

    fn latest_commitment_height(&self, _id: StateMachineId) -> Result<u64, Error> {
        todo!()
    }

    fn state_machine_commitment(
        &self,
        _height: StateMachineHeight,
    ) -> Result<StateCommitment, Error> {
        todo!()
    }

    fn consensus_update_time(
        &self,
        _consensus_state_id: ConsensusStateId,
    ) -> Result<Duration, Error> {
        todo!()
    }

    fn state_machine_update_time(
        &self,
        _state_machine_height: StateMachineHeight,
    ) -> Result<Duration, Error> {
        todo!()
    }

    fn consensus_client_id(
        &self,
        _consensus_state_id: ConsensusStateId,
    ) -> Option<ConsensusClientId> {
        todo!()
    }

    fn consensus_state(&self, _consensus_state_id: ConsensusStateId) -> Result<Vec<u8>, Error> {
        todo!()
    }

    fn timestamp(&self) -> Duration {
        todo!()
    }

    fn is_state_machine_frozen(&self, _machine: StateMachineHeight) -> Result<(), Error> {
        todo!()
    }

    fn is_consensus_client_frozen(
        &self,
        _consensus_state_id: ConsensusStateId,
    ) -> Result<(), Error> {
        todo!()
    }

    fn request_commitment(&self, _req: H256) -> Result<(), Error> {
        todo!()
    }

    fn next_nonce(&self) -> u64 {
        todo!()
    }

    fn request_receipt(&self, _req: &Request) -> Option<()> {
        todo!()
    }

    fn response_receipt(&self, _res: &Request) -> Option<()> {
        todo!()
    }

    fn store_consensus_state_id(
        &self,
        _consensus_state_id: ConsensusStateId,
        _client_id: ConsensusClientId,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_consensus_state(
        &self,
        _consensus_state_id: ConsensusStateId,
        _consensus_state: Vec<u8>,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_unbonding_period(
        &self,
        _consensus_state_id: ConsensusStateId,
        _period: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_consensus_update_time(
        &self,
        _consensus_state_id: ConsensusStateId,
        _timestamp: Duration,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_state_machine_update_time(
        &self,
        _state_machine_height: StateMachineHeight,
        _timestamp: Duration,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_state_machine_commitment(
        &self,
        _height: StateMachineHeight,
        _state: StateCommitment,
    ) -> Result<(), Error> {
        todo!()
    }

    fn freeze_state_machine(&self, _height: StateMachineHeight) -> Result<(), Error> {
        todo!()
    }

    fn freeze_consensus_client(&self, _consensus_state_id: ConsensusStateId) -> Result<(), Error> {
        todo!()
    }

    fn store_latest_commitment_height(&self, _height: StateMachineHeight) -> Result<(), Error> {
        todo!()
    }

    fn delete_request_commitment(&self, _req: &Request) -> Result<(), Error> {
        todo!()
    }

    fn store_request_receipt(&self, _req: &Request) -> Result<(), Error> {
        todo!()
    }

    fn store_response_receipt(&self, _req: &Request) -> Result<(), Error> {
        todo!()
    }

    fn consensus_client(&self, _id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
        todo!()
    }

    fn challenge_period(&self, _consensus_state_id: ConsensusStateId) -> Option<Duration> {
        todo!()
    }

    fn store_challenge_period(
        &self,
        _consensus_state_id: ConsensusStateId,
        _period: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    fn allowed_proxies(&self) -> Vec<StateMachine> {
        todo!()
    }

    fn store_allowed_proxies(&self, _allowed: Vec<StateMachine>) {
        todo!()
    }

    fn unbonding_period(&self, _consensus_state_id: ConsensusStateId) -> Option<Duration> {
        todo!()
    }

    fn ismp_router(&self) -> Box<dyn IsmpRouter> {
        todo!()
    }
}

impl ismp::util::Keccak256 for Host {
    fn keccak256(bytes: &[u8]) -> H256
    where
        Self: Sized,
    {
        sp_core::keccak_256(bytes).into()
    }
}
