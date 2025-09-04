// Copyright (c) 2025 Polytope Labs.
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

//! [`IsmpHost`] implementation for pallet-ismp
use polkadot_sdk::*;

use crate::{
	child_trie,
	dispatcher::{RefundingRouter, RequestMetadata},
	utils::{ConsensusClientProvider, ResponseReceipt},
	ChallengePeriod, Config, ConsensusClientUpdateTime, ConsensusStateClient, ConsensusStates,
	FrozenConsensusClients, LatestStateMachineHeight, Nonce, Pallet, PreviousStateMachineHeight,
	Responded, StateMachineUpdateTime, UnbondingPeriod,
};
use alloc::{format, string::ToString};
use codec::{Decode, Encode};
use core::time::Duration;
use crypto_utils::verification::Signature;
use frame_support::traits::{Get, UnixTime};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
		StateMachineId,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::{hash_post_response, hash_request, hash_response},
	router::{IsmpRouter, PostResponse, Request, Response},
};
use sp_core::H256;
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

impl<T: Config> IsmpHost for Pallet<T> {
	fn host_state_machine(&self) -> StateMachine {
		<T as Config>::HostStateMachine::get()
	}

	fn latest_commitment_height(&self, id: StateMachineId) -> Result<u64, Error> {
		Ok(LatestStateMachineHeight::<T>::get(id).unwrap_or_default())
	}

