// Copyright (C) Polytope Labs Ltd.
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

use crate::{presets::NODES_SLOT, utils::derive_map_key};
use ethers::prelude::*;
use hex_literal::hex;
use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
        StateMachineId,
    },
    error::Error,
    host::{IsmpHost, StateMachine},
    router::{IsmpRouter, PostResponse, Request, Response},
};
use sp_core::{H160, H256};
use std::time::Duration;

pub struct Host;

impl IsmpHost for Host {
    fn host_state_machine(&self) -> StateMachine {
        todo!()
    }

    fn latest_commitment_height(&self, id: StateMachineId) -> Result<u64, Error> {
        todo!()
    }

    fn state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<StateCommitment, Error> {
        todo!()
    }

    fn consensus_update_time(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Result<Duration, Error> {
        todo!()
    }

    fn state_machine_update_time(
        &self,
        state_machine_height: StateMachineHeight,
    ) -> Result<Duration, Error> {
        todo!()
    }

    fn consensus_client_id(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Option<ConsensusClientId> {
        todo!()
    }

    fn consensus_state(&self, consensus_state_id: ConsensusStateId) -> Result<Vec<u8>, Error> {
        todo!()
    }

    fn timestamp(&self) -> Duration {
        todo!()
    }

    fn is_state_machine_frozen(&self, machine: StateMachineId) -> Result<(), Error> {
        todo!()
    }

    fn is_consensus_client_frozen(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Result<(), Error> {
        todo!()
    }

    fn request_commitment(&self, req: TxHash) -> Result<(), Error> {
        todo!()
    }

    fn response_commitment(&self, req: TxHash) -> Result<(), Error> {
        todo!()
    }

    fn next_nonce(&self) -> u64 {
        todo!()
    }

    fn request_receipt(&self, req: &Request) -> Option<()> {
        todo!()
    }

    fn response_receipt(&self, res: &Response) -> Option<()> {
        todo!()
    }

    fn store_consensus_state_id(
        &self,
        consensus_state_id: ConsensusStateId,
        client_id: ConsensusClientId,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_consensus_state(
        &self,
        consensus_state_id: ConsensusStateId,
        consensus_state: Vec<u8>,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_unbonding_period(
        &self,
        consensus_state_id: ConsensusStateId,
        period: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_consensus_update_time(
        &self,
        consensus_state_id: ConsensusStateId,
        timestamp: Duration,
    ) -> Result<(), Error> {
        todo!()
    }

    fn store_state_machine_update_time(
        &self,
        state_machine_height: StateMachineHeight,
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

    fn freeze_state_machine(&self, state_machine: StateMachineId) -> Result<(), Error> {
        todo!()
    }

    fn unfreeze_state_machine(&self, state_machine: StateMachineId) -> Result<(), Error> {
        todo!()
    }

    fn freeze_consensus_client(&self, consensus_state_id: ConsensusStateId) -> Result<(), Error> {
        todo!()
    }

    fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error> {
        todo!()
    }

    fn delete_request_commitment(&self, req: &Request) -> Result<(), Error> {
        todo!()
    }

    fn delete_response_commitment(&self, res: &PostResponse) -> Result<(), Error> {
        todo!()
    }

    fn store_request_receipt(&self, req: &Request, _signer: &Vec<u8>) -> Result<(), Error> {
        todo!()
    }

    fn store_response_receipt(&self, req: &Response, _signer: &Vec<u8>) -> Result<(), Error> {
        todo!()
    }

    fn consensus_client(&self, id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
        todo!()
    }

    fn challenge_period(&self, consensus_state_id: ConsensusStateId) -> Option<Duration> {
        todo!()
    }

    fn store_challenge_period(
        &self,
        consensus_state_id: ConsensusStateId,
        period: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    fn allowed_proxy(&self) -> Option<StateMachine> {
        todo!()
    }

    fn unbonding_period(&self, consensus_state_id: ConsensusStateId) -> Option<Duration> {
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

#[tokio::test]
#[ignore]
/// This test ensures that the key derivation works correctly.
async fn fetch_arbitrum_node_state_hash() {
    // Initialize a new Http provider
    let rpc_url = "https://rpc.ankr.com/eth";
    let provider = Provider::try_from(rpc_url).unwrap();
    let rollup = H160::from_slice(hex!("5eF0D09d1E6204141B4d37530808eD19f60FBa35").as_slice());
    let mut latest_node_created_bytes = [0u8; 32];
    // Latest node created is at slot 117
    U256::from(117).to_big_endian(&mut latest_node_created_bytes);
    let latest_node_created_position = H256::from_slice(&latest_node_created_bytes[..]);

    let latest_node_created_proof = provider
        .get_proof(rollup, vec![latest_node_created_position], None)
        .await
        .unwrap();
    // the latest node created is the second item in this slot
    let latest_node_created = latest_node_created_proof.storage_proof[0].value.0[1];
    dbg!(latest_node_created);
    let mut key = [0u8; 32];
    U256::from(latest_node_created).to_big_endian(&mut key);
    let position = derive_map_key::<Host>(key.to_vec(), NODES_SLOT);
    let proof = provider.get_proof(rollup, vec![position], None).await.unwrap();

    let mut buf = vec![0u8; 32];
    proof.storage_proof[0].clone().value.to_big_endian(&mut buf);
    let state_hash = hex::encode(buf);
    println!("State Hash {}", state_hash);
}
