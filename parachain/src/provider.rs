use crate::ParachainClient;
use anyhow::Error;
use codec::{Decode, Encode};
use ismp::{
    consensus::{ConsensusClientId, StateMachineId},
    router::{Request, Response},
};
use ismp_parachain::consensus::{HashAlgorithm, MembershipProof, ParachainStateProof};
use ismp_primitives::LeafIndexQuery;
use ismp_rpc::BlockNumberOrHash;
use pallet_ismp::primitives::Proof as MmrProof;
use sp_core::H256;
use std::time::Duration;
use subxt::rpc_params;
use tesseract_primitives::{IsmpProvider, Query, StateMachineUpdated};

#[async_trait::async_trait]
impl<T> IsmpProvider for ParachainClient<T>
where
    T: subxt::Config,
{
    type TransactionId = ();

    async fn query_consensus_state(
        &self,
        at: u64,
        id: ConsensusClientId,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let params = rpc_params![Some(at), id];
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
            mmr_size: mmr_proof.leaf_count,
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
            mmr_size: mmr_proof.leaf_count,
            leaf_indices: mmr_proof.leaf_indices,
            proof: mmr_proof.items,
        };
        Ok(proof.encode())
    }

    async fn query_state_proof(&self, at: u64, keys: Vec<Vec<u8>>) -> Result<Vec<u8>, Error> {
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
        unimplemented!()
        // let block_numbers: Vec<BlockNumberOrHash<sp_core::H256>> = ((event.previous_height +
        // 1)..=     event.latest_height)
        //     .into_iter()
        //     .map(|block_height| BlockNumberOrHash::Number(block_height as u32))
        //     .collect();
        //
        // let params = rpc_params![block_numbers];
        // let response = self.parachain.rpc().request("ismp_queryEvents", params).await?;
        //
        // Ok(response)
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