	fn state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		child_trie::StateCommitments::<T>::get(height)
			.ok_or_else(|| Error::StateCommitmentNotFound { height })
	}

	fn consensus_update_time(&self, id: ConsensusClientId) -> Result<Duration, Error> {
		ConsensusClientUpdateTime::<T>::get(id)
			.map(|timestamp| Duration::from_secs(timestamp))
			.ok_or_else(|| Error::Custom(format!("Update time not found for {:?}", id)))
	}

	fn state_machine_update_time(
		&self,
		state_machine_height: StateMachineHeight,
	) -> Result<Duration, Error> {
		StateMachineUpdateTime::<T>::get(state_machine_height)
			.map(|timestamp| Duration::from_secs(timestamp))
			.ok_or_else(|| {
				Error::Custom(format!("Update time not found for {:?}", state_machine_height))
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
		<T::TimestampProvider as UnixTime>::now()
	}

	fn is_consensus_client_frozen(&self, client: ConsensusStateId) -> Result<(), Error> {
		if FrozenConsensusClients::<T>::get(client) {
			Err(Error::FrozenConsensusClient { consensus_state_id: client })?
		}
		Ok(())
	}

	fn request_commitment(&self, commitment: H256) -> Result<(), Error> {
		let _ = child_trie::RequestCommitments::<T>::get(commitment)
			.ok_or_else(|| Error::Custom("Request commitment not found".to_string()))?;

		Ok(())
	}

	fn response_commitment(&self, commitment: H256) -> Result<(), Error> {
		let _ = child_trie::ResponseCommitments::<T>::get(commitment)
			.ok_or_else(|| Error::Custom("Response commitment not found".to_string()))?;

		Ok(())
	}

	fn next_nonce(&self) -> u64 {
		let nonce = Nonce::<T>::get();
		Nonce::<T>::put(nonce + 1);
		nonce
	}

	fn request_receipt(&self, req: &Request) -> Option<()> {
		let commitment = hash_request::<Self>(req);

		let _ = child_trie::RequestReceipts::<T>::get(commitment)
			.ok_or_else(|| Error::RequestCommitmentNotFound { meta: req.into() })
			.ok()?;

		Some(())
	}

	fn response_receipt(&self, res: &Response) -> Option<()> {
		let commitment = hash_request::<Self>(&res.request());

		let _ = child_trie::ResponseReceipts::<T>::get(commitment)
			.ok_or_else(|| Error::Custom("Response receipt not found".to_string()))
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
		child_trie::StateCommitments::<T>::insert(height, state);
		Ok(())
	}

	fn delete_state_commitment(&self, height: StateMachineHeight) -> Result<(), Error> {
		child_trie::StateCommitments::<T>::remove(height);

		// technically any state commitment can be vetoed,
		// safety check that it's the latest before resetting it.
		if let Some(latest) = LatestStateMachineHeight::<T>::get(height.id) {
			if latest == height.height {
				// Reset back to the initial height to allow for honest updates
				LatestStateMachineHeight::<T>::insert(height.id, 1);
			}
		}
		Ok(())
	}

	fn freeze_consensus_client(&self, client: ConsensusStateId) -> Result<(), Error> {
		FrozenConsensusClients::<T>::insert(client, true);
		Ok(())
	}

	fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error> {
		let previous_height = LatestStateMachineHeight::<T>::get(height.id).unwrap_or_default();
		PreviousStateMachineHeight::<T>::insert(height.id, previous_height);
		LatestStateMachineHeight::<T>::insert(height.id, height.height);
		Ok(())
	}

	fn delete_request_commitment(&self, req: &Request) -> Result<Vec<u8>, Error> {
		let hash = hash_request::<Self>(req);
		// We can't delete actual leaves in the mmr so this serves as a replacement for that
		let meta = child_trie::RequestCommitments::<T>::get(hash)
			.ok_or_else(|| Error::Custom("Request Commitment not found".to_string()))?;
		child_trie::RequestCommitments::<T>::remove(hash);
		Ok(meta.encode())
	}

	fn delete_response_commitment(&self, res: &PostResponse) -> Result<Vec<u8>, Error> {
		let req_commitment = hash_request::<Self>(&res.request());
		let hash = hash_post_response::<Self>(res);
		let meta = child_trie::ResponseCommitments::<T>::get(hash)
			.ok_or_else(|| Error::Custom("Response Commitment not found".to_string()))?;
		// We can't delete actual leaves in the mmr so this serves as a replacement for that
		child_trie::ResponseCommitments::<T>::remove(hash);
		Responded::<T>::remove(req_commitment);
		Ok(meta.encode())
	}

	fn delete_request_receipt(&self, req: &Request) -> Result<Vec<u8>, Error> {
		let req_commitment = hash_request::<Self>(req);
		let relayer = child_trie::RequestReceipts::<T>::get(req_commitment)
			.ok_or_else(|| Error::Custom("Request receipt not found".to_string()))?;
		child_trie::RequestReceipts::<T>::remove(req_commitment);
		Ok(relayer)
	}

	fn delete_response_receipt(&self, res: &Response) -> Result<Vec<u8>, Error> {
		let hash = hash_request::<Self>(&res.request());
		let meta = child_trie::ResponseReceipts::<T>::get(hash)
			.ok_or_else(|| Error::Custom("Response receipt not found".to_string()))?;
		child_trie::ResponseReceipts::<T>::remove(hash);
		Ok(meta.relayer)
	}

	fn store_request_receipt(&self, req: &Request, signer: &Vec<u8>) -> Result<Vec<u8>, Error> {
		let signer = extract_signer(signer)?;

		let hash = hash_request::<Self>(req);
		child_trie::RequestReceipts::<T>::insert(hash, &signer);
		Ok(signer)
	}

	fn store_response_receipt(&self, res: &Response, signer: &Vec<u8>) -> Result<Vec<u8>, Error> {
		let signer = extract_signer(signer)?;

		let hash = hash_request::<Self>(&res.request());
		let response = hash_response::<Self>(&res);
		child_trie::ResponseReceipts::<T>::insert(
			hash,
			ResponseReceipt { response, relayer: signer.clone() },
		);
		Ok(signer)
	}

	fn consensus_clients(&self) -> Vec<Box<dyn ConsensusClient>> {
		<T as Config>::ConsensusClients::consensus_clients()
	}

	fn challenge_period(&self, state_machine: StateMachineId) -> Option<Duration> {
		ChallengePeriod::<T>::get(&state_machine).map(Duration::from_secs)
	}

	fn store_challenge_period(
		&self,
		state_machine: StateMachineId,
		period: u64,
	) -> Result<(), Error> {
		ChallengePeriod::<T>::insert(state_machine, period);
		Ok(())
	}

	fn allowed_proxy(&self) -> Option<StateMachine> {
		T::Coprocessor::get()
	}

	fn unbonding_period(&self, consensus_state_id: ConsensusStateId) -> Option<Duration> {
		UnbondingPeriod::<T>::get(&consensus_state_id).map(Duration::from_secs)
	}

	fn ismp_router(&self) -> Box<dyn IsmpRouter> {
		Box::new(RefundingRouter::<T>::new(Box::new(T::Router::default())))
	}

	fn store_request_commitment(&self, req: &Request, meta: Vec<u8>) -> Result<(), Error> {
		let hash = hash_request::<Self>(req);
		let leaf_meta = RequestMetadata::<T>::decode(&mut &*meta)
			.map_err(|_| Error::Custom("Failed to decode leaf metadata".to_string()))?;
		child_trie::RequestCommitments::<T>::insert(hash, leaf_meta);
		Ok(())
	}

	fn store_response_commitment(&self, res: &PostResponse, meta: Vec<u8>) -> Result<(), Error> {
		let hash = hash_post_response::<Self>(res);
		let req_commitment = hash_request::<Self>(&res.request());
		let leaf_meta = RequestMetadata::<T>::decode(&mut &*meta)
			.map_err(|_| Error::Custom("Failed to decode leaf metadata".to_string()))?;
		child_trie::ResponseCommitments::<T>::insert(hash, leaf_meta);
		Responded::<T>::insert(req_commitment, true);
		Ok(())
	}

	fn previous_commitment_height(&self, id: StateMachineId) -> Option<u64> {
		PreviousStateMachineHeight::<T>::get(id)
	}
}

impl<T: Config> ismp::messaging::Keccak256 for Pallet<T> {
	fn keccak256(bytes: &[u8]) -> H256
	where
		Self: Sized,
	{
		sp_io::hashing::keccak_256(bytes).into()
	}
}

fn extract_signer(signer: &[u8]) -> Result<Vec<u8>, Error> {
	if signer.len() > 32 {
		Signature::decode(&mut signer.as_ref())
			.map(|sig| sig.signer())
			.map_err(|_| Error::SignatureDecodingFailed)
	} else {
		Ok(signer.to_vec())
	}
}
