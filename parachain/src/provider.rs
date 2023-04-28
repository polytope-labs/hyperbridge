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

use crate::ParachainClient;
use codec::{Decode, Encode};
use ismp::{
    consensus::{ConsensusClientId, StateMachineId},
    router::{Request, Response},
};
use ismp_parachain::consensus::{HashAlgorithm, MembershipProof, ParachainStateProof};
use ismp_primitives::LeafIndexQuery;
use ismp_rpc::BlockNumberOrHash;
use pallet_ismp::{primitives::Proof as MmrProof, NodesUtils};
use sp_core::{sp_std::sync::Arc, H256};
use std::{collections::HashMap, time::Duration};
use subxt::rpc_params;
use tesseract_primitives::{IsmpProvider, Query, StateMachineUpdated};

#[async_trait::async_trait]
impl<T> IsmpProvider for ParachainClient<T>
where
    T: subxt::Config,
{
    async fn query_consensus_state(
        &self,
        at: Option<u64>,
        id: ConsensusClientId,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let params = rpc_params![at, id];
        let response = self.parachain.rpc().request("ismp_queryConsensusState", params).await?;

        Ok(response)
    }

    async fn query_latest_state_machine_height(
        &self,
        id: StateMachineId,
    ) -> Result<u32, anyhow::Error> {
        let params = rpc_params![id];
        let response =
            self.parachain.rpc().request("ismp_queryStateMachineLatestHeight", params).await?;

        Ok(response)
    }

    async fn query_consensus_update_time(
        &self,
        id: ConsensusClientId,
    ) -> Result<Duration, anyhow::Error> {
        let params = rpc_params![id];
        let response: u64 =
            self.parachain.rpc().request("ismp_queryConsensusUpdateTime", params).await?;

        Ok(Duration::from_secs(response))
    }

    async fn query_requests_proof(
        &self,
        at: u64,
        keys: Vec<Query>,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let params = rpc_params![at, convert_queries(keys)];
        let response: ismp_rpc::Proof =
            self.parachain.rpc().request("ismp_queryRequestsMmrProof", params).await?;
        let mmr_proof: MmrProof<H256> = Decode::decode(&mut &*response.proof)?;
        let proof = MembershipProof {
            mmr_size: NodesUtils::new(mmr_proof.leaf_count).size(),
            leaf_indices: mmr_proof.leaf_indices,
            proof: mmr_proof.items,
        };
        Ok(proof.encode())
    }

    async fn query_responses_proof(
        &self,
        at: u64,
        keys: Vec<Query>,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let params = rpc_params![at, convert_queries(keys)];
        let response: ismp_rpc::Proof =
            self.parachain.rpc().request("ismp_queryResponsesMmrProof", params).await?;
        let mmr_proof: MmrProof<H256> = Decode::decode(&mut &*response.proof)?;
        let proof = MembershipProof {
            mmr_size: NodesUtils::new(mmr_proof.leaf_count).size(),
            leaf_indices: mmr_proof.leaf_indices,
            proof: mmr_proof.items,
        };
        Ok(proof.encode())
    }

    async fn query_state_proof(
        &self,
        at: u64,
        keys: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let params = rpc_params![at, keys];
        let response: ismp_rpc::Proof =
            self.parachain.rpc().request("ismp_queryStateProof", params).await?;

        let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*response.proof)?;
        let proof = ParachainStateProof { hasher: HashAlgorithm::Keccak, storage_proof };

        Ok(proof.encode())
    }

    async fn query_ismp_events(
        &self,
        event: StateMachineUpdated,
    ) -> Result<Vec<pallet_ismp::events::Event>, anyhow::Error> {
        let latest_state_machine_height = Arc::clone(&self.latest_state_machine_height);

        let block_numbers: Vec<BlockNumberOrHash<sp_core::H256>> =
            ((*latest_state_machine_height.lock() + 1)..=event.latest_height)
                .into_iter()
                .map(|block_height| BlockNumberOrHash::Number(block_height as u32))
                .collect();
        *latest_state_machine_height.lock() = event.latest_height;

        let params = rpc_params![block_numbers];
        let response: HashMap<String, Vec<pallet_ismp::events::Event>> =
            self.parachain.rpc().request("ismp_queryEvents", params).await?;

        Ok(response.values().into_iter().cloned().flatten().collect())
    }

    async fn query_requests(&self, keys: Vec<Query>) -> Result<Vec<Request>, anyhow::Error> {
        let queries = convert_queries(keys);
        let params = rpc_params![queries];
        let response = self.parachain.rpc().request("ismp_queryRequests", params).await?;

        Ok(response)
    }

    async fn query_responses(&self, keys: Vec<Query>) -> Result<Vec<Response>, anyhow::Error> {
        let queries = convert_queries(keys);
        let params = rpc_params![queries];
        let response = self.parachain.rpc().request("ismp_queryResponses", params).await?;

        Ok(response)
    }
}

fn convert_queries(queries: Vec<Query>) -> Vec<LeafIndexQuery> {
    queries
        .into_iter()
        .map(|query| LeafIndexQuery {
            source_chain: query.source_chain,
            dest_chain: query.dest_chain,
            nonce: query.nonce,
        })
        .collect()
}
