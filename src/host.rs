use crate::router::Router;
use crate::{
    Config, ConsensusClientUpdateTime, ConsensusStates, FrozenHeights, LatestStateMachineHeight,
    RequestAcks, StateCommitments,
};
use alloc::format;
use alloc::string::ToString;
use core::time::Duration;
use frame_support::traits::UnixTime;
use ismp_rust::consensus_client::{
    ConsensusClient, ConsensusClientId, StateCommitment, StateMachineHeight, StateMachineId,
    ETHEREUM_CONSENSUS_CLIENT_ID,
};
use ismp_rust::error::Error;
use ismp_rust::host::{ChainID, ISMPHost};
use ismp_rust::paths::RequestPath;
use ismp_rust::router::{IISMPRouter, Request};
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

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
        LatestStateMachineHeight::<T>::get(id)
            .map(|height| StateMachineHeight { id, height })
            .ok_or_else(|| {
                Error::ImplementationSpecific("Missing latest state machine height".to_string())
            })
    }

    fn state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<StateCommitment, Error> {
        StateCommitments::<T>::get(height).ok_or_else(|| Error::StateCommitmentNotFound { height })
    }

    fn consensus_update_time(&self, id: ConsensusClientId) -> Result<Duration, Error> {
        ConsensusClientUpdateTime::<T>::get(id)
            .map(|timestamp| Duration::from_nanos(timestamp))
            .ok_or_else(|| {
                Error::ImplementationSpecific(format!("Update time not found for {:?}", id))
            })
    }

    fn consensus_state(&self, id: ConsensusClientId) -> Result<Vec<u8>, Error> {
        ConsensusStates::<T>::get(id).ok_or_else(|| Error::ConsensusStateNotFound { id })
    }

    fn host_timestamp(&self) -> Duration {
        <T::TimeProvider as UnixTime>::now()
    }

    fn is_frozen(&self, height: StateMachineHeight) -> Result<bool, Error> {
        if let Some(frozen_height) = FrozenHeights::<T>::get(height.id) {
            Ok(height.height >= frozen_height)
        } else {
            Ok(false)
        }
    }

    fn request_commitment(&self, req: &Request) -> Result<Vec<u8>, Error> {
        let key = RequestPath {
            dest_chain: req.dest_chain,
            source_chain: req.source_chain,
            nonce: req.nonce,
        }
        .to_string()
        .as_bytes()
        .to_vec();
        RequestAcks::<T>::get(key).ok_or_else(|| Error::RequestCommitmentNotFound {
            nonce: req.nonce,
            source: req.source_chain,
            dest: req.dest_chain,
        })
    }

    fn store_consensus_state(&self, id: ConsensusClientId, state: Vec<u8>) -> Result<(), Error> {
        ConsensusStates::<T>::insert(id, state);
        Ok(())
    }

    fn store_consensus_update_time(
        &self,
        id: ConsensusClientId,
        timestamp: Duration,
    ) -> Result<(), Error> {
        ConsensusClientUpdateTime::<T>::insert(id, timestamp.as_nanos().saturated_into::<u64>());
        Ok(())
    }

    fn store_state_machine_commitment(
        &self,
        height: StateMachineHeight,
        state: StateCommitment,
    ) -> Result<(), Error> {
        StateCommitments::<T>::insert(height, state);
        Ok(())
    }

    fn freeze_state_machine(&self, height: StateMachineHeight) -> Result<(), Error> {
        FrozenHeights::<T>::insert(height.id, height.height);
        Ok(())
    }

    fn consensus_client(&self, _id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
        todo!()
    }

    fn keccak256(&self, bytes: &[u8]) -> [u8; 32] {
        sp_io::hashing::keccak_256(bytes)
    }

    fn delay_period(&self, id: ConsensusClientId) -> Duration {
        match id {
            id if id == ETHEREUM_CONSENSUS_CLIENT_ID => Duration::from_secs(30 * 60),
            _ => Duration::from_secs(15 * 60),
        }
    }

    fn ismp_router(&self) -> Box<dyn IISMPRouter> {
        Box::new(Router::<T>::default())
    }
}
