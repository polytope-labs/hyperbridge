use crate::types::{
    filter_map_system_events, system_events_key, BoxStream, Extrinsic, HyperBridgeConfig,
    LeafIndexQuery,
};
use codec::{Decode, Encode};
use ethers::prelude::H256;
use hex_literal::hex;
use ismp::{
    consensus::{StateCommitment, StateMachineHeight, StateMachineId},
    events::StateMachineUpdated,
    host::StateMachine,
    messaging::{Message, Proof},
    router::Request,
};
use sp_core::{blake2_128, storage::StorageChangeSet};
use subxt::{rpc_params, OnlineClient};

#[derive(Debug, Clone)]
pub struct HyperBridgeClient {
    // WS RPC url of a hyperbridge node
    pub rpc_url: String,
    // An instance of Hyper bridge client using the default config
    pub client: OnlineClient<HyperBridgeConfig>,
}

impl HyperBridgeClient {
    pub async fn new(rpc_url: String) -> Result<Self, anyhow::Error> {
        let client = OnlineClient::<HyperBridgeConfig>::from_url(rpc_url.clone()).await?;

        Ok(Self { rpc_url, client })
    }

    pub async fn get_current_timestamp(&self) -> Result<u64, anyhow::Error> {
        let timestamp_addr: [u8; 32] =
            hex!("f0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb");
        let timestamp_op = self.client.rpc().storage(&timestamp_addr, None).await?;

        return if let Some(timestamp) = timestamp_op {
            let hyper_bridge_timestamp: u64 = codec::Decode::decode(&mut &*timestamp.0).unwrap();
            Ok(hyper_bridge_timestamp)
        } else {
            Ok(0u64)
        };
    }

    pub async fn query_request(
        &self,
        source_chain: &StateMachine,
        dest_chain: &StateMachine,
        nonce: u64,
    ) -> Result<Vec<Request>, anyhow::Error> {
        let build_leaf_index_query =
            LeafIndexQuery { source_chain: *source_chain, dest_chain: *dest_chain, nonce };

        let leaf_index_query = rpc_params![build_leaf_index_query];
        let hyper_bridge_response: Vec<Request> =
            self.client.rpc().request("ismp_queryRequests", leaf_index_query).await?;

        Ok(hyper_bridge_response)
    }

    pub async fn query_response(
        &self,
        source_chain: &StateMachine,
        dest_chain: &StateMachine,
        nonce: u64,
    ) -> Result<Vec<Request>, anyhow::Error> {
        let build_leaf_index_query =
            LeafIndexQuery { source_chain: *source_chain, dest_chain: *dest_chain, nonce };

        let leaf_index_query = rpc_params![build_leaf_index_query];
        let hyper_bridge_response: Vec<Request> =
            self.client.rpc().request("ismp_queryResponses", leaf_index_query).await?;

        Ok(hyper_bridge_response)
    }

    pub async fn query_state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<StateCommitment, anyhow::Error> {
        let mut partial_key =
            hex!("103895530afb23bb607661426d55eb8bf0f16a60fa21b8baaa82ee16ed43643d").to_vec();
        let encoded_height = blake2_128(&height.encode());
        partial_key.extend_from_slice(&encoded_height);

        let response = self.client.rpc().storage(&partial_key, None).await.unwrap().unwrap();
        let commitment: StateCommitment = codec::Decode::decode(&mut response.0.as_slice())?;
        Ok(commitment)
    }

    pub async fn state_machine_update_notification(
        &self,
        counterparty_state_id: StateMachineId,
    ) -> Result<BoxStream<StateMachineUpdated>, anyhow::Error> {
        let keys = vec![system_events_key()];
        let subscription = self
            .client
            .rpc()
            .subscribe::<StorageChangeSet<H256>>(
                "state_subscribeStorage",
                rpc_params![keys],
                "state_unsubscribeStorage",
            )
            .await
            .expect("Storage subscription failed");

        Ok(filter_map_system_events(subscription, counterparty_state_id))
    }


    pub async fn send_message(
        &self,
        proof: Proof,
        message: Message,
    ) -> Result<H256, anyhow::Error> {
        let call = vec![message].encode();
        let hyper_bridge_timeout_extrinsic = Extrinsic::new("Ismp", "handle", call);
        let ext = self.client.tx().create_unsigned(&hyper_bridge_timeout_extrinsic).unwrap();
        let timeout_progress = ext.submit_and_watch().await.unwrap();
        let timeout_outcome = timeout_progress.wait_for_in_block().await.unwrap();
        let timeout_hash = timeout_outcome.wait_for_success().await.unwrap().block_hash();

        Ok(timeout_hash)
    }

    pub async fn get_state_proof(
        &self,
        at: u64,
        keys: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let params = rpc_params![at, keys];
        let res: Proof = self.client.rpc().request("ismp_queryStateProof", params).await?;
        let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*res.proof)?;
        Ok(storage_proof.encode())
    }
}

const REQUEST_PARTIAL_KEY: [u8; 32] =
    hex!("103895530afb23bb607661426d55eb8bbd3caa596ab5c98b359f0ffc7d17e376");
const RESPONSE_PARTIAL_KEY: [u8; 32] =
    hex!("103895530afb23bb607661426d55eb8b8fdfbc1b10c58ed36779810ffdba8e79");

pub fn get_request_storage_key(request_commitment: Vec<u8>) -> Vec<u8> {
    let mut key = REQUEST_PARTIAL_KEY.to_vec();
    key.extend_from_slice(&*request_commitment);

    key
}

pub fn get_response_storage_key(response_commitment: Vec<u8>) -> Vec<u8> {
    let mut key = RESPONSE_PARTIAL_KEY.to_vec();
    key.extend_from_slice(&*response_commitment);

    key
}
