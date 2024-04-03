// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Host implementation for ISMP
use crate::{
    child_trie::{RequestCommitments, RequestReceipts, ResponseCommitments, ResponseReceipts},
    primitives::ConsensusClientProvider,
    ChallengePeriod, Config, ConsensusClientUpdateTime, ConsensusStateClient, ConsensusStates,
    FrozenConsensusClients, FrozenStateMachine, LatestStateMachineHeight, Nonce, Responded,
    ResponseReceipt, StateCommitments, StateMachineUpdateTime, UnbondingPeriod,
};
use alloc::{format, string::ToString};
use core::time::Duration;
use frame_support::traits::{Get, UnixTime};
use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
        StateMachineId,
    },
    error::Error,
    host::{IsmpHost, StateMachine},
    router::{IsmpRouter, PostResponse, Request, Response},
    util::{hash_post_response, hash_request, hash_response},
};
use sp_core::H256;
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

/// An implementation for the IsmpHost
#[derive(Clone)]
pub struct Host<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> Default for Host<T> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<T: Config> IsmpHost for Host<T> {
    fn host_state_machine(&self) -> StateMachine {
        T::HostStateMachine::get()
    }

    fn latest_commitment_height(&self, id: StateMachineId) -> Result<u64, Error> {
        Ok(LatestStateMachineHeight::<T>::get(id))
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

    fn state_machine_update_time(
        &self,
        state_machine_height: StateMachineHeight,
    ) -> Result<Duration, Error> {
        StateMachineUpdateTime::<T>::get(state_machine_height)
            .map(|timestamp| Duration::from_secs(timestamp))
            .ok_or_else(|| {
                Error::ImplementationSpecific(format!(
                    "Update time not found for {:?}",
                    state_machine_height
                ))
            })
    }

    fn consensus_client_id(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Option<ConsensusClientId> {
        ConsensusStateClient::<T>::get(&consensus_state_id)
    }

    fn consensus_state(&self, id: ConsensusClientId) -> Result<Vec<u8>, Error> {
        ConsensusStates::<T>::get(id)
            .ok_or_else(|| Error::ConsensusStateNotFound { consensus_state_id: id })
    }

    fn timestamp(&self) -> Duration {
        <T::TimeProvider as UnixTime>::now()
    }

    fn is_state_machine_frozen(&self, machine: StateMachineId) -> Result<(), Error> {
        if let Some(frozen) = FrozenStateMachine::<T>::get(machine) {
            if frozen {
                Err(Error::FrozenStateMachine { id: machine })?
            }
        }
        Ok(())
    }

    fn is_consensus_client_frozen(&self, client: ConsensusStateId) -> Result<(), Error> {
        if FrozenConsensusClients::<T>::get(client) {
            Err(Error::FrozenConsensusClient { consensus_state_id: client })?
        }
        Ok(())
    }

    fn request_commitment(&self, commitment: H256) -> Result<(), Error> {
        let _ = RequestCommitments::<T>::get(commitment).ok_or_else(|| {
            Error::ImplementationSpecific("Request commitment not found".to_string())
        })?;

        Ok(())
    }

    fn response_commitment(&self, commitment: H256) -> Result<(), Error> {
        let _ = ResponseCommitments::<T>::get(commitment).ok_or_else(|| {
            Error::ImplementationSpecific("Response commitment not found".to_string())
        })?;

        Ok(())
    }

    fn next_nonce(&self) -> u64 {
        let nonce = Nonce::<T>::get();
        Nonce::<T>::put(nonce + 1);
        nonce
    }

    fn request_receipt(&self, req: &Request) -> Option<()> {
        let commitment = hash_request::<Self>(req);

        let _ = RequestReceipts::<T>::get(commitment)
            .ok_or_else(|| Error::RequestCommitmentNotFound {
                nonce: req.nonce(),
                source: req.source_chain(),
                dest: req.dest_chain(),
            })
            .ok()?;

        Some(())
    }

    fn response_receipt(&self, res: &Response) -> Option<()> {
        let commitment = hash_request::<Self>(&res.request());

        let _ = ResponseReceipts::<T>::get(commitment)
            .ok_or_else(|| Error::ImplementationSpecific("Response receipt not found".to_string()))
            .ok()?;

        Some(())
    }

    fn store_consensus_state_id(
        &self,
        consensus_state_id: ConsensusStateId,
        client_id: ConsensusClientId,
    ) -> Result<(), Error> {
        ConsensusStateClient::<T>::insert(consensus_state_id, client_id);
        Ok(())
    }

    fn store_consensus_state(&self, id: ConsensusClientId, state: Vec<u8>) -> Result<(), Error> {
        ConsensusStates::<T>::insert(id, state);
        Ok(())
    }

    fn store_unbonding_period(
        &self,
        consensus_state_id: ConsensusStateId,
        period: u64,
    ) -> Result<(), Error> {
        UnbondingPeriod::<T>::insert(consensus_state_id, period);
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

    fn store_state_machine_update_time(
        &self,
        state_machine_height: StateMachineHeight,
        timestamp: Duration,
    ) -> Result<(), Error> {
        StateMachineUpdateTime::<T>::insert(
            state_machine_height,
            timestamp.as_secs().saturated_into::<u64>(),
        );
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

    fn delete_state_commitment(&self, height: StateMachineHeight) -> Result<(), Error> {
        StateCommitments::<T>::remove(height);
        Ok(())
    }

    fn freeze_consensus_client(&self, client: ConsensusStateId) -> Result<(), Error> {
        FrozenConsensusClients::<T>::insert(client, true);
        Ok(())
    }

    fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error> {
        LatestStateMachineHeight::<T>::insert(height.id, height.height);
        Ok(())
    }

    fn delete_request_commitment(&self, req: &Request) -> Result<(), Error> {
        let hash = hash_request::<Self>(req);
        // We can't delete actual leaves in the mmr so this serves as a replacement for that
        RequestCommitments::<T>::remove(hash);
        Ok(())
    }

    fn delete_response_commitment(&self, res: &PostResponse) -> Result<(), Error> {
        let req_commitment = hash_request::<Self>(&res.request());
        let hash = hash_post_response::<Self>(res);

        // We can't delete actual leaves in the mmr so this serves as a replacement for that
        ResponseCommitments::<T>::remove(hash);
        Responded::<T>::remove(req_commitment);
        Ok(())
    }

    fn delete_request_receipt(&self, req: &Request) -> Result<(), Error> {
        let req_commitment = hash_request::<Self>(req);
        RequestReceipts::<T>::remove(req_commitment);
        Ok(())
    }

    fn delete_response_receipt(&self, res: &PostResponse) -> Result<(), Error> {
        let hash = hash_request::<Self>(&res.request());
        ResponseReceipts::<T>::remove(hash);
        Ok(())
    }

    fn store_request_receipt(&self, req: &Request, signer: &Vec<u8>) -> Result<(), Error> {
        let hash = hash_request::<Self>(req);
        RequestReceipts::<T>::insert(hash, signer);
        Ok(())
    }

    fn store_response_receipt(&self, res: &Response, signer: &Vec<u8>) -> Result<(), Error> {
        let hash = hash_request::<Self>(&res.request());
        let response = hash_response::<Self>(&res);
        ResponseReceipts::<T>::insert(hash, ResponseReceipt { response, relayer: signer.clone() });
        Ok(())
    }

    fn consensus_clients(&self) -> Vec<Box<dyn ConsensusClient>> {
        <T as Config>::ConsensusClients::consensus_clients()
    }

    fn challenge_period(&self, id: ConsensusStateId) -> Option<Duration> {
        ChallengePeriod::<T>::get(&id).map(Duration::from_secs)
    }

    fn store_challenge_period(
        &self,
        consensus_state_id: ConsensusStateId,
        period: u64,
    ) -> Result<(), Error> {
        ChallengePeriod::<T>::insert(consensus_state_id, period);
        Ok(())
    }

    fn allowed_proxy(&self) -> Option<StateMachine> {
        T::Coprocessor::get()
    }

    fn unbonding_period(&self, consensus_state_id: ConsensusStateId) -> Option<Duration> {
        UnbondingPeriod::<T>::get(&consensus_state_id).map(Duration::from_secs)
    }

    fn ismp_router(&self) -> Box<dyn IsmpRouter> {
        Box::new(T::Router::default())
    }

    fn freeze_state_machine_client(&self, state_machine: StateMachineId) -> Result<(), Error> {
        FrozenStateMachine::<T>::insert(state_machine, true);
        Ok(())
    }
}

impl<T: Config> ismp::util::Keccak256 for Host<T> {
    fn keccak256(bytes: &[u8]) -> H256
    where
        Self: Sized,
    {
        sp_io::hashing::keccak_256(bytes).into()
    }
}
