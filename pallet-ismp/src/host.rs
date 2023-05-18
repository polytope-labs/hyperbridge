use crate::{
    primitives::ConsensusClientProvider, router::Receipt, Config, ConsensusClientUpdateTime,
    ConsensusStates, FrozenHeights, LatestStateMachineHeight, RequestAcks, StateCommitments,
};
use alloc::{format, string::ToString};
use core::time::Duration;
use frame_support::traits::{Get, UnixTime};
use ismp_rs::{
    consensus::{
        ConsensusClient, ConsensusClientId, StateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error,
    host::{ISMPHost, StateMachine},
    router::{ISMPRouter, Request},
    util::hash_request,
};
use sp_core::H256;
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

#[derive(Clone)]
pub struct Host<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> Default for Host<T> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<T: Config> ISMPHost for Host<T>
where
    <T as frame_system::Config>::Hash: From<H256>,
{
    fn host_state_machine(&self) -> StateMachine {
        T::StateMachine::get()
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
            .map(|timestamp| Duration::from_secs(timestamp))
            .ok_or_else(|| {
                Error::ImplementationSpecific(format!("Update time not found for {:?}", id))
            })
    }

    fn consensus_state(&self, id: ConsensusClientId) -> Result<Vec<u8>, Error> {
        ConsensusStates::<T>::get(id).ok_or_else(|| Error::ConsensusStateNotFound { id })
    }

    fn timestamp(&self) -> Duration {
        <T::TimeProvider as UnixTime>::now()
    }

    fn is_frozen(&self, height: StateMachineHeight) -> Result<bool, Error> {
        if let Some(frozen_height) = FrozenHeights::<T>::get(height.id) {
            Ok(height.height >= frozen_height)
        } else {
            Ok(false)
        }
    }

    fn request_commitment(&self, req: &Request) -> Result<H256, Error> {
        let commitment = hash_request::<Self>(req);

        let _ = RequestAcks::<T>::get(commitment.0.to_vec()).ok_or_else(|| {
            Error::RequestCommitmentNotFound {
                nonce: req.nonce(),
                source: req.source_chain(),
                dest: req.dest_chain(),
            }
        })?;

        Ok(commitment)
    }

    fn get_request_receipt(&self, req: &Request) -> Option<()> {
        let commitment = hash_request::<Self>(req);

        let _ = RequestAcks::<T>::get(commitment.0.to_vec())
            .ok_or_else(|| Error::RequestCommitmentNotFound {
                nonce: req.nonce(),
                source: req.source_chain(),
                dest: req.dest_chain(),
            })
            .ok()?;

        Some(())
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
        ConsensusClientUpdateTime::<T>::insert(id, timestamp.as_secs().saturated_into::<u64>());
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

    fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error> {
        LatestStateMachineHeight::<T>::insert(height.id, height.height);
        Ok(())
    }

    fn delete_request_commitment(&self, req: &Request) -> Result<(), Error> {
        let hash = hash_request::<Self>(req);
        // We can't delete actual leaves in the mmr so this serves as a replacement for that
        RequestAcks::<T>::remove(hash.0.to_vec());
        Ok(())
    }

    fn store_request_receipt(&self, req: &Request) -> Result<(), Error> {
        let hash = hash_request::<Self>(req);
        RequestAcks::<T>::insert(hash.0.to_vec(), Receipt::Ok);
        Ok(())
    }

    fn consensus_client(&self, id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
        <T as Config>::ConsensusClientProvider::consensus_client(id)
    }

    fn keccak256(bytes: &[u8]) -> H256
    where
        Self: Sized,
    {
        sp_io::hashing::keccak_256(bytes).into()
    }

    fn challenge_period(&self, id: ConsensusClientId) -> Duration {
        <T as Config>::ConsensusClientProvider::challenge_period(id)
    }

    fn ismp_router(&self) -> Box<dyn ISMPRouter> {
        Box::new(T::IsmpRouter::default())
    }
}
