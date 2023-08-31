use crate::arbitrum::{derive_key, NODES_SLOT};
use ethers::prelude::*;
use hex_literal::hex;
use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, StateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error,
    host::{ISMPHost, StateMachine},
    router::{ISMPRouter, Request},
};
use sp_core::{H160, H256};
use std::time::Duration;

pub struct Host;

impl ISMPHost for Host {
    fn host_state_machine(&self) -> StateMachine {
        todo!()
    }

    fn latest_commitment_height(&self, _id: StateMachineId) -> Result<StateMachineHeight, Error> {
        todo!()
    }

    fn state_machine_commitment(
        &self,
        _height: StateMachineHeight,
    ) -> Result<StateCommitment, Error> {
        todo!()
    }

    fn consensus_update_time(&self, _id: ConsensusClientId) -> Result<Duration, Error> {
        todo!()
    }

    fn consensus_state(&self, _id: ConsensusClientId) -> Result<Vec<u8>, Error> {
        todo!()
    }

    fn timestamp(&self) -> Duration {
        todo!()
    }

    fn is_frozen(&self, _height: StateMachineHeight) -> Result<bool, Error> {
        todo!()
    }

    fn request_commitment(&self, _req: &Request) -> Result<TxHash, Error> {
        todo!()
    }

    fn store_consensus_state(&self, _id: ConsensusClientId, _state: Vec<u8>) -> Result<(), Error> {
        todo!()
    }

    fn store_consensus_update_time(
        &self,
        _id: ConsensusClientId,
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

    fn store_latest_commitment_height(&self, _height: StateMachineHeight) -> Result<(), Error> {
        todo!()
    }

    fn consensus_client(&self, _id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
        todo!()
    }

    fn keccak256(bytes: &[u8]) -> H256
    where
        Self: Sized,
    {
        sp_core::keccak_256(bytes).into()
    }

    fn challenge_period(&self, _id: ConsensusClientId) -> Duration {
        todo!()
    }

    fn ismp_router(&self) -> Box<dyn ISMPRouter> {
        todo!()
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
    let position = H256::from_slice(derive_key::<Host>(latest_node_created, NODES_SLOT).as_slice());
    let proof = provider.get_proof(rollup, vec![position], None).await.unwrap();

    let mut buf = vec![0u8; 32];
    proof.storage_proof[0].clone().value.to_big_endian(&mut buf);
    let state_hash = hex::encode(buf);
    println!("State Hash {}", state_hash);
}
