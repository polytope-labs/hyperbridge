use crate::Config;
use ismp_rust::consensus_client::{
    ConsensusClient, ConsensusClientId, StateCommitment, StateMachineHeight, StateMachineId,
};
use ismp_rust::error::Error;
use ismp_rust::host::{ChainID, ISMPHost};
use ismp_rust::router::{IISMPRouter, Request, Response};
use std::time::Duration;

#[derive(Clone)]
pub struct Host<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> Default for Host<T> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<T: Config> ISMPHost for Host<T> {
    fn host(&self) -> ChainID {
        <T as Config>::CHAIN_ID
    }

    fn latest_commitment_height(&self, id: StateMachineId) -> Result<StateMachineHeight, Error> {
        todo!()
    }

    fn state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<StateCommitment, Error> {
        todo!()
    }

    fn consensus_update_time(&self, id: ConsensusClientId) -> Result<Duration, Error> {
        todo!()
    }

    fn state_machine_update_time(&self, height: StateMachineHeight) -> Result<Duration, Error> {
        todo!()
    }

    fn consensus_state(&self, id: ConsensusClientId) -> Result<Vec<u8>, Error> {
        todo!()
    }

    fn host_timestamp(&self) -> Duration {
        todo!()
    }

    fn is_frozen(&self, height: StateMachineHeight) -> Result<bool, Error> {
        todo!()
    }

    fn request_commitment(&self, req: &Request) -> Result<Vec<u8>, Error> {
        todo!()
    }

    fn response_commitment(&self, res: &Response) -> Result<Vec<u8>, Error> {
        todo!()
    }

    fn store_consensus_state(&self, id: ConsensusClientId, state: Vec<u8>) -> Result<(), Error> {
        todo!()
    }

    fn store_consensus_update_time(
        &self,
        id: ConsensusClientId,
        timestamp: Duration,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_state_machine_update_time(
        &self,
        height: StateMachineHeight,
        timestamp: Duration,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_state_machine_commitment(
        &self,
        height: StateMachineHeight,
        state: StateCommitment,
    ) -> Result<(), Error> {
        todo!()
    }

    fn freeze_state_machine(&self, height: StateMachineHeight) -> Result<(), Error> {
        todo!()
    }

    fn consensus_client(&self, id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
        todo!()
    }

    fn keccak256(&self, bytes: &[u8]) -> [u8; 32] {
        todo!()
    }

    fn delay_period(&self, id: StateMachineId) -> Duration {
        todo!()
    }

    fn client_id_from_state_id(&self, id: StateMachineId) -> Result<ConsensusClientId, Error> {
        todo!()
    }

    fn ismp_router(&self) -> Box<dyn IISMPRouter> {
        todo!()
    }
}
